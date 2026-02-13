use pyo3::basic::CompareOp;
use pyo3::exceptions::{PyKeyError, PyRuntimeError, PyTypeError};
use pyo3::prelude::*;
use pyo3::types::{PyDict, PyFloat, PyInt, PyList, PyModule, PyString, PyTuple};
use regex::Regex;
use std::sync::LazyLock;

#[derive(Clone, Debug)]
enum TokenKind {
    Root,
    Wildcard,
    DeepWildcard,
    Map(String),
    Get(String),
    Index {
        key: String,
        index: isize,
    },
    Slice {
        key: String,
        start: Option<isize>,
        end: Option<isize>,
    },
    Filter {
        list_key: String,
        field: String,
        operator: String,
        value: String,
    },
}

#[derive(Clone, Debug)]
struct ParsedToken {
    raw: String,
    kind: TokenKind,
}

static INDEX_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^(.+)\[(-?\d+)\]$").expect("valid regex"));
static SLICE_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^(.+)\[(-?\d*):(-?\d*)\]$").expect("valid regex"));
static FILTER_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^(.+)\[\?(.+?)(==|!=|>=|<=|>|<)(.+?)\]$").expect("valid regex"));
static PATH_FILTER_SEGMENT_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^\$([a-zA-Z_]\w*)(?:\((.*)\))?(\[\])?$").expect("valid regex"));

enum BuiltinFilter {
    Inc,
    Dec,
    Double,
    Square,
    String,
    Int,
    Float,
    Decimal,
    Quote,
    Even,
    Odd,
    Gt(PyObject),
    Lt(PyObject),
    Gte(PyObject),
    Lte(PyObject),
    Add(PyObject),
    Sub(PyObject),
    Mul(PyObject),
    Div(PyObject),
    Mod(PyObject),
    Neg,
    Pow(PyObject),
    RPow(PyObject),
    Sqrt,
    Root(PyObject),
    Round(Option<PyObject>),
    Floor,
    Ceil,
    Max,
    Min,
    Len,
    Pick(Vec<PyObject>),
    Unpick(Vec<PyObject>),
    Abs,
    Clamp(PyObject, PyObject),
    Sign,
    Log(Option<PyObject>),
    Exp,
    Pct(PyObject),
    Pctile(PyObject),
    Median,
    Q1,
    Q3,
    Iqr,
    Mode,
    Stdev,
    Between(PyObject, PyObject),
    Sum,
    Avg,
    Unique,
    Sorted(Option<PyObject>),
    First,
    Last,
    Contains(PyObject),
    In(PyObject),
    Lower,
    Upper,
    Title,
    Strip(Option<PyObject>),
    Replace(PyObject, PyObject),
    Split(Option<PyObject>),
    Join(PyObject),
    Startswith(PyObject),
    Endswith(PyObject),
    Matches(PyObject),
    Default(PyObject),
    Coalesce(Vec<PyObject>),
    Bool,
    TypeIs(PyObject),
    IsEmpty,
    NonEmpty,
    ToDatetime(Option<PyObject>),
    Timestamp,
    AgeSeconds,
    Before(PyObject),
    After(PyObject),
}

struct BuiltinFilterStep {
    filter: BuiltinFilter,
    map_suffix: bool,
}

type BuiltinFilterPipeline = Vec<BuiltinFilterStep>;

fn make_error(py: Python<'_>, class_name: &str, message: &str) -> PyErr {
    match py.import_bound("dictwalk.errors") {
        Ok(errors_module) => match errors_module.getattr(class_name) {
            Ok(error_type) => match error_type.call1((message,)) {
                Ok(instance) => PyErr::from_value_bound(instance),
                Err(_) => PyRuntimeError::new_err(message.to_string()),
            },
            Err(_) => PyRuntimeError::new_err(message.to_string()),
        },
        Err(_) => PyRuntimeError::new_err(message.to_string()),
    }
}

fn make_parse_error(py: Python<'_>, path: &str, token: Option<&str>, message: &str) -> PyErr {
    match py.import_bound("dictwalk.errors") {
        Ok(errors_module) => match errors_module.getattr("DictWalkParseError") {
            Ok(error_type) => {
                let token_obj = match token {
                    Some(value) => value.to_object(py),
                    None => py.None(),
                };
                match error_type.call1((path, token_obj, message)) {
                    Ok(instance) => PyErr::from_value_bound(instance),
                    Err(_) => PyRuntimeError::new_err(message.to_string()),
                }
            }
            Err(_) => PyRuntimeError::new_err(message.to_string()),
        },
        Err(_) => PyRuntimeError::new_err(message.to_string()),
    }
}

fn make_resolution_error(py: Python<'_>, path: &str, token: Option<&str>, message: &str) -> PyErr {
    match py.import_bound("dictwalk.errors") {
        Ok(errors_module) => match errors_module.getattr("DictWalkResolutionError") {
            Ok(error_type) => {
                let token_obj = match token {
                    Some(value) => value.to_object(py),
                    None => py.None(),
                };
                match error_type.call1((path, token_obj, message)) {
                    Ok(instance) => PyErr::from_value_bound(instance),
                    Err(_) => PyRuntimeError::new_err(message.to_string()),
                }
            }
            Err(_) => PyRuntimeError::new_err(message.to_string()),
        },
        Err(_) => PyRuntimeError::new_err(message.to_string()),
    }
}

