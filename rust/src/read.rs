//! Resolution of parsed path tokens for read and existence operations.

use crate::*;

pub(crate) fn resolve_get_token(
    py: Python<'_>,
    current: &PyObject,
    key: &str,
) -> PyResult<PyObject> {
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

pub(crate) fn get_type_name(bound: &Bound<'_, PyAny>) -> String {
    let bound_type = bound.get_type();
    bound_type
        .name()
        .map(|name: Bound<'_, PyString>| name.to_string_lossy().into_owned())
        .unwrap_or_else(|_| "unknown".to_string())
}

pub(crate) fn resolve_map_token(
    py: Python<'_>,
    current: &PyObject,
    key: &str,
) -> PyResult<PyObject> {
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

pub(crate) fn resolve_root_map_token(py: Python<'_>, current: &PyObject) -> PyResult<PyObject> {
    let bound = current.bind(py);
    if !bound.is_instance_of::<PyList>() {
        return Err(PyTypeError::new_err(format!(
            "Expected a list for root map '[]', got {}.",
            get_type_name(&bound)
        )));
    }
    Ok(current.clone_ref(py))
}

pub(crate) fn iter_child_nodes(py: Python<'_>, node: &Bound<'_, PyAny>) -> PyResult<Vec<PyObject>> {
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

pub(crate) fn resolve_wildcard_token(py: Python<'_>, current: &PyObject) -> PyResult<PyObject> {
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

pub(crate) fn collect_descendants(
    py: Python<'_>,
    node: PyObject,
    out: &Bound<'_, PyList>,
) -> PyResult<()> {
    let bound = node.bind(py);
    for child in iter_child_nodes(py, &bound)? {
        out.append(child.clone_ref(py))?;
        collect_descendants(py, child, out)?;
    }
    Ok(())
}

pub(crate) fn resolve_deep_wildcard_token(
    py: Python<'_>,
    current: &PyObject,
) -> PyResult<PyObject> {
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

pub(crate) fn apply_output_transform(
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

pub(crate) fn resolve_index_token(
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

pub(crate) fn resolve_root_index_token(
    py: Python<'_>,
    current: &PyObject,
    index: isize,
) -> PyResult<PyObject> {
    let bound = current.bind(py);
    let list = bound.downcast::<PyList>().map_err(|_| {
        PyTypeError::new_err(format!(
            "Expected a list for root index '[{index}]', got {}.",
            get_type_name(&bound)
        ))
    })?;

    let index_obj = index.to_object(py);
    list.as_any().get_item(index_obj).map(|value| value.into())
}

pub(crate) fn resolve_slice_token(
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

pub(crate) fn resolve_root_slice_token(
    py: Python<'_>,
    current: &PyObject,
    start: Option<isize>,
    end: Option<isize>,
) -> PyResult<PyObject> {
    let bound = current.bind(py);
    let list = bound.downcast::<PyList>().map_err(|_| {
        PyTypeError::new_err(format!(
            "Expected a list for root slice, got {}.",
            get_type_name(&bound)
        ))
    })?;

    let indexes = compute_slice_indexes(list.len(), start, end);
    let out = PyList::empty_bound(py);
    for idx in indexes {
        out.append(list.get_item(idx)?)?;
    }
    Ok(out.into())
}

pub(crate) fn is_soft_resolution_error(py: Python<'_>, err: &PyErr) -> bool {
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

pub(crate) fn resolve_token(
    py: Python<'_>,
    module: &Bound<'_, PyModule>,
    registry: &Bound<'_, PyAny>,
    current: &PyObject,
    root_data: &PyObject,
    kind: &TokenKind,
) -> PyResult<PyObject> {
    match kind {
        TokenKind::RootMap => resolve_root_map_token(py, current),
        TokenKind::RootIndex { index } => resolve_root_index_token(py, current, *index),
        TokenKind::RootSlice { start, end } => resolve_root_slice_token(py, current, *start, *end),
        TokenKind::RootFilter {
            field,
            operator,
            value,
        } => resolve_root_filter_token(
            py, module, registry, current, root_data, field, operator, value,
        ),
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
