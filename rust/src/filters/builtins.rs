//! Built-in transform definitions, pipeline compilation, and value conversion.

use crate::*;

pub(crate) enum BuiltinFilter {
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
    IDiv(PyObject),
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
    Reverse,
    Chunk(PyObject),
    Flatten,
    FlattenDeep,
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
    RegexReplace(PyObject, PyObject),
    Split(Option<PyObject>),
    Join(PyObject),
    Startswith(PyObject),
    Endswith(PyObject),
    Matches(PyObject),
    Keys,
    Values,
    Items,
    SortBy(PyObject, Option<PyObject>),
    UniqueBy(PyObject),
    IndexBy(PyObject),
    GroupBy(PyObject),
    Const(PyObject),
    Default(PyObject),
    Coalesce(Vec<PyObject>),
    Bool,
    TypeIs(PyObject),
    IsEmpty,
    NonEmpty,
    Compact,
    FromJson,
    ToJson,
    ToDatetime(Option<PyObject>),
    Strftime(PyObject),
    Timestamp,
    AgeSeconds,
    Before(PyObject),
    After(PyObject),
}

pub(crate) struct BuiltinFilterStep {
    filter: BuiltinFilter,
    map_suffix: bool,
}

pub(crate) type BuiltinFilterPipeline = Vec<BuiltinFilterStep>;