fn load_registry(py: Python<'_>) -> PyResult<Bound<'_, PyAny>> {
    Ok(py.None().into_bound(py))
}

fn split_raw_path_tokens(path: &str) -> Vec<String> {
    let mut tokens: Vec<String> = Vec::new();
    let mut current = String::new();
    let mut bracket_depth = 0i32;

    for ch in path.chars() {
        if ch == '[' {
            bracket_depth += 1;
            current.push(ch);
            continue;
        }
        if ch == ']' {
            bracket_depth = (bracket_depth - 1).max(0);
            current.push(ch);
            continue;
        }
        if ch == '.' && bracket_depth == 0 {
            tokens.push(current);
            current = String::new();
            continue;
        }
        current.push(ch);
    }
    tokens.push(current);
    tokens
}

fn split_path_and_transform(path: &str) -> (String, Option<String>) {
    let mut bracket_depth = 0i32;
    let chars: Vec<char> = path.chars().collect();

    let mut i = 0usize;
    while i < chars.len() {
        let ch = chars[i];
        if ch == '[' {
            bracket_depth += 1;
            i += 1;
            continue;
        }
        if ch == ']' {
            bracket_depth = (bracket_depth - 1).max(0);
            i += 1;
            continue;
        }
        if ch == '|' && bracket_depth == 0 && i + 1 < chars.len() && chars[i + 1] == '$' {
            let base = chars[..i].iter().collect::<String>();
            let transform = chars[i + 1..].iter().collect::<String>();
            return (base, Some(transform));
        }
        i += 1;
    }
    (path.to_string(), None)
}

fn parse_token(raw_token: &str) -> Result<TokenKind, String> {
    if raw_token == "$$root" {
        return Ok(TokenKind::Root);
    }
    if raw_token == "*" {
        return Ok(TokenKind::Wildcard);
    }
    if raw_token == "**" {
        return Ok(TokenKind::DeepWildcard);
    }
    if raw_token.ends_with("[]") {
        return Ok(TokenKind::Map(raw_token[..raw_token.len() - 2].to_string()));
    }

    if let Some(captures) = INDEX_RE.captures(raw_token) {
        let key = captures
            .get(1)
            .map(|m| m.as_str().to_string())
            .ok_or("Failed to parse index key.")?;
        let index = captures
            .get(2)
            .and_then(|m| m.as_str().parse::<isize>().ok())
            .ok_or("Failed to parse list index.")?;
        return Ok(TokenKind::Index { key, index });
    }

    if let Some(captures) = SLICE_RE.captures(raw_token) {
        let key = captures
            .get(1)
            .map(|m| m.as_str().to_string())
            .ok_or("Failed to parse slice key.")?;
        let start = captures
            .get(2)
            .map(|m| m.as_str())
            .filter(|s| !s.is_empty())
            .and_then(|s| s.parse::<isize>().ok());
        let end = captures
            .get(3)
            .map(|m| m.as_str())
            .filter(|s| !s.is_empty())
            .and_then(|s| s.parse::<isize>().ok());
        return Ok(TokenKind::Slice { key, start, end });
    }

    if let Some(captures) = FILTER_RE.captures(raw_token) {
        let list_key = captures
            .get(1)
            .map(|m| m.as_str().to_string())
            .ok_or("Failed to parse filter list key.")?;
        let field = captures
            .get(2)
            .map(|m| m.as_str().to_string())
            .ok_or("Failed to parse filter field.")?;
        let operator = captures
            .get(3)
            .map(|m| m.as_str().to_string())
            .ok_or("Failed to parse filter operator.")?;
        let value = captures
            .get(4)
            .map(|m| m.as_str().to_string())
            .ok_or("Failed to parse filter value.")?;
        return Ok(TokenKind::Filter {
            list_key,
            field,
            operator,
            value,
        });
    }

    Ok(TokenKind::Get(raw_token.to_string()))
}

fn validate_filter_token(
    py: Python<'_>,
    _module: &Bound<'_, PyModule>,
    _registry: &Bound<'_, PyAny>,
    list_key: &str,
    field: &str,
    operator: &str,
    value: &str,
) -> PyResult<()> {
    if field.starts_with('$') {
        return Err(make_parse_error(
            py,
            &format!("{list_key}[?{field}{operator}{value}]"),
            Some(field),
            "Left-hand predicate filter functions must use '?.|$name' syntax (for example: '[?.|$len>3]').",
        ));
    }

    if field == "." {
        // Valid root-field expression.
    } else if let Some(field_transform) = field.strip_prefix(".|") {
        if compile_builtin_pipeline(py, field_transform, None).is_none() {
            return Err(make_parse_error(
                py,
                &format!("{list_key}[?{field}{operator}{value}]"),
                Some(field),
                &format!("Invalid left-hand predicate expression '{field}'."),
            ));
        }
    } else {
        // Validate expression syntax for field-side predicate filter expressions.
        if let Err(message) = compile_builtin_or_boolean_predicate(py, field) {
            return Err(make_parse_error(
                py,
                &format!("{list_key}[?{field}{operator}{value}]"),
                Some(field),
                &message,
            ));
        }
    }

    // Validate right-side predicate expression/filter syntax.
    if let Err(message) = compile_builtin_or_boolean_predicate(py, value) {
        return Err(make_parse_error(
            py,
            &format!("{list_key}[?{field}{operator}{value}]"),
            Some(value),
            &message,
        ));
    }

    Ok(())
}

fn parse_path(
    py: Python<'_>,
    module: &Bound<'_, PyModule>,
    registry: &Bound<'_, PyAny>,
    path: &str,
) -> PyResult<Vec<ParsedToken>> {
    if path.is_empty() {
        return Err(make_parse_error(py, path, None, "Path cannot be empty."));
    }

    let mut tokens: Vec<ParsedToken> = Vec::new();
    for raw_token in split_raw_path_tokens(path) {
        let kind = match parse_token(&raw_token) {
            Ok(parsed) => parsed,
            Err(message) => return Err(make_parse_error(py, path, Some(&raw_token), &message)),
        };

        if let TokenKind::Filter {
            list_key,
            field,
            operator,
            value,
        } = &kind
        {
            validate_filter_token(py, module, registry, list_key, field, operator, value)?;
        }

        tokens.push(ParsedToken {
            raw: raw_token,
            kind,
        });
    }
    Ok(tokens)
}

fn resolve_get_token(py: Python<'_>, current: &PyObject, key: &str) -> PyResult<PyObject> {
    let bound = current.bind(py);
    if let Ok(dict) = bound.downcast::<PyDict>() {
        let value = match dict.get_item(key)? {
            Some(inner) => inner,
            None => return Err(PyKeyError::new_err(key.to_string())),
        };
        return Ok(value.into());
    }

    if let Ok(list) = bound.downcast::<PyList>() {
        let out = PyList::empty_bound(py);
        for item in list.iter() {
            if let Ok(item_dict) = item.downcast::<PyDict>() {
                if item_dict.contains(key)? {
                    if let Some(value) = item_dict.get_item(key)? {
                        out.append(value)?;
                    }
                }
            }
        }
        return Ok(out.into());
    }

    Err(PyTypeError::new_err(format!(
        "Key '{key}' not found in current context."
    )))
}

fn get_type_name(bound: &Bound<'_, PyAny>) -> String {
    let bound_type = bound.get_type();
    bound_type
        .name()
        .map(|name: Bound<'_, PyString>| name.to_string_lossy().into_owned())
        .unwrap_or_else(|_| "unknown".to_string())
}

fn resolve_map_token(py: Python<'_>, current: &PyObject, key: &str) -> PyResult<PyObject> {
    let bound = current.bind(py);
    let type_name = get_type_name(&bound);
    let list = bound.downcast::<PyList>().map_err(|_| {
        PyTypeError::new_err(format!("Expected a list for key '{key}', got {type_name}."))
    })?;

    let out = PyList::empty_bound(py);
    for item in list.iter() {
        if let Ok(item_dict) = item.downcast::<PyDict>() {
            if item_dict.contains(key)? {
                if let Some(value) = item_dict.get_item(key)? {
                    out.append(value)?;
                }
            }
        }
    }
    Ok(out.into())
}

fn iter_child_nodes(py: Python<'_>, node: &Bound<'_, PyAny>) -> PyResult<Vec<PyObject>> {
    if let Ok(dict) = node.downcast::<PyDict>() {
        let mut out: Vec<PyObject> = Vec::new();
        for (_, value) in dict.iter() {
            out.push(value.into());
        }
        return Ok(out);
    }
    if let Ok(list) = node.downcast::<PyList>() {
        let mut out: Vec<PyObject> = Vec::new();
        for item in list.iter() {
            out.push(item.into());
        }
        return Ok(out);
    }
    let _ = py;
    Ok(Vec::new())
}

fn resolve_wildcard_token(py: Python<'_>, current: &PyObject) -> PyResult<PyObject> {
    let bound = current.bind(py);
    let type_name = get_type_name(&bound);
    let children = iter_child_nodes(py, &bound)?;
    if children.is_empty() && !bound.is_instance_of::<PyDict>() && !bound.is_instance_of::<PyList>()
    {
        return Err(PyTypeError::new_err(format!(
            "Expected dict or list for wildcard '*', got {type_name}."
        )));
    }

    let out = PyList::empty_bound(py);
    for child in children {
        out.append(child)?;
    }
    Ok(out.into())
}

fn collect_descendants(py: Python<'_>, node: PyObject, out: &Bound<'_, PyList>) -> PyResult<()> {
    let bound = node.bind(py);
    for child in iter_child_nodes(py, &bound)? {
        out.append(child.clone_ref(py))?;
        collect_descendants(py, child, out)?;
    }
    Ok(())
}

fn resolve_deep_wildcard_token(py: Python<'_>, current: &PyObject) -> PyResult<PyObject> {
    let bound = current.bind(py);
    let type_name = get_type_name(&bound);
    let direct_children = iter_child_nodes(py, &bound)?;
    if direct_children.is_empty()
        && !bound.is_instance_of::<PyDict>()
        && !bound.is_instance_of::<PyList>()
    {
        return Err(PyTypeError::new_err(format!(
            "Expected dict or list for wildcard '**', got {type_name}."
        )));
    }

    let out = PyList::empty_bound(py);
    for child in direct_children {
        out.append(child.clone_ref(py))?;
        collect_descendants(py, child, &out)?;
    }
    Ok(out.into())
}

fn apply_output_transform(
    py: Python<'_>,
    _module: &Bound<'_, PyModule>,
    _registry: &Bound<'_, PyAny>,
    current: &PyObject,
    transform: &str,
    root_data: &PyObject,
) -> PyResult<PyObject> {
    if let Some(pipeline) = compile_builtin_pipeline(py, transform, Some(root_data)) {
        return apply_builtin_pipeline(py, current.clone_ref(py), &pipeline);
    }
    Ok(current.clone_ref(py))
}

fn resolve_index_token(
    py: Python<'_>,
    current: &PyObject,
    key: &str,
    index: isize,
) -> PyResult<PyObject> {
    let bound = current.bind(py);
    let dict = bound.downcast::<PyDict>().map_err(|_| {
        PyTypeError::new_err(format!(
            "Expected a dict for key '{key}', got {}.",
            get_type_name(&bound)
        ))
    })?;

    let list_value = match dict.get_item(key)? {
        Some(value) => value,
        None => return Err(PyKeyError::new_err(key.to_string())),
    };
    let list = list_value.downcast::<PyList>().map_err(|_| {
        PyTypeError::new_err(format!(
            "Expected a list for key '{key}', got {}.",
            get_type_name(&list_value)
        ))
    })?;

    let index_obj = index.to_object(py);
    list.as_any().get_item(index_obj).map(|value| value.into())
}

fn resolve_slice_token(
    py: Python<'_>,
    current: &PyObject,
    key: &str,
    start: Option<isize>,
    end: Option<isize>,
) -> PyResult<PyObject> {
    let bound = current.bind(py);
    let dict = bound.downcast::<PyDict>().map_err(|_| {
        PyTypeError::new_err(format!(
            "Expected a dict for key '{key}', got {}.",
            get_type_name(&bound)
        ))
    })?;

    let list_value = match dict.get_item(key)? {
        Some(value) => value,
        None => return Err(PyKeyError::new_err(key.to_string())),
    };
    let list = list_value.downcast::<PyList>().map_err(|_| {
        PyTypeError::new_err(format!(
            "Expected a list for key '{key}', got {}.",
            get_type_name(&list_value)
        ))
    })?;

    let len = list.len() as isize;

    let mut slice_start = start.unwrap_or(0);
    if slice_start < 0 {
        slice_start += len;
    }
    if slice_start < 0 {
        slice_start = 0;
    }
    if slice_start > len {
        slice_start = len;
    }

    let mut slice_end = end.unwrap_or(len);
    if slice_end < 0 {
        slice_end += len;
    }
    if slice_end < 0 {
        slice_end = 0;
    }
    if slice_end > len {
        slice_end = len;
    }

    let out = PyList::empty_bound(py);
    if slice_start >= slice_end {
        return Ok(out.into());
    }

    for idx in slice_start..slice_end {
        out.append(list.get_item(idx as usize)?)?;
    }
    Ok(out.into())
}

fn parse_literal(py: Python<'_>, value: &str) -> PyObject {
    match py.import_bound("ast") {
        Ok(ast) => match ast.getattr("literal_eval") {
            Ok(literal_eval) => match literal_eval.call1((value,)) {
                Ok(parsed) => parsed.into(),
                Err(_) => value.to_object(py),
            },
            Err(_) => value.to_object(py),
        },
        Err(_) => value.to_object(py),
    }
}

fn split_filter_args(args_string: &str) -> Option<Vec<String>> {
    let mut out: Vec<String> = Vec::new();
    let mut current = String::new();
    let mut paren_depth = 0i32;
    let mut bracket_depth = 0i32;
    let mut brace_depth = 0i32;
    let mut in_single = false;
    let mut in_double = false;
    let mut escaped = false;

    for ch in args_string.chars() {
        if escaped {
            current.push(ch);
            escaped = false;
            continue;
        }
        if ch == '\\' {
            current.push(ch);
            escaped = true;
            continue;
        }

        if in_single {
            current.push(ch);
            if ch == '\'' {
                in_single = false;
            }
            continue;
        }
        if in_double {
            current.push(ch);
            if ch == '"' {
                in_double = false;
            }
            continue;
        }

        match ch {
            '\'' => {
                in_single = true;
                current.push(ch);
            }
            '"' => {
                in_double = true;
                current.push(ch);
            }
            '(' => {
                paren_depth += 1;
                current.push(ch);
            }
            ')' => {
                paren_depth -= 1;
                if paren_depth < 0 {
                    return None;
                }
                current.push(ch);
            }
            '[' => {
                bracket_depth += 1;
                current.push(ch);
            }
            ']' => {
                bracket_depth -= 1;
                if bracket_depth < 0 {
                    return None;
                }
                current.push(ch);
            }
            '{' => {
                brace_depth += 1;
                current.push(ch);
            }
            '}' => {
                brace_depth -= 1;
                if brace_depth < 0 {
                    return None;
                }
                current.push(ch);
            }
            ',' if paren_depth == 0 && bracket_depth == 0 && brace_depth == 0 => {
                out.push(current.trim().to_string());
                current.clear();
            }
            _ => current.push(ch),
        }
    }

    if in_single || in_double || paren_depth != 0 || bracket_depth != 0 || brace_depth != 0 {
        return None;
    }

    if !current.trim().is_empty() {
        out.push(current.trim().to_string());
    } else if !args_string.trim().is_empty() {
        return None;
    }

    Some(out)
}

fn parse_filter_args(
    py: Python<'_>,
    args_string: &str,
    root_data: Option<&PyObject>,
) -> Option<Vec<PyObject>> {
    let arg_tokens = split_filter_args(args_string)?;
    let mut out: Vec<PyObject> = Vec::new();
    for token in arg_tokens {
        if token.starts_with("$$root") {
            let root = root_data?;
            let resolved = resolve_root_reference_value(py, root, &token).ok()?;
            out.push(resolved);
            continue;
        }
        out.push(parse_literal(py, &token));
    }
    Some(out)
}

fn compile_builtin_filter(py: Python<'_>, name: &str, args: &[PyObject]) -> Option<BuiltinFilter> {
    match (name, args.len()) {
        ("inc", 0) => Some(BuiltinFilter::Inc),
        ("dec", 0) => Some(BuiltinFilter::Dec),
        ("double", 0) => Some(BuiltinFilter::Double),
        ("square", 0) => Some(BuiltinFilter::Square),
        ("string", 0) => Some(BuiltinFilter::String),
        ("int", 0) => Some(BuiltinFilter::Int),
        ("float", 0) => Some(BuiltinFilter::Float),
        ("decimal", 0) => Some(BuiltinFilter::Decimal),
        ("round", 0) => Some(BuiltinFilter::Round(None)),
        ("round", 1) => Some(BuiltinFilter::Round(Some(args[0].clone_ref(py)))),
        ("floor", 0) => Some(BuiltinFilter::Floor),
        ("ceil", 0) => Some(BuiltinFilter::Ceil),
        ("quote", 0) => Some(BuiltinFilter::Quote),
        ("even", 0) => Some(BuiltinFilter::Even),
        ("odd", 0) => Some(BuiltinFilter::Odd),
        ("neg", 0) => Some(BuiltinFilter::Neg),
        ("pow", 1) => Some(BuiltinFilter::Pow(args[0].clone_ref(py))),
        ("rpow", 1) => Some(BuiltinFilter::RPow(args[0].clone_ref(py))),
        ("sqrt", 0) => Some(BuiltinFilter::Sqrt),
        ("root", 1) => Some(BuiltinFilter::Root(args[0].clone_ref(py))),
        ("max", 0) => Some(BuiltinFilter::Max),
        ("min", 0) => Some(BuiltinFilter::Min),
        ("len", 0) => Some(BuiltinFilter::Len),
        ("pick", n) => Some(BuiltinFilter::Pick(
            args.iter().take(n).map(|arg| arg.clone_ref(py)).collect(),
        )),
        ("unpick", n) => Some(BuiltinFilter::Unpick(
            args.iter().take(n).map(|arg| arg.clone_ref(py)).collect(),
        )),
        ("abs", 0) => Some(BuiltinFilter::Abs),
        ("clamp", 2) => Some(BuiltinFilter::Clamp(
            args[0].clone_ref(py),
            args[1].clone_ref(py),
        )),
        ("sign", 0) => Some(BuiltinFilter::Sign),
        ("log", 0) => Some(BuiltinFilter::Log(None)),
        ("log", 1) => Some(BuiltinFilter::Log(Some(args[0].clone_ref(py)))),
        ("exp", 0) => Some(BuiltinFilter::Exp),
        ("pct", 1) => Some(BuiltinFilter::Pct(args[0].clone_ref(py))),
        ("pctile", 1) => Some(BuiltinFilter::Pctile(args[0].clone_ref(py))),
        ("median", 0) => Some(BuiltinFilter::Median),
        ("q1", 0) => Some(BuiltinFilter::Q1),
        ("q3", 0) => Some(BuiltinFilter::Q3),
        ("iqr", 0) => Some(BuiltinFilter::Iqr),
        ("mode", 0) => Some(BuiltinFilter::Mode),
        ("stdev", 0) => Some(BuiltinFilter::Stdev),
        ("between", 2) => Some(BuiltinFilter::Between(
            args[0].clone_ref(py),
            args[1].clone_ref(py),
        )),
        ("sum", 0) => Some(BuiltinFilter::Sum),
        ("avg", 0) => Some(BuiltinFilter::Avg),
        ("unique", 0) => Some(BuiltinFilter::Unique),
        ("sorted", 0) => Some(BuiltinFilter::Sorted(None)),
        ("sorted", 1) => Some(BuiltinFilter::Sorted(Some(args[0].clone_ref(py)))),
        ("first", 0) => Some(BuiltinFilter::First),
        ("last", 0) => Some(BuiltinFilter::Last),
        ("contains", 1) => Some(BuiltinFilter::Contains(args[0].clone_ref(py))),
        ("in", 1) => Some(BuiltinFilter::In(args[0].clone_ref(py))),
        ("lower", 0) => Some(BuiltinFilter::Lower),
        ("upper", 0) => Some(BuiltinFilter::Upper),
        ("title", 0) => Some(BuiltinFilter::Title),
        ("strip", 0) => Some(BuiltinFilter::Strip(None)),
        ("strip", 1) => Some(BuiltinFilter::Strip(Some(args[0].clone_ref(py)))),
        ("replace", 2) => Some(BuiltinFilter::Replace(
            args[0].clone_ref(py),
            args[1].clone_ref(py),
        )),
        ("split", 0) => Some(BuiltinFilter::Split(None)),
        ("split", 1) => Some(BuiltinFilter::Split(Some(args[0].clone_ref(py)))),
        ("join", 1) => Some(BuiltinFilter::Join(args[0].clone_ref(py))),
        ("startswith", 1) => Some(BuiltinFilter::Startswith(args[0].clone_ref(py))),
        ("endswith", 1) => Some(BuiltinFilter::Endswith(args[0].clone_ref(py))),
        ("matches", 1) => Some(BuiltinFilter::Matches(args[0].clone_ref(py))),
        ("default", 1) => Some(BuiltinFilter::Default(args[0].clone_ref(py))),
        ("coalesce", n) if n >= 1 => Some(BuiltinFilter::Coalesce(
            args.iter().map(|arg| arg.clone_ref(py)).collect(),
        )),
        ("bool", 0) => Some(BuiltinFilter::Bool),
        ("type_is", 1) => Some(BuiltinFilter::TypeIs(args[0].clone_ref(py))),
        ("is_empty", 0) => Some(BuiltinFilter::IsEmpty),
        ("non_empty", 0) => Some(BuiltinFilter::NonEmpty),
        ("to_datetime", 0) => Some(BuiltinFilter::ToDatetime(None)),
        ("to_datetime", 1) => Some(BuiltinFilter::ToDatetime(Some(args[0].clone_ref(py)))),
        ("timestamp", 0) => Some(BuiltinFilter::Timestamp),
        ("age_seconds", 0) => Some(BuiltinFilter::AgeSeconds),
        ("before", 1) => Some(BuiltinFilter::Before(args[0].clone_ref(py))),
        ("after", 1) => Some(BuiltinFilter::After(args[0].clone_ref(py))),
        ("gt", 1) => Some(BuiltinFilter::Gt(args[0].clone_ref(py))),
        ("lt", 1) => Some(BuiltinFilter::Lt(args[0].clone_ref(py))),
        ("gte", 1) => Some(BuiltinFilter::Gte(args[0].clone_ref(py))),
        ("lte", 1) => Some(BuiltinFilter::Lte(args[0].clone_ref(py))),
        ("add", 1) => Some(BuiltinFilter::Add(args[0].clone_ref(py))),
        ("sub", 1) => Some(BuiltinFilter::Sub(args[0].clone_ref(py))),
        ("mul", 1) => Some(BuiltinFilter::Mul(args[0].clone_ref(py))),
        ("div", 1) => Some(BuiltinFilter::Div(args[0].clone_ref(py))),
        ("mod", 1) => Some(BuiltinFilter::Mod(args[0].clone_ref(py))),
        _ => None,
    }
}

fn compile_builtin_pipeline(
    py: Python<'_>,
    expression: &str,
    root_data: Option<&PyObject>,
) -> Option<BuiltinFilterPipeline> {
    if !expression.starts_with('$') {
        return None;
    }

    let mut out: BuiltinFilterPipeline = Vec::new();
    for segment in expression.split('|') {
        let captures = PATH_FILTER_SEGMENT_RE.captures(segment)?;
        let name = captures.get(1)?.as_str();
        let args = if let Some(args_match) = captures.get(2) {
            parse_filter_args(py, args_match.as_str(), root_data)?
        } else {
            Vec::new()
        };
        let map_suffix = captures.get(3).is_some();
        let filter = compile_builtin_filter(py, name, &args)?;
        out.push(BuiltinFilterStep { filter, map_suffix });
    }

    Some(out)
}

fn apply_binary_op(
    py: Python<'_>,
    left: &PyObject,
    method: &str,
    right: &PyObject,
) -> PyResult<PyObject> {
    let direct = left.bind(py).call_method1(method, (right.clone_ref(py),))?;
    if !direct.is(py.NotImplemented().bind(py)) {
        return Ok(direct.into());
    }

    let reflected_method = match method {
        "__add__" => "__radd__",
        "__sub__" => "__rsub__",
        "__mul__" => "__rmul__",
        "__truediv__" => "__rtruediv__",
        "__mod__" => "__rmod__",
        _ => return Ok(direct.into()),
    };

    let reflected = right
        .bind(py)
        .call_method1(reflected_method, (left.clone_ref(py),))?;
    if !reflected.is(py.NotImplemented().bind(py)) {
        return Ok(reflected.into());
    }

    let operator_fn = match method {
        "__add__" => "add",
        "__sub__" => "sub",
        "__mul__" => "mul",
        "__truediv__" => "truediv",
        "__mod__" => "mod",
        _ => return Ok(direct.into()),
    };

    py.import_bound("operator")?
        .getattr(operator_fn)?
        .call1((left.clone_ref(py), right.clone_ref(py)))
        .map(|value| value.into())
}

fn call_builtin1(py: Python<'_>, name: &str, arg: &PyObject) -> PyResult<PyObject> {
    py.import_bound("builtins")?
        .getattr(name)?
        .call1((arg.clone_ref(py),))
        .map(|v| v.into())
}

fn call_builtin2(
    py: Python<'_>,
    name: &str,
    arg1: &PyObject,
    arg2: &PyObject,
) -> PyResult<PyObject> {
    py.import_bound("builtins")?
        .getattr(name)?
        .call1((arg1.clone_ref(py), arg2.clone_ref(py)))
        .map(|v| v.into())
}

fn compare_with_fallback(
    py: Python<'_>,
    left: &PyObject,
    right: &PyObject,
    operator: &str,
) -> PyResult<bool> {
    match compare_values(py, left, right, operator) {
        Ok(result) => Ok(result),
        Err(err) => {
            if !err.is_instance_of::<PyTypeError>(py) {
                return Err(err);
            }
            let left_str = left.bind(py).str()?.to_string_lossy().to_string();
            let right_str = right.bind(py).str()?.to_string_lossy().to_string();
            compare_values(
                py,
                &left_str.to_object(py),
                &right_str.to_object(py),
                operator,
            )
        }
    }
}

fn has_len_zero(py: Python<'_>, value: &PyObject) -> bool {
    value.bind(py).len().map(|len| len == 0).unwrap_or(false)
}

fn as_datetime(
    py: Python<'_>,
    value: &PyObject,
    fmt: Option<&PyObject>,
) -> PyResult<Option<PyObject>> {
    let datetime_mod = py.import_bound("datetime")?;
    let datetime_type = datetime_mod.getattr("datetime")?;
    let timezone_type = datetime_mod.getattr("timezone")?;
    let utc = timezone_type.getattr("utc")?;
    let value_bound = value.bind(py);

    if value_bound.is_instance(&datetime_type)? {
        return Ok(Some(value.clone_ref(py)));
    }

    if value_bound.is_instance_of::<PyInt>() || value_bound.is_instance_of::<PyFloat>() {
        let dt = datetime_type.call_method1("fromtimestamp", (value.clone_ref(py), utc))?;
        return Ok(Some(dt.into()));
    }

    if !value_bound.is_instance_of::<PyString>() {
        return Ok(None);
    }

    if let Some(fmt_value) = fmt {
        let dt = datetime_type
            .call_method1("strptime", (value.clone_ref(py), fmt_value.clone_ref(py)))?;
        return Ok(Some(dt.into()));
    }

    let normalized = value_bound.str()?.to_string_lossy().replace('Z', "+00:00");
    let dt = datetime_type.call_method1("fromisoformat", (normalized,))?;
    Ok(Some(dt.into()))
}

fn collect_numeric_sequence(py: Python<'_>, value: &PyObject) -> PyResult<Option<Vec<f64>>> {
    let value_bound = value.bind(py);
    if !(value_bound.is_instance_of::<PyList>() || value_bound.is_instance_of::<PyTuple>()) {
        return Ok(None);
    }

    let len = value_bound.len()?;
    let mut values: Vec<f64> = Vec::with_capacity(len);
    for idx in 0..len {
        let item_obj: PyObject = value_bound.get_item(idx)?.into();
        let float_obj = call_builtin1(py, "float", &item_obj)?;
        values.push(float_obj.bind(py).extract::<f64>()?);
    }

    Ok(Some(values))
}

fn percentile_value(sorted_values: &[f64], percentile: f64) -> Option<f64> {
    if sorted_values.is_empty() || !(0.0..=100.0).contains(&percentile) {
        return None;
    }
    if sorted_values.len() == 1 {
        return Some(sorted_values[0]);
    }

    let rank = (percentile / 100.0) * (sorted_values.len() as f64 - 1.0);
    let lower_idx = rank.floor() as usize;
    let upper_idx = rank.ceil() as usize;
    let fraction = rank - lower_idx as f64;

    let lower = sorted_values[lower_idx];
    let upper = sorted_values[upper_idx];
    Some(lower + (upper - lower) * fraction)
}

fn apply_builtin_filter(
    py: Python<'_>,
    value: &PyObject,
    filter: &BuiltinFilter,
) -> PyResult<PyObject> {
    match filter {
        BuiltinFilter::Inc => apply_binary_op(py, value, "__add__", &1i32.to_object(py)),
        BuiltinFilter::Dec => apply_binary_op(py, value, "__sub__", &1i32.to_object(py)),
        BuiltinFilter::Double => apply_binary_op(py, value, "__mul__", &2i32.to_object(py)),
        BuiltinFilter::Square => apply_binary_op(py, value, "__mul__", value),
        BuiltinFilter::String => value.bind(py).str().map(|s| s.into()),
        BuiltinFilter::Int => call_builtin1(py, "int", value),
        BuiltinFilter::Float => call_builtin1(py, "float", value),
        BuiltinFilter::Decimal => py
            .import_bound("decimal")?
            .getattr("Decimal")?
            .call1((value.clone_ref(py),))
            .map(|v| v.into()),
        BuiltinFilter::Round(ndigits) => {
            if let Some(nd) = ndigits {
                value
                    .bind(py)
                    .call_method1("__round__", (nd.clone_ref(py),))
                    .map(|v| v.into())
            } else {
                value.bind(py).call_method0("__round__").map(|v| v.into())
            }
        }
        BuiltinFilter::Floor => py
            .import_bound("math")?
            .getattr("floor")?
            .call1((value.clone_ref(py),))
            .map(|v| v.into()),
        BuiltinFilter::Ceil => py
            .import_bound("math")?
            .getattr("ceil")?
            .call1((value.clone_ref(py),))
            .map(|v| v.into()),
        BuiltinFilter::Quote => {
            let inner = value.bind(py).str()?.to_string_lossy().to_string();
            Ok(format!("\"{inner}\"").to_object(py))
        }
        BuiltinFilter::Even | BuiltinFilter::Odd => {
            let is_int = value.bind(py).is_instance_of::<PyInt>();
            if !is_int {
                return Ok(false.to_object(py));
            }
            let rem = apply_binary_op(py, value, "__mod__", &2i32.to_object(py))?;
            let expected = if matches!(filter, BuiltinFilter::Even) {
                0
            } else {
                1
            };
            Ok(compare_values(py, &rem, &expected.to_object(py), "==")?.to_object(py))
        }
        BuiltinFilter::Gt(threshold) => {
            Ok(compare_with_fallback(py, value, threshold, ">")?.to_object(py))
        }
        BuiltinFilter::Lt(threshold) => {
            Ok(compare_with_fallback(py, value, threshold, "<")?.to_object(py))
        }
        BuiltinFilter::Gte(threshold) => {
            Ok(compare_with_fallback(py, value, threshold, ">=")?.to_object(py))
        }
        BuiltinFilter::Lte(threshold) => {
            Ok(compare_with_fallback(py, value, threshold, "<=")?.to_object(py))
        }
        BuiltinFilter::Add(rhs) => apply_binary_op(py, value, "__add__", rhs),
        BuiltinFilter::Sub(rhs) => apply_binary_op(py, value, "__sub__", rhs),
        BuiltinFilter::Mul(rhs) => apply_binary_op(py, value, "__mul__", rhs),
        BuiltinFilter::Div(rhs) => {
            let is_zero = compare_values(py, rhs, &0i32.to_object(py), "==").unwrap_or(false);
            if is_zero {
                return Ok(py.None());
            }
            apply_binary_op(py, value, "__truediv__", rhs)
        }
        BuiltinFilter::Mod(rhs) => {
            let is_zero = compare_values(py, rhs, &0i32.to_object(py), "==").unwrap_or(false);
            if is_zero {
                return Ok(py.None());
            }
            apply_binary_op(py, value, "__mod__", rhs)
        }
        BuiltinFilter::Neg => value
            .bind(py)
            .call_method0("__neg__")
            .map(|result| result.into()),
        BuiltinFilter::Pow(exponent) => call_builtin2(py, "pow", value, exponent),
        BuiltinFilter::RPow(base) => call_builtin2(py, "pow", base, value),
        BuiltinFilter::Sqrt => {
            if compare_with_fallback(py, value, &0i32.to_object(py), "<")? {
                return Ok(py.None());
            }
            call_builtin2(py, "pow", value, &0.5f64.to_object(py))
        }
        BuiltinFilter::Root(degree) => {
            if compare_with_fallback(py, value, &0i32.to_object(py), "<")?
                || compare_with_fallback(py, degree, &0i32.to_object(py), "<=")?
            {
                return Ok(py.None());
            }
            let exponent = apply_binary_op(py, &1f64.to_object(py), "__truediv__", degree)?;
            call_builtin2(py, "pow", value, &exponent)
        }
        BuiltinFilter::Max => {
            let value_bound = value.bind(py);
            if value_bound.is_instance_of::<PyList>() || value_bound.is_instance_of::<PyTuple>() {
                return call_builtin1(py, "max", value);
            }
            Ok(value.clone_ref(py))
        }
        BuiltinFilter::Min => {
            let value_bound = value.bind(py);
            if value_bound.is_instance_of::<PyList>() || value_bound.is_instance_of::<PyTuple>() {
                return call_builtin1(py, "min", value);
            }
            Ok(value.clone_ref(py))
        }
        BuiltinFilter::Len => Ok(value.bind(py).len()?.to_object(py)),
        BuiltinFilter::Pick(keys) => {
            if !value.bind(py).is_instance_of::<PyDict>() {
                return Ok(py.None());
            }
            let source = value.bind(py).downcast::<PyDict>()?;
            let out = PyDict::new_bound(py);
            for key in keys {
                if source.contains(key.clone_ref(py))? {
                    if let Some(v) = source.get_item(key.clone_ref(py))? {
                        out.set_item(key.clone_ref(py), v)?;
                    }
                }
            }
            Ok(out.into())
        }
        BuiltinFilter::Unpick(keys) => {
            if !value.bind(py).is_instance_of::<PyDict>() {
                return Ok(py.None());
            }
            let source = value.bind(py).downcast::<PyDict>()?;
            let out = PyDict::new_bound(py);
            for (key, v) in source.iter() {
                let key_obj = key.to_object(py);
                let mut remove = false;
                for candidate in keys {
                    if compare_values(py, &key_obj, candidate, "==").unwrap_or(false) {
                        remove = true;
                        break;
                    }
                }
                if !remove {
                    out.set_item(key, v)?;
                }
            }
            Ok(out.into())
        }
        BuiltinFilter::Abs => call_builtin1(py, "abs", value),
        BuiltinFilter::Clamp(min_value, max_value) => {
            let min_applied = call_builtin2(py, "max", min_value, value)?;
            call_builtin2(py, "min", max_value, &min_applied)
        }
        BuiltinFilter::Sign => Ok((compare_with_fallback(py, value, &0i32.to_object(py), ">")?
            as i32
            - compare_with_fallback(py, value, &0i32.to_object(py), "<")? as i32)
            .to_object(py)),
        BuiltinFilter::Log(base) => {
            let base = base
                .as_ref()
                .map(|v| v.clone_ref(py))
                .unwrap_or_else(|| std::f64::consts::E.to_object(py));
            if !compare_with_fallback(py, value, &0i32.to_object(py), ">")?
                || !compare_with_fallback(py, &base, &0i32.to_object(py), ">")?
                || compare_with_fallback(py, &base, &1i32.to_object(py), "==")?
            {
                return Ok(py.None());
            }
            py.import_bound("math")?
                .getattr("log")?
                .call1((value.clone_ref(py), base))
                .map(|v| v.into())
        }
        BuiltinFilter::Exp => py
            .import_bound("math")?
            .getattr("exp")?
            .call1((value.clone_ref(py),))
            .map(|v| v.into()),
        BuiltinFilter::Pct(percent) => {
            let percent_float = call_builtin1(py, "float", percent)?;
            let value_float = call_builtin1(py, "float", value)?;
            let scale = apply_binary_op(py, &percent_float, "__truediv__", &100f64.to_object(py))?;
            apply_binary_op(py, &value_float, "__mul__", &scale)
        }
        BuiltinFilter::Pctile(percentile) => {
            let Some(mut values) = collect_numeric_sequence(py, value)? else {
                return Ok(value.clone_ref(py));
            };
            if values.is_empty() {
                return Ok(py.None());
            }

            let p_obj = call_builtin1(py, "float", percentile)?;
            let p = p_obj.bind(py).extract::<f64>()?;
            values.sort_by(|a, b| a.total_cmp(b));
            let Some(result) = percentile_value(&values, p) else {
                return Ok(py.None());
            };
            Ok(result.to_object(py))
        }
        BuiltinFilter::Median => {
            let Some(mut values) = collect_numeric_sequence(py, value)? else {
                return Ok(value.clone_ref(py));
            };
            if values.is_empty() {
                return Ok(py.None());
            }
            values.sort_by(|a, b| a.total_cmp(b));
            let result = percentile_value(&values, 50.0).expect("non-empty checked");
            Ok(result.to_object(py))
        }
        BuiltinFilter::Q1 => {
            let Some(mut values) = collect_numeric_sequence(py, value)? else {
                return Ok(value.clone_ref(py));
            };
            if values.is_empty() {
                return Ok(py.None());
            }
            values.sort_by(|a, b| a.total_cmp(b));
            let result = percentile_value(&values, 25.0).expect("non-empty checked");
            Ok(result.to_object(py))
        }
        BuiltinFilter::Q3 => {
            let Some(mut values) = collect_numeric_sequence(py, value)? else {
                return Ok(value.clone_ref(py));
            };
            if values.is_empty() {
                return Ok(py.None());
            }
            values.sort_by(|a, b| a.total_cmp(b));
            let result = percentile_value(&values, 75.0).expect("non-empty checked");
            Ok(result.to_object(py))
        }
        BuiltinFilter::Iqr => {
            let Some(mut values) = collect_numeric_sequence(py, value)? else {
                return Ok(value.clone_ref(py));
            };
            if values.is_empty() {
                return Ok(py.None());
            }
            values.sort_by(|a, b| a.total_cmp(b));
            let q1 = percentile_value(&values, 25.0).expect("non-empty checked");
            let q3 = percentile_value(&values, 75.0).expect("non-empty checked");
            Ok((q3 - q1).to_object(py))
        }
        BuiltinFilter::Mode => {
            let value_bound = value.bind(py);
            if !(value_bound.is_instance_of::<PyList>() || value_bound.is_instance_of::<PyTuple>())
            {
                return Ok(value.clone_ref(py));
            }

            let len = value_bound.len()?;
            if len == 0 {
                return Ok(py.None());
            }

            let mut best: PyObject = py.None();
            let mut best_count: usize = 0;

            for idx in 0..len {
                let candidate: PyObject = value_bound.get_item(idx)?.into();
                let mut count = 0usize;
                for j in 0..len {
                    let item: PyObject = value_bound.get_item(j)?.into();
                    if compare_values(py, &item, &candidate, "==").unwrap_or(false) {
                        count += 1;
                    }
                }
                if count > best_count {
                    best_count = count;
                    best = candidate;
                }
            }

            Ok(best)
        }
        BuiltinFilter::Stdev => {
            let Some(values) = collect_numeric_sequence(py, value)? else {
                return Ok(value.clone_ref(py));
            };
            if values.is_empty() {
                return Ok(py.None());
            }
            let n = values.len() as f64;
            let mean = values.iter().sum::<f64>() / n;
            let variance = values
                .iter()
                .map(|x| {
                    let diff = *x - mean;
                    diff * diff
                })
                .sum::<f64>()
                / n;
            Ok(variance.sqrt().to_object(py))
        }
        BuiltinFilter::Between(min_value, max_value) => {
            let ge_min = compare_with_fallback(py, value, min_value, ">=")?;
            let le_max = compare_with_fallback(py, value, max_value, "<=")?;
            Ok((ge_min && le_max).to_object(py))
        }
        BuiltinFilter::Sum => {
            let value_bound = value.bind(py);
            if value_bound.is_instance_of::<PyList>() || value_bound.is_instance_of::<PyTuple>() {
                return call_builtin1(py, "sum", value);
            }
            Ok(value.clone_ref(py))
        }
        BuiltinFilter::Avg => {
            let value_bound = value.bind(py);
            if value_bound.is_instance_of::<PyList>() || value_bound.is_instance_of::<PyTuple>() {
                let len = value_bound.len()?;
                if len == 0 {
                    return Ok(py.None());
                }
                let sum_value = call_builtin1(py, "sum", value)?;
                return apply_binary_op(py, &sum_value, "__truediv__", &(len as i64).to_object(py));
            }
            Ok(value.clone_ref(py))
        }
        BuiltinFilter::Unique => {
            if !value.bind(py).is_instance_of::<PyList>() {
                return Ok(value.clone_ref(py));
            }
            let dict_type = py.import_bound("builtins")?.getattr("dict")?;
            let fromkeys = dict_type.getattr("fromkeys")?;
            let dedup_dict = fromkeys.call1((value.clone_ref(py),))?;
            call_builtin1(py, "list", &dedup_dict.into())
        }
        BuiltinFilter::Sorted(reverse) => {
            let value_bound = value.bind(py);
            if !(value_bound.is_instance_of::<PyList>() || value_bound.is_instance_of::<PyTuple>())
            {
                return Ok(value.clone_ref(py));
            }
            if let Some(reverse_flag) = reverse {
                let kwargs = PyDict::new_bound(py);
                kwargs.set_item("reverse", reverse_flag.clone_ref(py))?;
                py.import_bound("builtins")?
                    .getattr("sorted")?
                    .call((value.clone_ref(py),), Some(&kwargs))
                    .map(|v| v.into())
            } else {
                call_builtin1(py, "sorted", value)
            }
        }
        BuiltinFilter::First => {
            let value_bound = value.bind(py);
            if value_bound.is_instance_of::<PyList>() || value_bound.is_instance_of::<PyTuple>() {
                if value_bound.len()? == 0 {
                    return Ok(py.None());
                }
                return value_bound.get_item(0).map(|v| v.into());
            }
            Ok(value.clone_ref(py))
        }
        BuiltinFilter::Last => {
            let value_bound = value.bind(py);
            if value_bound.is_instance_of::<PyList>() || value_bound.is_instance_of::<PyTuple>() {
                let len = value_bound.len()?;
                if len == 0 {
                    return Ok(py.None());
                }
                return value_bound.get_item(len - 1).map(|v| v.into());
            }
            Ok(value.clone_ref(py))
        }
        BuiltinFilter::Contains(needle) => {
            Ok(value.bind(py).contains(needle.clone_ref(py))?.to_object(py))
        }
        BuiltinFilter::In(haystack) => Ok(haystack
            .bind(py)
            .contains(value.clone_ref(py))?
            .to_object(py)),
        BuiltinFilter::Lower => value
            .bind(py)
            .str()?
            .call_method0("lower")
            .map(|v| v.into()),
        BuiltinFilter::Upper => value
            .bind(py)
            .str()?
            .call_method0("upper")
            .map(|v| v.into()),
        BuiltinFilter::Title => value
            .bind(py)
            .str()?
            .call_method0("title")
            .map(|v| v.into()),
        BuiltinFilter::Strip(chars) => {
            let s = value.bind(py).str()?;
            if let Some(chars) = chars {
                s.call_method1("strip", (chars.clone_ref(py),))
                    .map(|v| v.into())
            } else {
                s.call_method0("strip").map(|v| v.into())
            }
        }
        BuiltinFilter::Replace(old, new) => value
            .bind(py)
            .str()?
            .call_method1("replace", (old.clone_ref(py), new.clone_ref(py)))
            .map(|v| v.into()),
        BuiltinFilter::Split(sep) => {
            let s = value.bind(py).str()?;
            if let Some(sep) = sep {
                s.call_method1("split", (sep.clone_ref(py),))
                    .map(|v| v.into())
            } else {
                s.call_method0("split").map(|v| v.into())
            }
        }
        BuiltinFilter::Join(sep) => {
            let sep_obj = sep.bind(py).str()?;
            let join_input = if value.bind(py).is_instance_of::<PyList>()
                || value.bind(py).is_instance_of::<PyTuple>()
            {
                let builtins = py.import_bound("builtins")?;
                builtins
                    .getattr("map")?
                    .call1((builtins.getattr("str")?, value.clone_ref(py)))?
            } else {
                return value.bind(py).str().map(|s| s.into());
            };
            sep_obj
                .call_method1("join", (join_input,))
                .map(|v| v.into())
        }
        BuiltinFilter::Startswith(prefix) => value
            .bind(py)
            .str()?
            .call_method1("startswith", (prefix.clone_ref(py),))
            .map(|v| v.into()),
        BuiltinFilter::Endswith(suffix) => value
            .bind(py)
            .str()?
            .call_method1("endswith", (suffix.clone_ref(py),))
            .map(|v| v.into()),
        BuiltinFilter::Matches(pattern) => {
            let re = py.import_bound("re")?;
            let searched = re
                .getattr("search")?
                .call1((pattern.clone_ref(py), value.bind(py).str()?))?;
            Ok((!searched.is_none()).to_object(py))
        }
        BuiltinFilter::Default(default_value) => {
            if value.bind(py).is_none() {
                Ok(default_value.clone_ref(py))
            } else {
                Ok(value.clone_ref(py))
            }
        }
        BuiltinFilter::Coalesce(values) => {
            if !value.bind(py).is_none() {
                return Ok(value.clone_ref(py));
            }
            for item in values {
                if !item.bind(py).is_none() {
                    return Ok(item.clone_ref(py));
                }
            }
            Ok(py.None())
        }
        BuiltinFilter::Bool => {
            if value.bind(py).is_instance_of::<PyString>() {
                let normalized = value
                    .bind(py)
                    .str()?
                    .to_string_lossy()
                    .trim()
                    .to_lowercase();
                return Ok(
                    matches!(normalized.as_str(), "1" | "true" | "yes" | "y" | "on").to_object(py),
                );
            }
            Ok(value.bind(py).is_truthy()?.to_object(py))
        }
        BuiltinFilter::TypeIs(name) => {
            let type_name = value
                .bind(py)
                .get_type()
                .name()?
                .to_string_lossy()
                .to_lowercase();
            let expected = name.bind(py).str()?.to_string_lossy().to_lowercase();
            Ok((type_name == expected).to_object(py))
        }
        BuiltinFilter::IsEmpty => {
            let result = value.bind(py).is_none() || has_len_zero(py, value);
            Ok(result.to_object(py))
        }
        BuiltinFilter::NonEmpty => {
            let result = !(value.bind(py).is_none() || has_len_zero(py, value));
            Ok(result.to_object(py))
        }
        BuiltinFilter::ToDatetime(fmt) => {
            Ok(as_datetime(py, value, fmt.as_ref())?.unwrap_or_else(|| py.None()))
        }
        BuiltinFilter::Timestamp => {
            let dt = match as_datetime(py, value, None)? {
                Some(dt) => dt,
                None => return Ok(py.None()),
            };
            dt.bind(py).call_method0("timestamp").map(|v| v.into())
        }
        BuiltinFilter::AgeSeconds => {
            let dt = match as_datetime(py, value, None)? {
                Some(dt) => dt,
                None => return Ok(py.None()),
            };
            let datetime_mod = py.import_bound("datetime")?;
            let datetime_type = datetime_mod.getattr("datetime")?;
            let timezone_utc = datetime_mod.getattr("timezone")?.getattr("utc")?;
            let tzinfo = dt.bind(py).getattr("tzinfo")?;
            let now = if tzinfo.is_none() {
                datetime_type.call_method1("now", (timezone_utc,))?
            } else {
                datetime_type.call_method1("now", (tzinfo,))?
            };
            now.call_method1("__sub__", (dt,))
                .and_then(|delta| delta.call_method0("total_seconds"))
                .map(|v| v.into())
        }
        BuiltinFilter::Before(rhs) => {
            let left = match as_datetime(py, value, None)? {
                Some(dt) => dt,
                None => return Ok(false.to_object(py)),
            };
            let right = match as_datetime(py, rhs, None)? {
                Some(dt) => dt,
                None => return Ok(false.to_object(py)),
            };
            Ok(compare_with_fallback(py, &left, &right, "<")?.to_object(py))
        }
        BuiltinFilter::After(rhs) => {
            let left = match as_datetime(py, value, None)? {
                Some(dt) => dt,
                None => return Ok(false.to_object(py)),
            };
            let right = match as_datetime(py, rhs, None)? {
                Some(dt) => dt,
                None => return Ok(false.to_object(py)),
            };
            Ok(compare_with_fallback(py, &left, &right, ">")?.to_object(py))
        }
    }
}

fn apply_builtin_pipeline(
    py: Python<'_>,
    input: PyObject,
    pipeline: &BuiltinFilterPipeline,
) -> PyResult<PyObject> {
    let mut current = input;
    let mut idx = 0usize;

    while idx < pipeline.len() {
        let step = &pipeline[idx];
        if step.map_suffix && current.bind(py).is_instance_of::<PyList>() {
            let list = current.bind(py).downcast::<PyList>()?;
            let mut run_end = idx + 1;
            while run_end < pipeline.len() && pipeline[run_end].map_suffix {
                run_end += 1;
            }

            let mapped = PyList::empty_bound(py);
            for item in list.iter() {
                let mut mapped_item: PyObject = item.into();
                for mapped_step in &pipeline[idx..run_end] {
                    mapped_item = apply_builtin_filter(py, &mapped_item, &mapped_step.filter)?;
                }
                mapped.append(mapped_item)?;
            }
            current = mapped.into();
            idx = run_end;
            continue;
        }

        current = apply_builtin_filter(py, &current, &step.filter)?;
        idx += 1;
    }

    Ok(current)
}

fn compare_values(
    py: Python<'_>,
    left: &PyObject,
    right: &PyObject,
    operator: &str,
) -> PyResult<bool> {
    let left_bound = left.bind(py);
    let right_bound = right.bind(py);

    let op = match operator {
        "==" => CompareOp::Eq,
        "!=" => CompareOp::Ne,
        ">" => CompareOp::Gt,
        "<" => CompareOp::Lt,
        ">=" => CompareOp::Ge,
        "<=" => CompareOp::Le,
        _ => {
            return Err(make_error(
                py,
                "DictWalkOperatorError",
                &format!("Unsupported operator '{operator}'."),
            ));
        }
    };

    left_bound.rich_compare(right_bound, op)?.is_truthy()
}

fn resolve_root_reference_value(
    py: Python<'_>,
    root_data: &PyObject,
    value: &str,
) -> PyResult<PyObject> {
    let root_path = if value == "$$root" {
        ".".to_string()
    } else if let Some(rest) = value.strip_prefix("$$root.") {
        rest.to_string()
    } else if let Some(rest) = value.strip_prefix("$$root|") {
        format!(".|{rest}")
    } else {
        return Err(make_parse_error(
            py,
            value,
            Some(value),
            "Invalid '$$root' value expression. Expected '$$root', '$$root.<path>', or '$$root|$filter'.",
        ));
    };

    let rust_module = py.import_bound("dictwalk._dictwalk_rs")?;
    let backend = rust_module.getattr("dictwalk")?;
    let kwargs = PyDict::new_bound(py);
    kwargs.set_item("strict", true)?;
    backend
        .call_method("get", (root_data.clone_ref(py), root_path), Some(&kwargs))
        .map(|value| value.into())
}

enum PredicateExpr {
    Pipeline(BuiltinFilterPipeline),
    Not(Box<PredicateExpr>),
    And(Box<PredicateExpr>, Box<PredicateExpr>),
    Or(Box<PredicateExpr>, Box<PredicateExpr>),
}

fn tokenize_boolean_filter_expression(expression: &str) -> Vec<String> {
    let mut tokens: Vec<String> = Vec::new();
    let bytes = expression.as_bytes();
    let mut i = 0usize;

    while i < bytes.len() {
        let ch = bytes[i] as char;
        if ch.is_whitespace() {
            i += 1;
            continue;
        }
        if i + 1 < bytes.len() && &expression[i..i + 2] == "&&" {
            tokens.push("&&".to_string());
            i += 2;
            continue;
        }
        if i + 1 < bytes.len() && &expression[i..i + 2] == "||" {
            tokens.push("||".to_string());
            i += 2;
            continue;
        }
        if ch == '(' || ch == ')' || ch == '!' {
            tokens.push(ch.to_string());
            i += 1;
            continue;
        }

        let start = i;
        let mut paren_depth = 0i32;
        while i < bytes.len() {
            let c = bytes[i] as char;
            if c == '(' {
                paren_depth += 1;
                i += 1;
                continue;
            }
            if c == ')' {
                if paren_depth == 0 {
                    break;
                }
                paren_depth -= 1;
                i += 1;
                continue;
            }
            if paren_depth == 0 {
                if i + 1 < bytes.len() && &expression[i..i + 2] == "&&" {
                    break;
                }
                if i + 1 < bytes.len() && &expression[i..i + 2] == "||" {
                    break;
                }
                if c == '!' {
                    break;
                }
            }
            i += 1;
        }
        let operand = expression[start..i].trim();
        if !operand.is_empty() {
            tokens.push(operand.to_string());
        }
    }

    tokens
}

struct PredicateParser<'py> {
    py: Python<'py>,
    tokens: Vec<String>,
    idx: usize,
}

impl PredicateParser<'_> {
    fn parse(mut self) -> Result<PredicateExpr, String> {
        let result = self.parse_or()?;
        if self.idx != self.tokens.len() {
            return Err(format!(
                "Unexpected token '{}' in boolean path filter expression.",
                self.tokens[self.idx]
            ));
        }
        Ok(result)
    }

    fn parse_or(&mut self) -> Result<PredicateExpr, String> {
        let mut left = self.parse_and()?;
        while self.peek() == Some("||") {
            self.consume("||")?;
            let right = self.parse_and()?;
            left = PredicateExpr::Or(Box::new(left), Box::new(right));
        }
        Ok(left)
    }

    fn parse_and(&mut self) -> Result<PredicateExpr, String> {
        let mut left = self.parse_not()?;
        while self.peek() == Some("&&") {
            self.consume("&&")?;
            let right = self.parse_not()?;
            left = PredicateExpr::And(Box::new(left), Box::new(right));
        }
        Ok(left)
    }

    fn parse_not(&mut self) -> Result<PredicateExpr, String> {
        if self.peek() == Some("!") {
            self.consume("!")?;
            let inner = self.parse_not()?;
            return Ok(PredicateExpr::Not(Box::new(inner)));
        }
        self.parse_primary()
    }

    fn parse_primary(&mut self) -> Result<PredicateExpr, String> {
        if self.peek() == Some("(") {
            self.consume("(")?;
            let inner = self.parse_or()?;
            self.consume(")")?;
            return Ok(inner);
        }

        let token = self
            .peek()
            .ok_or("Unexpected end of boolean path filter expression.".to_string())?
            .to_string();
        self.idx += 1;
        let pipeline = compile_builtin_pipeline(self.py, &token, None)
            .ok_or_else(|| format!("Invalid path filter token '{token}' in boolean expression."))?;
        Ok(PredicateExpr::Pipeline(pipeline))
    }

    fn peek(&self) -> Option<&str> {
        if self.idx >= self.tokens.len() {
            None
        } else {
            Some(self.tokens[self.idx].as_str())
        }
    }

    fn consume(&mut self, expected: &str) -> Result<(), String> {
        let token = self.peek();
        if token != Some(expected) {
            return Err(format!(
                "Expected '{expected}' in boolean path filter expression, got '{:?}'.",
                token
            ));
        }
        self.idx += 1;
        Ok(())
    }
}

