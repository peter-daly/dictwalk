//! Shared mutation options, validation, and container helpers.

use crate::*;

#[derive(Clone, Copy)]
pub(crate) struct WriteOptions {
    pub(crate) create_missing: bool,
    pub(crate) create_filter_match: bool,
    pub(crate) overwrite_incompatible: bool,
}

pub(crate) fn token_uses_root_selector(kind: &TokenKind) -> bool {
    matches!(
        kind,
        TokenKind::Root
            | TokenKind::RootMap
            | TokenKind::RootIndex { .. }
            | TokenKind::RootSlice { .. }
            | TokenKind::RootFilter { .. }
    )
}

pub(crate) fn path_uses_bare_root_token(tokens: &[ParsedToken]) -> bool {
    tokens
        .iter()
        .any(|token| matches!(token.kind, TokenKind::Root))
}

pub(crate) fn validate_read_path_root_token(
    py: Python<'_>,
    path: &str,
    tokens: &[ParsedToken],
) -> PyResult<()> {
    for (index, token) in tokens.iter().enumerate() {
        if token_uses_root_selector(&token.kind) && index != 0 {
            return Err(make_parse_error(
                py,
                path,
                Some(&token.raw),
                "Root selectors are only allowed at the start of a path; mid-path usage is not supported.",
            ));
        }
    }
    Ok(())
}

pub(crate) fn validate_write_path_root_selector(
    py: Python<'_>,
    path: &str,
    tokens: &[ParsedToken],
) -> PyResult<()> {
    for (index, token) in tokens.iter().enumerate() {
        if token_uses_root_selector(&token.kind) && index != 0 {
            return Err(make_parse_error(
                py,
                path,
                Some(&token.raw),
                "Root selectors are only allowed at the start of a path; mid-path usage is not supported.",
            ));
        }
    }
    Ok(())
}

pub(crate) fn ensure_path_resolves(
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

pub(crate) fn is_dict_or_list(bound: &Bound<'_, PyAny>) -> bool {
    bound.is_instance_of::<PyDict>() || bound.is_instance_of::<PyList>()
}

pub(crate) fn new_write_container(py: Python<'_>) -> PyObject {
    PyDict::new_bound(py).into()
}

pub(crate) fn resolve_new_value(
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

pub(crate) fn dict_keys(dict: &Bound<'_, PyDict>) -> Vec<PyObject> {
    let mut keys: Vec<PyObject> = Vec::new();
    for (key, _) in dict.iter() {
        keys.push(key.into());
    }
    keys
}

pub(crate) fn coerce_current_to_dict_for_write(
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

pub(crate) fn compute_slice_indexes(
    len: usize,
    start: Option<isize>,
    end: Option<isize>,
) -> Vec<usize> {
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
