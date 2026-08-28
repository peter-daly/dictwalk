//! Boolean predicate parsing and list-filter matching.

use super::builtins::*;
use crate::*;
pub(crate) enum PredicateExpr {
    Pipeline(BuiltinFilterPipeline),
    Not(Box<PredicateExpr>),
    And(Box<PredicateExpr>, Box<PredicateExpr>),
    Or(Box<PredicateExpr>, Box<PredicateExpr>),
}

pub(crate) fn tokenize_boolean_filter_expression(expression: &str) -> Vec<String> {
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

pub(crate) struct PredicateParser<'py> {
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

pub(crate) fn compile_builtin_or_boolean_predicate(
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

pub(crate) fn eval_predicate_expr(
    py: Python<'_>,
    expr: &PredicateExpr,
    value: &PyObject,
) -> PyResult<bool> {
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

pub(crate) fn resolve_predicate_filter(
    _module: &Bound<'_, PyModule>,
    _registry: &Bound<'_, PyAny>,
    py: Python<'_>,
    expr: &str,
) -> PyResult<Option<PredicateExpr>> {
    compile_builtin_or_boolean_predicate(py, expr)
        .map_err(|message| make_parse_error(py, expr, Some(expr), &message))
}

pub(crate) enum FieldValueResolver {
    CurrentItem,
    CurrentItemBuiltinPipeline(BuiltinFilterPipeline),
    CurrentItemTransform(Option<BuiltinFilterPipeline>),
    PredicateFilter(PredicateExpr),
    Key(String),
    RelativePath(Vec<ParsedToken>),
}

pub(crate) enum ValueMatcher {
    BuiltinPipeline(BuiltinFilterPipeline),
    PredicateExpr(PredicateExpr),
    Literal(PyObject),
}

pub(crate) struct CompiledFilterMatcher {
    pub(crate) field_resolver: FieldValueResolver,
    pub(crate) value_matcher: ValueMatcher,
    pub(crate) raw_value: String,
}

pub(crate) fn compile_filter_matcher(
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
    } else if let Some(field_key) = field.strip_prefix('.') {
        if field_key.contains('[') || field_key.contains('.') {
            let field_tokens = parse_path(py, module, registry, field_key)?;
            FieldValueResolver::RelativePath(field_tokens)
        } else {
            FieldValueResolver::Key(field_key.to_string())
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

pub(crate) fn resolve_filter_field_value_compiled(
    py: Python<'_>,
    module: &Bound<'_, PyModule>,
    registry: &Bound<'_, PyAny>,
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
        FieldValueResolver::RelativePath(tokens) => {
            let mut current = item.clone_ref(py);
            for token in tokens {
                if matches!(token.kind, TokenKind::Root) {
                    current = item.clone_ref(py);
                    continue;
                }
                let resolved = resolve_token(py, module, registry, &current, item, &token.kind);
                match resolved {
                    Ok(value) => current = value,
                    Err(err) => {
                        if is_soft_resolution_error(py, &err) {
                            return Ok(py.None());
                        }
                        return Err(err);
                    }
                }
            }
            Ok(current)
        }
    }
}

pub(crate) fn filter_matches_compiled(
    py: Python<'_>,
    module: &Bound<'_, PyModule>,
    registry: &Bound<'_, PyAny>,
    operator: &str,
    matcher: &CompiledFilterMatcher,
    item: &PyObject,
    root_data: Option<&PyObject>,
) -> PyResult<bool> {
    let field_value = resolve_filter_field_value_compiled(py, module, registry, matcher, item)?;

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

pub(crate) fn resolve_filter_token(
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
        if filter_matches_compiled(
            py,
            module,
            registry,
            operator,
            &matcher,
            &item_obj,
            Some(root_data),
        )? {
            out.append(item)?;
        }
    }

    Ok(out.into())
}

pub(crate) fn resolve_root_filter_token(
    py: Python<'_>,
    module: &Bound<'_, PyModule>,
    registry: &Bound<'_, PyAny>,
    current: &PyObject,
    root_data: &PyObject,
    field: &str,
    operator: &str,
    value: &str,
) -> PyResult<PyObject> {
    let matcher = compile_filter_matcher(py, module, registry, field, value)?;
    let source_bound = current.bind(py);
    let source_list = source_bound.downcast::<PyList>().map_err(|_| {
        PyTypeError::new_err(format!(
            "Expected a list for root filter, got {}.",
            get_type_name(&source_bound)
        ))
    })?;

    let out = PyList::empty_bound(py);
    for item in source_list.iter() {
        let item_obj: PyObject = item.clone().into();
        if filter_matches_compiled(
            py,
            module,
            registry,
            operator,
            &matcher,
            &item_obj,
            Some(root_data),
        )? {
            out.append(item)?;
        }
    }

    Ok(out.into())
}