fn compile_builtin_or_boolean_predicate(
    py: Python<'_>,
    expr: &str,
) -> Result<Option<PredicateExpr>, String> {
    if expr.contains("&&") || expr.contains("||") || expr.contains('!') {
        let parser = PredicateParser {
            py,
            tokens: tokenize_boolean_filter_expression(expr),
            idx: 0,
        };
        return parser.parse().map(Some);
    }

    if let Some(pipeline) = compile_builtin_pipeline(py, expr, None) {
        return Ok(Some(PredicateExpr::Pipeline(pipeline)));
    }

    Ok(None)
}

fn eval_predicate_expr(py: Python<'_>, expr: &PredicateExpr, value: &PyObject) -> PyResult<bool> {
    match expr {
        PredicateExpr::Pipeline(pipeline) => {
            apply_builtin_pipeline(py, value.clone_ref(py), pipeline)?
                .bind(py)
                .is_truthy()
        }
        PredicateExpr::Not(inner) => Ok(!eval_predicate_expr(py, inner, value)?),
        PredicateExpr::And(left, right) => {
            if !eval_predicate_expr(py, left, value)? {
                return Ok(false);
            }
            eval_predicate_expr(py, right, value)
        }
        PredicateExpr::Or(left, right) => {
            if eval_predicate_expr(py, left, value)? {
                return Ok(true);
            }
            eval_predicate_expr(py, right, value)
        }
    }
}