pub(crate) fn parse_literal(py: Python<'_>, value: &str) -> PyObject {
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

pub(crate) fn split_filter_args(args_string: &str) -> Option<Vec<String>> {
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

pub(crate) fn parse_filter_args(
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

pub(crate) fn compile_builtin_filter(
    py: Python<'_>,
    name: &str,
    args: &[PyObject],
) -> Option<BuiltinFilter> {
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
        ("reverse", 0) => Some(BuiltinFilter::Reverse),
        ("chunk", 1) => Some(BuiltinFilter::Chunk(args[0].clone_ref(py))),
        ("flatten", 0) => Some(BuiltinFilter::Flatten),
        ("flatten_deep", 0) => Some(BuiltinFilter::FlattenDeep),
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
        ("regex_replace", 2) => Some(BuiltinFilter::RegexReplace(
            args[0].clone_ref(py),
            args[1].clone_ref(py),
        )),
        ("split", 0) => Some(BuiltinFilter::Split(None)),
        ("split", 1) => Some(BuiltinFilter::Split(Some(args[0].clone_ref(py)))),
        ("join", 1) => Some(BuiltinFilter::Join(args[0].clone_ref(py))),
        ("startswith", 1) => Some(BuiltinFilter::Startswith(args[0].clone_ref(py))),
        ("endswith", 1) => Some(BuiltinFilter::Endswith(args[0].clone_ref(py))),
        ("matches", 1) => Some(BuiltinFilter::Matches(args[0].clone_ref(py))),
        ("keys", 0) => Some(BuiltinFilter::Keys),
        ("values", 0) => Some(BuiltinFilter::Values),
        ("items", 0) => Some(BuiltinFilter::Items),
        ("sort_by", 1) => Some(BuiltinFilter::SortBy(args[0].clone_ref(py), None)),
        ("sort_by", 2) => Some(BuiltinFilter::SortBy(
            args[0].clone_ref(py),
            Some(args[1].clone_ref(py)),
        )),
        ("unique_by", 1) => Some(BuiltinFilter::UniqueBy(args[0].clone_ref(py))),
        ("index_by", 1) => Some(BuiltinFilter::IndexBy(args[0].clone_ref(py))),
        ("group_by", 1) => Some(BuiltinFilter::GroupBy(args[0].clone_ref(py))),
        ("const", 1) => Some(BuiltinFilter::Const(args[0].clone_ref(py))),
        ("default", 1) => Some(BuiltinFilter::Default(args[0].clone_ref(py))),
        ("coalesce", n) if n >= 1 => Some(BuiltinFilter::Coalesce(
            args.iter().map(|arg| arg.clone_ref(py)).collect(),
        )),
        ("bool", 0) => Some(BuiltinFilter::Bool),
        ("type_is", 1) => Some(BuiltinFilter::TypeIs(args[0].clone_ref(py))),
        ("is_empty", 0) => Some(BuiltinFilter::IsEmpty),
        ("non_empty", 0) => Some(BuiltinFilter::NonEmpty),
        ("compact", 0) => Some(BuiltinFilter::Compact),
        ("from_json", 0) => Some(BuiltinFilter::FromJson),
        ("to_json", 0) => Some(BuiltinFilter::ToJson),
        ("to_datetime", 0) => Some(BuiltinFilter::ToDatetime(None)),
        ("to_datetime", 1) => Some(BuiltinFilter::ToDatetime(Some(args[0].clone_ref(py)))),
        ("strftime", 1) => Some(BuiltinFilter::Strftime(args[0].clone_ref(py))),
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
        ("idiv", 1) => Some(BuiltinFilter::IDiv(args[0].clone_ref(py))),
        ("mod", 1) => Some(BuiltinFilter::Mod(args[0].clone_ref(py))),
        _ => None,
    }
}

pub(crate) fn compile_builtin_pipeline(
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

pub(crate) fn apply_binary_op(
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

pub(crate) fn call_builtin1(py: Python<'_>, name: &str, arg: &PyObject) -> PyResult<PyObject> {
    py.import_bound("builtins")?
        .getattr(name)?
        .call1((arg.clone_ref(py),))
        .map(|v| v.into())
}

pub(crate) fn call_builtin2(
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

pub(crate) fn compare_with_fallback(
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

pub(crate) fn has_len_zero(py: Python<'_>, value: &PyObject) -> bool {
    value.bind(py).len().map(|len| len == 0).unwrap_or(false)
}

pub(crate) fn is_list_or_tuple(bound: &Bound<'_, PyAny>) -> bool {
    bound.is_instance_of::<PyList>() || bound.is_instance_of::<PyTuple>()
}

pub(crate) fn collect_sequence_items(
    py: Python<'_>,
    value: &PyObject,
) -> PyResult<Option<Vec<PyObject>>> {
    let value_bound = value.bind(py);
    if !is_list_or_tuple(&value_bound) {
        return Ok(None);
    }

    let len = value_bound.len()?;
    let mut out: Vec<PyObject> = Vec::with_capacity(len);
    for idx in 0..len {
        out.push(value_bound.get_item(idx)?.into());
    }

    Ok(Some(out))
}

pub(crate) fn extract_string_arg(
    py: Python<'_>,
    value: &PyObject,
    filter_name: &str,
    arg_name: &str,
) -> PyResult<String> {
    value.bind(py).extract::<String>().map_err(|_| {
        PyTypeError::new_err(format!(
            "Filter '${filter_name}' expects {arg_name} to be a string."
        ))
    })
}

pub(crate) fn as_datetime(
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

pub(crate) fn collect_numeric_sequence(
    py: Python<'_>,
    value: &PyObject,
) -> PyResult<Option<Vec<f64>>> {
    let value_bound = value.bind(py);
    if !is_list_or_tuple(&value_bound) {
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

pub(crate) fn percentile_value(sorted_values: &[f64], percentile: f64) -> Option<f64> {
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

pub(crate) fn flatten_deep_into(
    value: &Bound<'_, PyAny>,
    flattened: &Bound<'_, PyList>,
) -> PyResult<()> {
    if is_list_or_tuple(value) {
        let value_len = value.len()?;
        for idx in 0..value_len {
            let nested = value.get_item(idx)?;
            flatten_deep_into(&nested, flattened)?;
        }
        return Ok(());
    }

    flattened.append(value)?;
    Ok(())
}

pub(crate) fn resolve_relative_read_path(
    py: Python<'_>,
    module: &Bound<'_, PyModule>,
    registry: &Bound<'_, PyAny>,
    item: &PyObject,
    path: &str,
) -> PyResult<Option<PyObject>> {
    let (base_path, output_transform) = split_path_and_transform(path);
    let mut current = item.clone_ref(py);

    if base_path != "." {
        let tokens = parse_path(py, module, registry, &base_path)?;
        validate_read_path_root_token(py, &base_path, &tokens)?;

        for token in tokens {
            if matches!(token.kind, TokenKind::Root) {
                current = item.clone_ref(py);
                continue;
            }

            match resolve_token(py, module, registry, &current, item, &token.kind) {
                Ok(value) => current = value,
                Err(err) => {
                    if is_soft_resolution_error(py, &err) {
                        return Ok(None);
                    }
                    return Err(err);
                }
            }
        }
    }

    if let Some(transform) = output_transform {
        current = apply_output_transform(py, module, registry, &current, &transform, item)?;
    }

    Ok(Some(current))
}

pub(crate) fn compare_selector_values(
    py: Python<'_>,
    left: &PyObject,
    right: &PyObject,
    reverse: bool,
) -> PyResult<Ordering> {
    if compare_with_fallback(py, left, right, "<")? {
        return Ok(if reverse {
            Ordering::Greater
        } else {
            Ordering::Less
        });
    }
    if compare_with_fallback(py, left, right, ">")? {
        return Ok(if reverse {
            Ordering::Less
        } else {
            Ordering::Greater
        });
    }
    Ok(Ordering::Equal)
}

pub(crate) fn apply_builtin_filter(
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
        BuiltinFilter::IDiv(rhs) => {
            let is_zero = compare_values(py, rhs, &0i32.to_object(py), "==").unwrap_or(false);
            if is_zero {
                return Ok(py.None());
            }
            apply_binary_op(py, value, "__floordiv__", rhs)
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
        BuiltinFilter::Keys => {
            if !value.bind(py).is_instance_of::<PyDict>() {
                return Ok(py.None());
            }
            let source = value.bind(py).downcast::<PyDict>()?;
            let out = PyList::empty_bound(py);
            for (key, _) in source.iter() {
                out.append(key)?;
            }
            Ok(out.into())
        }
        BuiltinFilter::Values => {
            if !value.bind(py).is_instance_of::<PyDict>() {
                return Ok(py.None());
            }
            let source = value.bind(py).downcast::<PyDict>()?;
            let out = PyList::empty_bound(py);
            for (_, item_value) in source.iter() {
                out.append(item_value)?;
            }
            Ok(out.into())
        }
        BuiltinFilter::Items => {
            if !value.bind(py).is_instance_of::<PyDict>() {
                return Ok(py.None());
            }
            let source = value.bind(py).downcast::<PyDict>()?;
            let out = PyList::empty_bound(py);
            for (key, item_value) in source.iter() {
                let item = PyDict::new_bound(py);
                item.set_item("key", key)?;
                item.set_item("value", item_value)?;
                out.append(item)?;
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
        BuiltinFilter::SortBy(path_value, reverse_flag) => {
            let selector_path = extract_string_arg(py, path_value, "sort_by", "selector path")?;
            let reverse = reverse_flag
                .as_ref()
                .map(|flag| flag.bind(py).is_truthy())
                .transpose()?
                .unwrap_or(false);
            let module = py.import_bound("dictwalk.dictwalk")?;
            let registry = load_registry(py)?;
            let Some(mut items) = collect_sequence_items(py, value)? else {
                return Ok(value.clone_ref(py));
            };

            let mut keyed: Vec<(Option<PyObject>, PyObject)> = Vec::with_capacity(items.len());
            for item in items.drain(..) {
                let resolved =
                    resolve_relative_read_path(py, &module, &registry, &item, &selector_path)?;
                keyed.push((resolved, item));
            }

            let mut sorted: Vec<(Option<PyObject>, PyObject)> = Vec::with_capacity(keyed.len());
            for entry in keyed {
                let mut insert_at = sorted.len();
                for (idx, (existing_key, _)) in sorted.iter().enumerate() {
                    let ordering = match (&entry.0, existing_key) {
                        (None, None) => Ordering::Equal,
                        (None, Some(_)) => Ordering::Greater,
                        (Some(_), None) => Ordering::Less,
                        (Some(left), Some(right)) => {
                            compare_selector_values(py, left, right, reverse)?
                        }
                    };
                    if ordering == Ordering::Less {
                        insert_at = idx;
                        break;
                    }
                }
                sorted.insert(insert_at, entry);
            }

            let out = PyList::empty_bound(py);
            for (_, item) in sorted {
                out.append(item)?;
            }
            Ok(out.into())
        }
        BuiltinFilter::UniqueBy(path_value) => {
            let selector_path = extract_string_arg(py, path_value, "unique_by", "selector path")?;
            let module = py.import_bound("dictwalk.dictwalk")?;
            let registry = load_registry(py)?;
            let Some(items) = collect_sequence_items(py, value)? else {
                return Ok(value.clone_ref(py));
            };

            let out = PyList::empty_bound(py);
            let mut seen: Vec<PyObject> = Vec::new();
            for item in items {
                let Some(key) =
                    resolve_relative_read_path(py, &module, &registry, &item, &selector_path)?
                else {
                    out.append(item)?;
                    continue;
                };

                let mut is_duplicate = false;
                for existing in &seen {
                    if compare_values(py, existing, &key, "==").unwrap_or(false) {
                        is_duplicate = true;
                        break;
                    }
                }

                if !is_duplicate {
                    seen.push(key);
                    out.append(item)?;
                }
            }
            Ok(out.into())
        }
        BuiltinFilter::IndexBy(path_value) => {
            let selector_path = extract_string_arg(py, path_value, "index_by", "selector path")?;
            let module = py.import_bound("dictwalk.dictwalk")?;
            let registry = load_registry(py)?;
            let Some(items) = collect_sequence_items(py, value)? else {
                return Ok(value.clone_ref(py));
            };

            let out = PyDict::new_bound(py);
            for item in items {
                let Some(key) =
                    resolve_relative_read_path(py, &module, &registry, &item, &selector_path)?
                else {
                    continue;
                };
                out.set_item(key, item)?;
            }
            Ok(out.into())
        }
        BuiltinFilter::GroupBy(path_value) => {
            let selector_path = extract_string_arg(py, path_value, "group_by", "selector path")?;
            let module = py.import_bound("dictwalk.dictwalk")?;
            let registry = load_registry(py)?;
            let Some(items) = collect_sequence_items(py, value)? else {
                return Ok(value.clone_ref(py));
            };

            let out = PyDict::new_bound(py);
            for item in items {
                let Some(key) =
                    resolve_relative_read_path(py, &module, &registry, &item, &selector_path)?
                else {
                    continue;
                };

                let group_obj: PyObject = match out.get_item(key.clone_ref(py))? {
                    Some(existing) => existing.into(),
                    None => {
                        let new_group: PyObject = PyList::empty_bound(py).into();
                        out.set_item(key.clone_ref(py), new_group.clone_ref(py))?;
                        new_group
                    }
                };
                group_obj.bind(py).downcast::<PyList>()?.append(item)?;
            }
            Ok(out.into())
        }
        BuiltinFilter::Reverse => {
            let value_bound = value.bind(py);
            if !is_list_or_tuple(&value_bound) {
                return Ok(value.clone_ref(py));
            }

            let reversed = PyList::empty_bound(py);
            let value_len = value_bound.len()?;
            for idx in (0..value_len).rev() {
                reversed.append(value_bound.get_item(idx)?)?;
            }
            Ok(reversed.into())
        }
        BuiltinFilter::Chunk(size_value) => {
            let value_bound = value.bind(py);
            if !is_list_or_tuple(&value_bound) {
                return Ok(value.clone_ref(py));
            }

            let chunk_size_obj = call_builtin1(py, "int", size_value)?;
            let chunk_size = chunk_size_obj.bind(py).extract::<isize>()?;
            if chunk_size <= 0 {
                return Ok(py.None());
            }

            let chunked = PyList::empty_bound(py);
            let value_len = value_bound.len()? as isize;
            let mut start = 0isize;
            while start < value_len {
                let end = (start + chunk_size).min(value_len);
                let chunk = PyList::empty_bound(py);
                for idx in start..end {
                    chunk.append(value_bound.get_item(idx as usize)?)?;
                }
                chunked.append(chunk)?;
                start += chunk_size;
            }
            Ok(chunked.into())
        }
        BuiltinFilter::Flatten => {
            let value_bound = value.bind(py);
            if !is_list_or_tuple(&value_bound) {
                return Ok(value.clone_ref(py));
            }

            let flattened = PyList::empty_bound(py);
            let value_len = value_bound.len()?;
            for idx in 0..value_len {
                let item = value_bound.get_item(idx)?;
                if item.is_instance_of::<PyList>() || item.is_instance_of::<PyTuple>() {
                    let nested_len = item.len()?;
                    for nested_idx in 0..nested_len {
                        flattened.append(item.get_item(nested_idx)?)?;
                    }
                } else {
                    flattened.append(item)?;
                }
            }
            Ok(flattened.into())
        }
        BuiltinFilter::FlattenDeep => {
            let value_bound = value.bind(py);
            if !is_list_or_tuple(&value_bound) {
                return Ok(value.clone_ref(py));
            }

            let flattened = PyList::empty_bound(py);
            flatten_deep_into(value_bound, &flattened)?;
            Ok(flattened.into())
        }
        BuiltinFilter::Sorted(reverse) => {
            let value_bound = value.bind(py);
            if !is_list_or_tuple(&value_bound) {
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
            if is_list_or_tuple(&value_bound) {
                if value_bound.len()? == 0 {
                    return Ok(py.None());
                }
                return value_bound.get_item(0).map(|v| v.into());
            }
            Ok(value.clone_ref(py))
        }
        BuiltinFilter::Last => {
            let value_bound = value.bind(py);
            if is_list_or_tuple(&value_bound) {
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
        BuiltinFilter::RegexReplace(pattern, repl) => py
            .import_bound("re")?
            .getattr("sub")?
            .call1((
                pattern.clone_ref(py),
                repl.clone_ref(py),
                value.bind(py).str()?,
            ))
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
        BuiltinFilter::Const(constant) => Ok(constant.clone_ref(py)),
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
        BuiltinFilter::Compact => {
            let Some(items) = collect_sequence_items(py, value)? else {
                return Ok(value.clone_ref(py));
            };
            let out = PyList::empty_bound(py);
            for item in items {
                if !item.bind(py).is_none() {
                    out.append(item)?;
                }
            }
            Ok(out.into())
        }
        BuiltinFilter::FromJson => {
            if !value.bind(py).is_instance_of::<PyString>() {
                return Ok(py.None());
            }
            match py
                .import_bound("json")?
                .getattr("loads")?
                .call1((value.clone_ref(py),))
            {
                Ok(parsed) => Ok(parsed.into()),
                Err(_) => Ok(py.None()),
            }
        }
        BuiltinFilter::ToJson => py
            .import_bound("json")?
            .getattr("dumps")?
            .call1((value.clone_ref(py),))
            .map(|v| v.into()),
        BuiltinFilter::ToDatetime(fmt) => {
            Ok(as_datetime(py, value, fmt.as_ref())?.unwrap_or_else(|| py.None()))
        }
        BuiltinFilter::Strftime(fmt) => {
            let dt = match as_datetime(py, value, None) {
                Ok(Some(dt)) => dt,
                Ok(None) => return Ok(py.None()),
                Err(err) if err.is_instance_of::<PyValueError>(py) => return Ok(py.None()),
                Err(err) => return Err(err),
            };
            dt.bind(py)
                .call_method1("strftime", (fmt.clone_ref(py),))
                .map(|v| v.into())
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

pub(crate) fn apply_builtin_pipeline(
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

pub(crate) fn compare_values(
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

pub(crate) fn resolve_root_reference_value(
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