fn resolve_predicate_filter(
    _module: &Bound<'_, PyModule>,
    _registry: &Bound<'_, PyAny>,
    py: Python<'_>,
    expr: &str,
) -> PyResult<Option<PredicateExpr>> {
    compile_builtin_or_boolean_predicate(py, expr)
        .map_err(|message| make_parse_error(py, expr, Some(expr), &message))
}

enum FieldValueResolver {
    CurrentItem,
    CurrentItemBuiltinPipeline(BuiltinFilterPipeline),
    CurrentItemTransform(Option<BuiltinFilterPipeline>),
    PredicateFilter(PredicateExpr),
    Key(String),
}

enum ValueMatcher {
    BuiltinPipeline(BuiltinFilterPipeline),
    PredicateExpr(PredicateExpr),
    Literal(PyObject),
}

struct CompiledFilterMatcher {
    field_resolver: FieldValueResolver,
    value_matcher: ValueMatcher,
    raw_value: String,
}

fn compile_filter_matcher(
    py: Python<'_>,
    module: &Bound<'_, PyModule>,
    registry: &Bound<'_, PyAny>,
    field: &str,
    value: &str,
) -> PyResult<CompiledFilterMatcher> {
    let field_resolver = if field == "." {
        FieldValueResolver::CurrentItem
    } else if let Some(field_transform) = field.strip_prefix(".|") {
        if let Some(pipeline) = compile_builtin_pipeline(py, field_transform, None) {
            FieldValueResolver::CurrentItemBuiltinPipeline(pipeline)
        } else {
            FieldValueResolver::CurrentItemTransform(None)
        }
    } else if let Some(field_path_filter) = resolve_predicate_filter(module, registry, py, field)? {
        FieldValueResolver::PredicateFilter(field_path_filter)
    } else {
        FieldValueResolver::Key(field.to_string())
    };

    let value_matcher = if let Some(pipeline) = compile_builtin_pipeline(py, value, None) {
        ValueMatcher::BuiltinPipeline(pipeline)
    } else if let Some(path_filter) = resolve_predicate_filter(module, registry, py, value)? {
        ValueMatcher::PredicateExpr(path_filter)
    } else {
        ValueMatcher::Literal(parse_literal(py, value))
    };

    Ok(CompiledFilterMatcher {
        field_resolver,
        value_matcher,
        raw_value: value.to_string(),
    })
}

fn resolve_filter_field_value_compiled(
    py: Python<'_>,
    matcher: &CompiledFilterMatcher,
    item: &PyObject,
) -> PyResult<PyObject> {
    match &matcher.field_resolver {
        FieldValueResolver::CurrentItem => Ok(item.clone_ref(py)),
        FieldValueResolver::CurrentItemBuiltinPipeline(pipeline) => {
            apply_builtin_pipeline(py, item.clone_ref(py), pipeline)
        }
        FieldValueResolver::CurrentItemTransform(field_path_filter) => {
            if let Some(path_filter) = field_path_filter.as_ref() {
                apply_builtin_pipeline(py, item.clone_ref(py), path_filter)
            } else {
                Ok(py.None())
            }
        }
        FieldValueResolver::PredicateFilter(path_filter) => {
            Ok(eval_predicate_expr(py, path_filter, item)?.to_object(py))
        }
        FieldValueResolver::Key(field) => {
            let item_bound = item.bind(py);
            if let Ok(item_dict) = item_bound.downcast::<PyDict>() {
                if let Some(value) = item_dict.get_item(field)? {
                    return Ok(value.into());
                }
            }
            Ok(py.None())
        }
    }
}

fn filter_matches_compiled(
    py: Python<'_>,
    operator: &str,
    matcher: &CompiledFilterMatcher,
    item: &PyObject,
    root_data: Option<&PyObject>,
) -> PyResult<bool> {
    let field_value = resolve_filter_field_value_compiled(py, matcher, item)?;

    if let ValueMatcher::BuiltinPipeline(pipeline) = &matcher.value_matcher {
        if operator == "==" || operator == "!=" {
            let predicate_value = apply_builtin_pipeline(py, field_value, pipeline)?;
            let truthy = predicate_value.bind(py).is_truthy()?;
            return Ok(if operator == "==" { truthy } else { !truthy });
        }
        return Err(make_error(
            py,
            "DictWalkOperatorError",
            &format!("Operator '{operator}' is not supported with path filters."),
        ));
    }

    if let ValueMatcher::PredicateExpr(path_filter) = &matcher.value_matcher {
        if operator == "==" {
            return eval_predicate_expr(py, path_filter, &field_value);
        }
        if operator == "!=" {
            return Ok(!eval_predicate_expr(py, path_filter, &field_value)?);
        }
        return Err(make_error(
            py,
            "DictWalkOperatorError",
            &format!("Operator '{operator}' is not supported with path filters."),
        ));
    }

    let expected_value = match &matcher.value_matcher {
        ValueMatcher::Literal(_value)
            if matcher.raw_value.starts_with("$$root") && root_data.is_some() =>
        {
            resolve_root_reference_value(
                py,
                root_data.expect("checked is_some"),
                &matcher.raw_value,
            )?
        }
        ValueMatcher::Literal(value) => value.clone_ref(py),
        _ => py.None(),
    };

    if operator == "==" || operator == "!=" {
        let result = compare_values(py, &field_value, &expected_value, "==")?
            || field_value.bind(py).str()?.to_string_lossy().as_ref() == matcher.raw_value;
        return Ok(if operator == "==" { result } else { !result });
    }

    match compare_values(py, &field_value, &expected_value, operator) {
        Ok(result) => return Ok(result),
        Err(err) => {
            if !err.is_instance_of::<PyTypeError>(py) {
                return Err(err);
            }
        }
    }

    if field_value.bind(py).is_instance_of::<PyString>() {
        let field_value_string = field_value.bind(py).extract::<String>()?;
        let parsed_field_value = parse_literal(py, &field_value_string);
        match compare_values(py, &parsed_field_value, &expected_value, operator) {
            Ok(result) => return Ok(result),
            Err(err) => {
                if !err.is_instance_of::<PyTypeError>(py) {
                    return Err(err);
                }
            }
        }
    }

    let left_str = field_value.bind(py).str()?.to_string_lossy().to_string();
    let left_obj = left_str.to_object(py);
    let right_obj = matcher.raw_value.to_object(py);
    compare_values(py, &left_obj, &right_obj, operator)
}

fn resolve_filter_token(
    py: Python<'_>,
    module: &Bound<'_, PyModule>,
    registry: &Bound<'_, PyAny>,
    current: &PyObject,
    root_data: &PyObject,
    list_key: &str,
    field: &str,
    operator: &str,
    value: &str,
) -> PyResult<PyObject> {
    let matcher = compile_filter_matcher(py, module, registry, field, value)?;
    let source_list_obj = {
        let current_bound = current.bind(py);
        if let Ok(current_dict) = current_bound.downcast::<PyDict>() {
            match current_dict.get_item(list_key)? {
                Some(list_value) => list_value.into(),
                None => PyList::empty_bound(py).into(),
            }
        } else {
            current.clone_ref(py)
        }
    };

    let source_bound = source_list_obj.bind(py);
    let source_list = source_bound.downcast::<PyList>().map_err(|_| {
        PyTypeError::new_err(format!(
            "Expected a list for key '{list_key}', got {}.",
            get_type_name(&source_bound)
        ))
    })?;

    let out = PyList::empty_bound(py);
    for item in source_list.iter() {
        let item_obj: PyObject = item.clone().into();
        if filter_matches_compiled(py, operator, &matcher, &item_obj, Some(root_data))? {
            out.append(item)?;
        }
    }

    Ok(out.into())
}

fn is_soft_resolution_error(py: Python<'_>, err: &PyErr) -> bool {
    if err.is_instance_of::<PyKeyError>(py) || err.is_instance_of::<PyTypeError>(py) {
        return true;
    }

    match py.import_bound("dictwalk.errors") {
        Ok(errors_module) => match errors_module.getattr("DictWalkOperatorError") {
            Ok(operator_error) => err
                .value_bound(py)
                .is_instance(&operator_error)
                .unwrap_or(false),
            Err(_) => false,
        },
        Err(_) => false,
    }
}

fn resolve_token(
    py: Python<'_>,
    module: &Bound<'_, PyModule>,
    registry: &Bound<'_, PyAny>,
    current: &PyObject,
    root_data: &PyObject,
    kind: &TokenKind,
) -> PyResult<PyObject> {
    match kind {
        TokenKind::Get(key) => resolve_get_token(py, current, key),
        TokenKind::Map(key) => resolve_map_token(py, current, key),
        TokenKind::Wildcard => resolve_wildcard_token(py, current),
        TokenKind::DeepWildcard => resolve_deep_wildcard_token(py, current),
        TokenKind::Index { key, index } => resolve_index_token(py, current, key, *index),
        TokenKind::Slice { key, start, end } => resolve_slice_token(py, current, key, *start, *end),
        TokenKind::Filter {
            list_key,
            field,
            operator,
            value,
        } => resolve_filter_token(
            py, module, registry, current, root_data, list_key, field, operator, value,
        ),
        TokenKind::Root => Ok(current.clone_ref(py)),
    }
}

#[derive(Clone, Copy)]
struct WriteOptions {
    create_missing: bool,
    create_filter_match: bool,
    overwrite_incompatible: bool,
}

fn path_uses_root_token(tokens: &[ParsedToken]) -> bool {
    tokens
        .iter()
        .any(|token| matches!(token.kind, TokenKind::Root))
}

fn ensure_path_resolves(
    py: Python<'_>,
    module: &Bound<'_, PyModule>,
    registry: &Bound<'_, PyAny>,
    data: &PyObject,
    path: &str,
    tokens: &[ParsedToken],
    until: usize,
) -> PyResult<()> {
    let mut current = data.clone_ref(py);

    for token in tokens.iter().take(until) {
        if matches!(token.kind, TokenKind::Root) {
            current = data.clone_ref(py);
            continue;
        }

        let resolved = resolve_token(py, module, registry, &current, data, &token.kind);
        match resolved {
            Ok(value) => current = value,
            Err(err) => {
                if is_soft_resolution_error(py, &err) {
                    return Err(make_resolution_error(
                        py,
                        path,
                        Some(&token.raw),
                        &err.to_string(),
                    ));
                }
                return Err(err);
            }
        }
    }

    Ok(())
}

fn is_dict_or_list(bound: &Bound<'_, PyAny>) -> bool {
    bound.is_instance_of::<PyDict>() || bound.is_instance_of::<PyList>()
}

fn new_write_container(py: Python<'_>) -> PyObject {
    PyDict::new_bound(py).into()
}

fn resolve_new_value(
    py: Python<'_>,
    _module: &Bound<'_, PyModule>,
    _registry: &Bound<'_, PyAny>,
    existing_value: Option<PyObject>,
    new_value: &PyObject,
    root_data: &PyObject,
) -> PyResult<PyObject> {
    if let Ok(filter_value) = new_value.bind(py).extract::<String>() {
        if filter_value.starts_with("$$root") {
            let root_path = if filter_value == "$$root" {
                ".".to_string()
            } else if let Some(rest) = filter_value.strip_prefix("$$root.") {
                rest.to_string()
            } else if let Some(rest) = filter_value.strip_prefix("$$root|") {
                format!(".|{rest}")
            } else {
                return Err(make_parse_error(
                    py,
                    &filter_value,
                    Some(&filter_value),
                    "Invalid '$$root' value expression. Expected '$$root', '$$root.<path>', or '$$root|$filter'.",
                ));
            };

            let rust_module = py.import_bound("dictwalk._dictwalk_rs")?;
            let backend = rust_module.getattr("dictwalk")?;
            let kwargs = PyDict::new_bound(py);
            kwargs.set_item("strict", true)?;
            return backend
                .call_method("get", (root_data.clone_ref(py), root_path), Some(&kwargs))
                .map(|value| value.into());
        }

        if !filter_value.starts_with("$$root") {
            if let Some(pipeline) = compile_builtin_pipeline(py, &filter_value, None) {
                let existing = existing_value.unwrap_or_else(|| py.None());
                return apply_builtin_pipeline(py, existing, &pipeline);
            }
        }
    }

    Ok(new_value.clone_ref(py))
}

fn dict_keys(dict: &Bound<'_, PyDict>) -> Vec<PyObject> {
    let mut keys: Vec<PyObject> = Vec::new();
    for (key, _) in dict.iter() {
        keys.push(key.into());
    }
    keys
}

fn coerce_current_to_dict_for_write(
    py: Python<'_>,
    current: PyObject,
    write_options: WriteOptions,
) -> PyObject {
    if current.bind(py).is_instance_of::<PyDict>() {
        return current;
    }
    if !write_options.overwrite_incompatible || !write_options.create_missing {
        return current;
    }
    PyDict::new_bound(py).into()
}

fn compute_slice_indexes(len: usize, start: Option<isize>, end: Option<isize>) -> Vec<usize> {
    let len_isize = len as isize;
    let mut slice_start = start.unwrap_or(0);
    if slice_start < 0 {
        slice_start += len_isize;
    }
    slice_start = slice_start.clamp(0, len_isize);

    let mut slice_end = end.unwrap_or(len_isize);
    if slice_end < 0 {
        slice_end += len_isize;
    }
    slice_end = slice_end.clamp(0, len_isize);

    if slice_start >= slice_end {
        return Vec::new();
    }

    (slice_start as usize..slice_end as usize).collect()
}

fn set_recurse(
    py: Python<'_>,
    module: &Bound<'_, PyModule>,
    registry: &Bound<'_, PyAny>,
    current: PyObject,
    remaining: &[ParsedToken],
    new_value: &PyObject,
    write_options: WriteOptions,
    root_data: &PyObject,
) -> PyResult<PyObject> {
    if remaining.is_empty() {
        return Ok(new_value.clone_ref(py));
    }

    match &remaining[0].kind {
        TokenKind::Get(key) => set_get_token(
            py,
            module,
            registry,
            current,
            remaining,
            key,
            new_value,
            write_options,
            root_data,
        ),
        TokenKind::Map(key) => set_map_token(
            py,
            module,
            registry,
            current,
            remaining,
            key,
            new_value,
            write_options,
            root_data,
        ),
        TokenKind::Wildcard => set_wildcard_token(
            py,
            module,
            registry,
            current,
            remaining,
            new_value,
            write_options,
            root_data,
        ),
        TokenKind::DeepWildcard => set_deep_wildcard_token(
            py,
            module,
            registry,
            current,
            remaining,
            new_value,
            write_options,
            root_data,
        ),
        TokenKind::Index { key, index } => set_index_token(
            py,
            module,
            registry,
            current,
            remaining,
            key,
            *index,
            new_value,
            write_options,
            root_data,
        ),
        TokenKind::Slice { key, start, end } => set_slice_token(
            py,
            module,
            registry,
            current,
            remaining,
            key,
            *start,
            *end,
            new_value,
            write_options,
            root_data,
        ),
        TokenKind::Filter {
            list_key,
            field,
            operator,
            value,
        } => set_filter_token(
            py,
            module,
            registry,
            current,
            remaining,
            list_key,
            field,
            operator,
            value,
            new_value,
            write_options,
            root_data,
        ),
        TokenKind::Root => Ok(current),
    }
}

fn set_get_token(
    py: Python<'_>,
    module: &Bound<'_, PyModule>,
    registry: &Bound<'_, PyAny>,
    current: PyObject,
    remaining: &[ParsedToken],
    key: &str,
    new_value: &PyObject,
    write_options: WriteOptions,
    root_data: &PyObject,
) -> PyResult<PyObject> {
    let next_kind = remaining.get(1).map(|token| &token.kind);
    let current = coerce_current_to_dict_for_write(py, current, write_options);
    if !current.bind(py).is_instance_of::<PyDict>() {
        return Ok(current);
    }

    let dict = current.bind(py).downcast::<PyDict>()?;
    if remaining.len() == 1 {
        let existing = dict.get_item(key)?.map(|value| value.into());
        if existing.is_none() && !write_options.create_missing {
            return Ok(current);
        }
        let resolved = resolve_new_value(py, module, registry, existing, new_value, root_data)?;
        dict.set_item(key, resolved)?;
        return Ok(current);
    }

    let child_opt = dict.get_item(key)?.map(|value| value.into());
    let had_child = child_opt.is_some();
    let mut child = match child_opt {
        Some(value) => value,
        None => {
            if !write_options.create_missing {
                return Ok(current);
            }
            new_write_container(py)
        }
    };

    if had_child && next_kind.is_some() && !is_dict_or_list(&child.bind(py)) {
        if !write_options.overwrite_incompatible {
            return Ok(current);
        }
        child = new_write_container(py);
    }

    let updated = set_recurse(
        py,
        module,
        registry,
        child,
        &remaining[1..],
        new_value,
        write_options,
        root_data,
    )?;
    dict.set_item(key, updated)?;
    Ok(current)
}

fn set_map_token(
    py: Python<'_>,
    module: &Bound<'_, PyModule>,
    registry: &Bound<'_, PyAny>,
    current: PyObject,
    remaining: &[ParsedToken],
    key: &str,
    new_value: &PyObject,
    write_options: WriteOptions,
    root_data: &PyObject,
) -> PyResult<PyObject> {
    let next_kind = remaining.get(1).map(|token| &token.kind);
    let current = coerce_current_to_dict_for_write(py, current, write_options);
    if !current.bind(py).is_instance_of::<PyDict>() {
        return Ok(current);
    }

    let dict = current.bind(py).downcast::<PyDict>()?;
    let list_obj: PyObject = match dict.get_item(key)? {
        Some(value) => {
            if value.is_instance_of::<PyList>() {
                value.into()
            } else {
                if !write_options.overwrite_incompatible {
                    return Ok(current);
                }
                PyList::empty_bound(py).into()
            }
        }
        None => {
            if !write_options.create_missing {
                return Ok(current);
            }
            PyList::empty_bound(py).into()
        }
    };
    let list = list_obj.bind(py).downcast::<PyList>()?;

    if remaining.len() == 1 {
        for idx in 0..list.len() {
            let existing: PyObject = list.get_item(idx)?.into();
            let resolved =
                resolve_new_value(py, module, registry, Some(existing), new_value, root_data)?;
            list.set_item(idx, resolved)?;
        }
        dict.set_item(key, list_obj)?;
        return Ok(current);
    }

    if list.is_empty() {
        if !write_options.create_missing {
            return Ok(current);
        }
        list.append(new_write_container(py))?;
    }

    for idx in 0..list.len() {
        let mut item: PyObject = list.get_item(idx)?.into();
        if next_kind.is_some() && !is_dict_or_list(&item.bind(py)) {
            if !write_options.overwrite_incompatible {
                continue;
            }
            item = new_write_container(py);
        }

        let updated = set_recurse(
            py,
            module,
            registry,
            item,
            &remaining[1..],
            new_value,
            write_options,
            root_data,
        )?;
        list.set_item(idx, updated)?;
    }

    dict.set_item(key, list_obj)?;
    Ok(current)
}

fn set_wildcard_token(
    py: Python<'_>,
    module: &Bound<'_, PyModule>,
    registry: &Bound<'_, PyAny>,
    current: PyObject,
    remaining: &[ParsedToken],
    new_value: &PyObject,
    write_options: WriteOptions,
    root_data: &PyObject,
) -> PyResult<PyObject> {
    if current.bind(py).is_instance_of::<PyDict>() {
        let dict = current.bind(py).downcast::<PyDict>()?;
        let keys = dict_keys(dict);

        for key in keys {
            let current_child = dict
                .get_item(key.bind(py))?
                .map(|value| value.into())
                .unwrap_or_else(|| py.None());
            let updated = if remaining.len() == 1 {
                resolve_new_value(
                    py,
                    module,
                    registry,
                    Some(current_child),
                    new_value,
                    root_data,
                )?
            } else {
                set_recurse(
                    py,
                    module,
                    registry,
                    current_child,
                    &remaining[1..],
                    new_value,
                    write_options,
                    root_data,
                )?
            };
            dict.set_item(key.bind(py), updated)?;
        }
        return Ok(current);
    }

    if current.bind(py).is_instance_of::<PyList>() {
        let list = current.bind(py).downcast::<PyList>()?;
        for idx in 0..list.len() {
            let current_child: PyObject = list.get_item(idx)?.into();
            let updated = if remaining.len() == 1 {
                resolve_new_value(
                    py,
                    module,
                    registry,
                    Some(current_child),
                    new_value,
                    root_data,
                )?
            } else {
                set_recurse(
                    py,
                    module,
                    registry,
                    current_child,
                    &remaining[1..],
                    new_value,
                    write_options,
                    root_data,
                )?
            };
            list.set_item(idx, updated)?;
        }
    }

    Ok(current)
}

fn deep_set_walk(
    py: Python<'_>,
    module: &Bound<'_, PyModule>,
    registry: &Bound<'_, PyAny>,
    node: PyObject,
    remaining: &[ParsedToken],
    new_value: &PyObject,
    write_options: WriteOptions,
    root_data: &PyObject,
) -> PyResult<()> {
    if node.bind(py).is_instance_of::<PyDict>() {
        let dict = node.bind(py).downcast::<PyDict>()?;
        let keys = dict_keys(dict);
        for key in keys {
            let child = match dict.get_item(key.bind(py))? {
                Some(value) => value.into(),
                None => continue,
            };

            if remaining.len() > 1 {
                let updated = set_recurse(
                    py,
                    module,
                    registry,
                    child,
                    &remaining[1..],
                    new_value,
                    write_options,
                    root_data,
                )?;
                dict.set_item(key.bind(py), updated)?;
            }

            if let Some(next_child) = dict.get_item(key.bind(py))? {
                if is_dict_or_list(&next_child) {
                    deep_set_walk(
                        py,
                        module,
                        registry,
                        next_child.into(),
                        remaining,
                        new_value,
                        write_options,
                        root_data,
                    )?;
                }
            }
        }
        return Ok(());
    }

    if node.bind(py).is_instance_of::<PyList>() {
        let list = node.bind(py).downcast::<PyList>()?;
        for idx in 0..list.len() {
            let child: PyObject = list.get_item(idx)?.into();
            if remaining.len() > 1 {
                let updated = set_recurse(
                    py,
                    module,
                    registry,
                    child,
                    &remaining[1..],
                    new_value,
                    write_options,
                    root_data,
                )?;
                list.set_item(idx, updated)?;
            }

            let next_child = list.get_item(idx)?;
            if is_dict_or_list(&next_child) {
                deep_set_walk(
                    py,
                    module,
                    registry,
                    next_child.into(),
                    remaining,
                    new_value,
                    write_options,
                    root_data,
                )?;
            }
        }
    }

    Ok(())
}

fn set_deep_wildcard_token(
    py: Python<'_>,
    module: &Bound<'_, PyModule>,
    registry: &Bound<'_, PyAny>,
    current: PyObject,
    remaining: &[ParsedToken],
    new_value: &PyObject,
    write_options: WriteOptions,
    root_data: &PyObject,
) -> PyResult<PyObject> {
    if !is_dict_or_list(&current.bind(py)) {
        return Ok(current);
    }

    let apply_options = WriteOptions {
        create_missing: false,
        create_filter_match: write_options.create_filter_match,
        overwrite_incompatible: write_options.overwrite_incompatible,
    };
    deep_set_walk(
        py,
        module,
        registry,
        current.clone_ref(py),
        remaining,
        new_value,
        apply_options,
        root_data,
    )?;
    Ok(current)
}

fn set_index_token(
    py: Python<'_>,
    module: &Bound<'_, PyModule>,
    registry: &Bound<'_, PyAny>,
    current: PyObject,
    remaining: &[ParsedToken],
    key: &str,
    index: isize,
    new_value: &PyObject,
    write_options: WriteOptions,
    root_data: &PyObject,
) -> PyResult<PyObject> {
    let next_kind = remaining.get(1).map(|token| &token.kind);
    let current = coerce_current_to_dict_for_write(py, current, write_options);
    if !current.bind(py).is_instance_of::<PyDict>() {
        return Ok(current);
    }

    let dict = current.bind(py).downcast::<PyDict>()?;
    let list_obj: PyObject = match dict.get_item(key)? {
        Some(value) => {
            if value.is_instance_of::<PyList>() {
                value.into()
            } else {
                if !write_options.overwrite_incompatible {
                    return Ok(current);
                }
                PyList::empty_bound(py).into()
            }
        }
        None => {
            if !write_options.create_missing {
                return Ok(current);
            }
            PyList::empty_bound(py).into()
        }
    };
    let list = list_obj.bind(py).downcast::<PyList>()?;

    let idx = index;
    if idx < 0 {
        if idx < -(list.len() as isize) {
            dict.set_item(key, list_obj)?;
            return Ok(current);
        }
    } else {
        if !write_options.create_missing {
            dict.set_item(key, list_obj)?;
            return Ok(current);
        }
        while list.len() <= idx as usize {
            let fill_value = if next_kind.is_some() {
                new_write_container(py)
            } else {
                py.None()
            };
            list.append(fill_value)?;
        }
    }

    let target_index = if idx < 0 {
        (list.len() as isize + idx) as usize
    } else {
        idx as usize
    };

    if remaining.len() == 1 {
        let existing = list.get_item(target_index)?.into();
        let resolved =
            resolve_new_value(py, module, registry, Some(existing), new_value, root_data)?;
        list.set_item(target_index, resolved)?;
        dict.set_item(key, list_obj)?;
        return Ok(current);
    }

    let mut item: PyObject = list.get_item(target_index)?.into();
    if next_kind.is_some() && !is_dict_or_list(&item.bind(py)) {
        if !write_options.overwrite_incompatible {
            dict.set_item(key, list_obj)?;
            return Ok(current);
        }
        item = new_write_container(py);
    }

    let updated = set_recurse(
        py,
        module,
        registry,
        item,
        &remaining[1..],
        new_value,
        write_options,
        root_data,
    )?;
    list.set_item(target_index, updated)?;
    dict.set_item(key, list_obj)?;
    Ok(current)
}

fn set_slice_token(
    py: Python<'_>,
    module: &Bound<'_, PyModule>,
    registry: &Bound<'_, PyAny>,
    current: PyObject,
    remaining: &[ParsedToken],
    key: &str,
    start: Option<isize>,
    end: Option<isize>,
    new_value: &PyObject,
    write_options: WriteOptions,
    root_data: &PyObject,
) -> PyResult<PyObject> {
    let next_kind = remaining.get(1).map(|token| &token.kind);
    let current = coerce_current_to_dict_for_write(py, current, write_options);
    if !current.bind(py).is_instance_of::<PyDict>() {
        return Ok(current);
    }

    let dict = current.bind(py).downcast::<PyDict>()?;
    let list_obj: PyObject = match dict.get_item(key)? {
        Some(value) => {
            if value.is_instance_of::<PyList>() {
                value.into()
            } else {
                if !write_options.overwrite_incompatible {
                    return Ok(current);
                }
                PyList::empty_bound(py).into()
            }
        }
        None => {
            if !write_options.create_missing {
                return Ok(current);
            }
            PyList::empty_bound(py).into()
        }
    };
    let list = list_obj.bind(py).downcast::<PyList>()?;
    let indexes = compute_slice_indexes(list.len(), start, end);

    if remaining.len() == 1 {
        for idx in indexes {
            let existing = list.get_item(idx)?.into();
            let resolved =
                resolve_new_value(py, module, registry, Some(existing), new_value, root_data)?;
            list.set_item(idx, resolved)?;
        }
        dict.set_item(key, list_obj)?;
        return Ok(current);
    }

    for idx in indexes {
        let mut item: PyObject = list.get_item(idx)?.into();
        if next_kind.is_some() && !is_dict_or_list(&item.bind(py)) {
            if !write_options.overwrite_incompatible {
                continue;
            }
            item = new_write_container(py);
        }
        let updated = set_recurse(
            py,
            module,
            registry,
            item,
            &remaining[1..],
            new_value,
            write_options,
            root_data,
        )?;
        list.set_item(idx, updated)?;
    }

    dict.set_item(key, list_obj)?;
    Ok(current)
}

fn set_filter_token(
    py: Python<'_>,
    module: &Bound<'_, PyModule>,
    registry: &Bound<'_, PyAny>,
    current: PyObject,
    remaining: &[ParsedToken],
    list_key: &str,
    field: &str,
    operator: &str,
    value: &str,
    new_value: &PyObject,
    write_options: WriteOptions,
    root_data: &PyObject,
) -> PyResult<PyObject> {
    if !current.bind(py).is_instance_of::<PyDict>() {
        return Ok(current);
    }
    let dict = current.bind(py).downcast::<PyDict>()?;

    let list_obj: PyObject = match dict.get_item(list_key)? {
        Some(value_obj) => {
            if value_obj.is_instance_of::<PyList>() {
                value_obj.into()
            } else {
                if !write_options.overwrite_incompatible {
                    return Ok(current);
                }
                PyList::empty_bound(py).into()
            }
        }
        None => {
            if !write_options.create_missing {
                return Ok(current);
            }
            PyList::empty_bound(py).into()
        }
    };
    let list = list_obj.bind(py).downcast::<PyList>()?;
    let matcher = compile_filter_matcher(py, module, registry, field, value)?;

    let mut matches: Vec<bool> = Vec::with_capacity(list.len());
    for idx in 0..list.len() {
        let item: PyObject = list.get_item(idx)?.into();
        matches.push(filter_matches_compiled(
            py,
            operator,
            &matcher,
            &item,
            Some(root_data),
        )?);
    }

    if !matches.iter().any(|matched| *matched) {
        let field_uses_item_root = matches!(
            matcher.field_resolver,
            FieldValueResolver::CurrentItem
                | FieldValueResolver::CurrentItemBuiltinPipeline(_)
                | FieldValueResolver::CurrentItemTransform(_)
        );
        let field_path_filter_present = matches!(
            matcher.field_resolver,
            FieldValueResolver::CurrentItemBuiltinPipeline(_)
                | FieldValueResolver::CurrentItemTransform(_)
                | FieldValueResolver::PredicateFilter(_)
        );
        let value_path_filter_present = matches!(
            matcher.value_matcher,
            ValueMatcher::BuiltinPipeline(_) | ValueMatcher::PredicateExpr(_)
        );

        if !field_uses_item_root
            && !field_path_filter_present
            && !value_path_filter_present
            && operator == "=="
            && write_options.create_missing
            && write_options.create_filter_match
        {
            let new_item = PyDict::new_bound(py);
            new_item.set_item(field, value)?;
            list.append(new_item.clone())?;
            matches.push(true);
        }
    }

    if remaining.len() == 1 {
        for idx in 0..list.len() {
            if !matches.get(idx).copied().unwrap_or(false) {
                continue;
            }
            let existing = list.get_item(idx)?.into();
            let resolved =
                resolve_new_value(py, module, registry, Some(existing), new_value, root_data)?;
            list.set_item(idx, resolved)?;
        }
        dict.set_item(list_key, list_obj)?;
        return Ok(current);
    }

    for idx in 0..list.len() {
        if !matches.get(idx).copied().unwrap_or(false) {
            continue;
        }
        let item: PyObject = list.get_item(idx)?.into();
        let updated = set_recurse(
            py,
            module,
            registry,
            item,
            &remaining[1..],
            new_value,
            write_options,
            root_data,
        )?;
        list.set_item(idx, updated)?;
    }

    dict.set_item(list_key, list_obj)?;
    Ok(current)
}

fn unset_recurse(
    py: Python<'_>,
    module: &Bound<'_, PyModule>,
    registry: &Bound<'_, PyAny>,
    current: PyObject,
    remaining: &[ParsedToken],
) -> PyResult<PyObject> {
    if remaining.is_empty() {
        return Ok(current);
    }

    match &remaining[0].kind {
        TokenKind::Get(key) => unset_get_token(py, module, registry, current, remaining, key),
        TokenKind::Map(key) => unset_map_token(py, module, registry, current, remaining, key),
        TokenKind::Wildcard => unset_wildcard_token(py, module, registry, current, remaining),
        TokenKind::DeepWildcard => {
            unset_deep_wildcard_token(py, module, registry, current, remaining)
        }
        TokenKind::Index { key, index } => {
            unset_index_token(py, module, registry, current, remaining, key, *index)
        }
        TokenKind::Slice { key, start, end } => {
            unset_slice_token(py, module, registry, current, remaining, key, *start, *end)
        }
        TokenKind::Filter {
            list_key,
            field,
            operator,
            value,
        } => unset_filter_token(
            py, module, registry, current, remaining, list_key, field, operator, value,
        ),
        TokenKind::Root => Ok(current),
    }
}

fn unset_get_token(
    py: Python<'_>,
    module: &Bound<'_, PyModule>,
    registry: &Bound<'_, PyAny>,
    current: PyObject,
    remaining: &[ParsedToken],
    key: &str,
) -> PyResult<PyObject> {
    if !current.bind(py).is_instance_of::<PyDict>() {
        return Ok(current);
    }

    let dict = current.bind(py).downcast::<PyDict>()?;
    if remaining.len() == 1 {
        if dict.contains(key)? {
            dict.del_item(key)?;
        }
        return Ok(current);
    }

    let child = match dict.get_item(key)? {
        Some(value) => value.into(),
        None => return Ok(current),
    };
    let updated = unset_recurse(py, module, registry, child, &remaining[1..])?;
    dict.set_item(key, updated)?;
    Ok(current)
}

fn unset_map_token(
    py: Python<'_>,
    module: &Bound<'_, PyModule>,
    registry: &Bound<'_, PyAny>,
    current: PyObject,
    remaining: &[ParsedToken],
    key: &str,
) -> PyResult<PyObject> {
    if !current.bind(py).is_instance_of::<PyDict>() {
        return Ok(current);
    }

    let dict = current.bind(py).downcast::<PyDict>()?;
    let list_obj: PyObject = match dict.get_item(key)? {
        Some(value) => {
            if value.is_instance_of::<PyList>() {
                value.into()
            } else {
                return Ok(current);
            }
        }
        None => return Ok(current),
    };
    let list = list_obj.bind(py).downcast::<PyList>()?;

    if remaining.len() == 1 {
        dict.set_item(key, PyList::empty_bound(py))?;
        return Ok(current);
    }

    for idx in 0..list.len() {
        let item: PyObject = list.get_item(idx)?.into();
        let updated = unset_recurse(py, module, registry, item, &remaining[1..])?;
        list.set_item(idx, updated)?;
    }
    dict.set_item(key, list_obj)?;
    Ok(current)
}

fn unset_wildcard_token(
    py: Python<'_>,
    module: &Bound<'_, PyModule>,
    registry: &Bound<'_, PyAny>,
    current: PyObject,
    remaining: &[ParsedToken],
) -> PyResult<PyObject> {
    if current.bind(py).is_instance_of::<PyDict>() {
        let dict = current.bind(py).downcast::<PyDict>()?;
        if remaining.len() == 1 {
            dict.clear();
            return Ok(current);
        }

        let keys = dict_keys(dict);
        for key in keys {
            let child = match dict.get_item(key.bind(py))? {
                Some(value) => value.into(),
                None => continue,
            };
            let updated = unset_recurse(py, module, registry, child, &remaining[1..])?;
            dict.set_item(key.bind(py), updated)?;
        }
        return Ok(current);
    }

    if current.bind(py).is_instance_of::<PyList>() {
        let list = current.bind(py).downcast::<PyList>()?;
        if remaining.len() == 1 {
            list.call_method0("clear")?;
            return Ok(current);
        }

        for idx in 0..list.len() {
            let child: PyObject = list.get_item(idx)?.into();
            let updated = unset_recurse(py, module, registry, child, &remaining[1..])?;
            list.set_item(idx, updated)?;
        }
    }

    Ok(current)
}

fn deep_unset_walk(
    py: Python<'_>,
    module: &Bound<'_, PyModule>,
    registry: &Bound<'_, PyAny>,
    node: PyObject,
    remaining: &[ParsedToken],
) -> PyResult<()> {
    if node.bind(py).is_instance_of::<PyDict>() {
        let dict = node.bind(py).downcast::<PyDict>()?;
        let keys = dict_keys(dict);
        for key in keys {
            let child = match dict.get_item(key.bind(py))? {
                Some(value) => value.into(),
                None => continue,
            };

            if remaining.len() > 1 {
                let updated = unset_recurse(py, module, registry, child, &remaining[1..])?;
                dict.set_item(key.bind(py), updated)?;
            }

            if let Some(next_child) = dict.get_item(key.bind(py))? {
                if is_dict_or_list(&next_child) {
                    deep_unset_walk(py, module, registry, next_child.into(), remaining)?;
                }
            }
        }
        return Ok(());
    }

    if node.bind(py).is_instance_of::<PyList>() {
        let list = node.bind(py).downcast::<PyList>()?;
        for idx in 0..list.len() {
            let child: PyObject = list.get_item(idx)?.into();
            if remaining.len() > 1 {
                let updated = unset_recurse(py, module, registry, child, &remaining[1..])?;
                list.set_item(idx, updated)?;
            }

            let next_child = list.get_item(idx)?;
            if is_dict_or_list(&next_child) {
                deep_unset_walk(py, module, registry, next_child.into(), remaining)?;
            }
        }
    }

    Ok(())
}

fn unset_deep_wildcard_token(
    py: Python<'_>,
    module: &Bound<'_, PyModule>,
    registry: &Bound<'_, PyAny>,
    current: PyObject,
    remaining: &[ParsedToken],
) -> PyResult<PyObject> {
    if !is_dict_or_list(&current.bind(py)) {
        return Ok(current);
    }

    deep_unset_walk(py, module, registry, current.clone_ref(py), remaining)?;
    Ok(current)
}

fn unset_index_token(
    py: Python<'_>,
    module: &Bound<'_, PyModule>,
    registry: &Bound<'_, PyAny>,
    current: PyObject,
    remaining: &[ParsedToken],
    key: &str,
    index: isize,
) -> PyResult<PyObject> {
    if !current.bind(py).is_instance_of::<PyDict>() {
        return Ok(current);
    }

    let dict = current.bind(py).downcast::<PyDict>()?;
    let list_obj: PyObject = match dict.get_item(key)? {
        Some(value) => {
            if value.is_instance_of::<PyList>() {
                value.into()
            } else {
                return Ok(current);
            }
        }
        None => return Ok(current),
    };
    let list = list_obj.bind(py).downcast::<PyList>()?;
    let in_bounds = index >= -(list.len() as isize) && index < list.len() as isize;

    if remaining.len() == 1 {
        if in_bounds {
            list.call_method1("pop", (index,))?;
        }
        dict.set_item(key, list_obj)?;
        return Ok(current);
    }

    if in_bounds {
        let target_index = if index < 0 {
            (list.len() as isize + index) as usize
        } else {
            index as usize
        };
        let child: PyObject = list.get_item(target_index)?.into();
        let updated = unset_recurse(py, module, registry, child, &remaining[1..])?;
        list.set_item(target_index, updated)?;
    }

    dict.set_item(key, list_obj)?;
    Ok(current)
}

fn unset_slice_token(
    py: Python<'_>,
    module: &Bound<'_, PyModule>,
    registry: &Bound<'_, PyAny>,
    current: PyObject,
    remaining: &[ParsedToken],
    key: &str,
    start: Option<isize>,
    end: Option<isize>,
) -> PyResult<PyObject> {
    if !current.bind(py).is_instance_of::<PyDict>() {
        return Ok(current);
    }

    let dict = current.bind(py).downcast::<PyDict>()?;
    let list_obj: PyObject = match dict.get_item(key)? {
        Some(value) => {
            if value.is_instance_of::<PyList>() {
                value.into()
            } else {
                return Ok(current);
            }
        }
        None => return Ok(current),
    };
    let list = list_obj.bind(py).downcast::<PyList>()?;
    let indexes = compute_slice_indexes(list.len(), start, end);

    if remaining.len() == 1 {
        for idx in indexes.iter().rev() {
            list.call_method1("pop", (*idx as isize,))?;
        }
        dict.set_item(key, list_obj)?;
        return Ok(current);
    }

    for idx in indexes {
        let child: PyObject = list.get_item(idx)?.into();
        let updated = unset_recurse(py, module, registry, child, &remaining[1..])?;
        list.set_item(idx, updated)?;
    }

    dict.set_item(key, list_obj)?;
    Ok(current)
}

fn unset_filter_token(
    py: Python<'_>,
    module: &Bound<'_, PyModule>,
    registry: &Bound<'_, PyAny>,
    current: PyObject,
    remaining: &[ParsedToken],
    list_key: &str,
    field: &str,
    operator: &str,
    value: &str,
) -> PyResult<PyObject> {
    if !current.bind(py).is_instance_of::<PyDict>() {
        return Ok(current);
    }

    let dict = current.bind(py).downcast::<PyDict>()?;
    let list_obj: PyObject = match dict.get_item(list_key)? {
        Some(value_obj) => {
            if value_obj.is_instance_of::<PyList>() {
                value_obj.into()
            } else {
                return Ok(current);
            }
        }
        None => return Ok(current),
    };
    let list = list_obj.bind(py).downcast::<PyList>()?;
    let matcher = compile_filter_matcher(py, module, registry, field, value)?;

    if remaining.len() == 1 {
        let filtered = PyList::empty_bound(py);
        for idx in 0..list.len() {
            let item = list.get_item(idx)?;
            let item_obj: PyObject = item.clone().into();
            if !filter_matches_compiled(py, operator, &matcher, &item_obj, None)? {
                filtered.append(item)?;
            }
        }
        dict.set_item(list_key, filtered)?;
        return Ok(current);
    }

    for idx in 0..list.len() {
        let child: PyObject = list.get_item(idx)?.into();
        if !filter_matches_compiled(py, operator, &matcher, &child, None)? {
            continue;
        }
        let updated = unset_recurse(py, module, registry, child, &remaining[1..])?;
        list.set_item(idx, updated)?;
    }

    dict.set_item(list_key, list_obj)?;
    Ok(current)
}

#[pyclass(name = "DictWalk")]
#[derive(Default)]
struct RustDictWalk;

#[allow(clippy::useless_conversion)]
#[pymethods]
impl RustDictWalk {
    #[new]
    fn new() -> Self {
        Self
    }

    #[pyo3(signature = (data, path, default=None, *, strict=false))]
    fn get(
        &self,
        py: Python<'_>,
        data: PyObject,
        path: &str,
        default: Option<PyObject>,
        strict: bool,
    ) -> PyResult<PyObject> {
        let module = py.import_bound("dictwalk.dictwalk")?;
        let registry = load_registry(py)?;
        let (base_path, output_transform) = split_path_and_transform(path);

        if base_path == "." {
            let mut current = data.clone_ref(py);
            if let Some(transform) = output_transform {
                current =
                    apply_output_transform(py, &module, &registry, &current, &transform, &data)?;
            }
            return Ok(current);
        }

        let tokens = parse_path(py, &module, &registry, &base_path)?;
        let mut current = data.clone_ref(py);

        for token in tokens {
            if matches!(token.kind, TokenKind::Root) {
                current = data.clone_ref(py);
                continue;
            }

            let resolved = resolve_token(py, &module, &registry, &current, &data, &token.kind);

            match resolved {
                Ok(value) => current = value,
                Err(err) => {
                    if is_soft_resolution_error(py, &err) {
                        if strict {
                            return Err(make_resolution_error(
                                py,
                                &base_path,
                                Some(&token.raw),
                                &err.to_string(),
                            ));
                        }
                        return Ok(default.unwrap_or_else(|| py.None()));
                    }
                    return Err(err);
                }
            }
        }

        if let Some(transform) = output_transform {
            current = apply_output_transform(py, &module, &registry, &current, &transform, &data)?;
        }

        Ok(current)
    }

    #[pyo3(signature = (data, path, *, strict=false))]
    fn exists(
        &self,
        py: Python<'_>,
        data: PyObject,
        path: &str,
        strict: bool,
    ) -> PyResult<PyObject> {
        let module = py.import_bound("dictwalk.dictwalk")?;
        let registry = load_registry(py)?;
        let tokens = parse_path(py, &module, &registry, path)?;
        let mut current = data.clone_ref(py);

        for token in tokens {
            if matches!(token.kind, TokenKind::Root) {
                current = data.clone_ref(py);
                continue;
            }

            let resolved = resolve_token(py, &module, &registry, &current, &data, &token.kind);

            match resolved {
                Ok(value) => current = value,
                Err(err) => {
                    if is_soft_resolution_error(py, &err) {
                        if strict {
                            return Err(make_resolution_error(
                                py,
                                path,
                                Some(&token.raw),
                                &err.to_string(),
                            ));
                        }
                        return Ok(false.to_object(py));
                    }
                    return Err(err);
                }
            }
        }

        Ok(true.to_object(py))
    }

    #[pyo3(signature = (data, path, value, *, strict=false, create_missing=true, create_filter_match=true, overwrite_incompatible=true))]
    fn set(
        &self,
        py: Python<'_>,
        data: PyObject,
        path: &str,
        value: PyObject,
        strict: bool,
        create_missing: bool,
        create_filter_match: bool,
        overwrite_incompatible: bool,
    ) -> PyResult<PyObject> {
        let module = py.import_bound("dictwalk.dictwalk")?;
        let registry = load_registry(py)?;
        let tokens = parse_path(py, &module, &registry, path)?;

        if path_uses_root_token(&tokens) {
            return Err(make_parse_error(
                py,
                path,
                Some("$$root"),
                "The '$$root' token is only supported in read paths.",
            ));
        }

        if strict && !tokens.is_empty() {
            ensure_path_resolves(
                py,
                &module,
                &registry,
                &data,
                path,
                &tokens,
                tokens.len() - 1,
            )?;
        }

        let write_options = WriteOptions {
            create_missing,
            create_filter_match,
            overwrite_incompatible,
        };
        let root_data = data.clone_ref(py);
        let _ = set_recurse(
            py,
            &module,
            &registry,
            data.clone_ref(py),
            &tokens,
            &value,
            write_options,
            &root_data,
        )?;

        Ok(data)
    }

    #[pyo3(signature = (data, path, *, strict=false))]
    fn unset(
        &self,
        py: Python<'_>,
        data: PyObject,
        path: &str,
        strict: bool,
    ) -> PyResult<PyObject> {
        let module = py.import_bound("dictwalk.dictwalk")?;
        let registry = load_registry(py)?;
        let tokens = parse_path(py, &module, &registry, path)?;

        if path_uses_root_token(&tokens) {
            return Err(make_parse_error(
                py,
                path,
                Some("$$root"),
                "The '$$root' token is only supported in read paths.",
            ));
        }

        if strict && !tokens.is_empty() {
            ensure_path_resolves(py, &module, &registry, &data, path, &tokens, tokens.len())?;
        }

        let _ = unset_recurse(py, &module, &registry, data.clone_ref(py), &tokens)?;
        Ok(data)
    }

    fn run_filter_function(
        &self,
        py: Python<'_>,
        path_filter: PyObject,
        value: PyObject,
    ) -> PyResult<PyObject> {
        if let Ok(filter_expr) = path_filter.bind(py).extract::<String>() {
            if let Some(pipeline) = compile_builtin_pipeline(py, &filter_expr, None) {
                return apply_builtin_pipeline(py, value, &pipeline);
            }
        }
        let filter_display = path_filter.bind(py).repr()?.to_string_lossy().to_string();
        Err(make_parse_error(
            py,
            &filter_display,
            None,
            "Invalid path filter expression. Expected a '$name' / '$name(...)' built-in filter string.",
        ))
    }

    fn register_path_filter(
        &self,
        py: Python<'_>,
        _name: &str,
        _path_filter: PyObject,
    ) -> PyResult<()> {
        Err(make_error(
            py,
            "DictWalkError",
            "Custom path filters are currently unsupported in the Rust backend.",
        ))
    }

    fn get_path_filter(&self, py: Python<'_>, _name: &str) -> PyResult<PyObject> {
        Err(make_error(
            py,
            "DictWalkError",
            "Custom path filters are currently unsupported in the Rust backend.",
        ))
    }
}

#[pyfunction]
fn backend_name() -> &'static str {
    "rust"
}

#[pymodule]
fn _dictwalk_rs(py: Python<'_>, module: &Bound<'_, PyModule>) -> PyResult<()> {
    module.add_class::<RustDictWalk>()?;
    module.add_function(wrap_pyfunction!(backend_name, module)?)?;
    let dictwalk = Py::new(py, RustDictWalk::new())?;
    module.add("dictwalk", dictwalk)?;
    Ok(())
}
