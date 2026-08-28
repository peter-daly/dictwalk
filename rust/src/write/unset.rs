//! Recursive in-place unset operations for every path token kind.

use super::shared::*;
use crate::*;
pub(crate) fn unset_recurse(
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
        TokenKind::RootMap => unset_root_map_token(py, module, registry, current, remaining),
        TokenKind::RootIndex { index } => {
            unset_root_index_token(py, module, registry, current, remaining, *index)
        }
        TokenKind::RootSlice { start, end } => {
            unset_root_slice_token(py, module, registry, current, remaining, *start, *end)
        }
        TokenKind::RootFilter {
            field,
            operator,
            value,
        } => unset_root_filter_token(
            py, module, registry, current, remaining, field, operator, value,
        ),
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

pub(crate) fn unset_get_token(
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

pub(crate) fn unset_map_token(
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

pub(crate) fn unset_root_map_token(
    py: Python<'_>,
    module: &Bound<'_, PyModule>,
    registry: &Bound<'_, PyAny>,
    current: PyObject,
    remaining: &[ParsedToken],
) -> PyResult<PyObject> {
    if !current.bind(py).is_instance_of::<PyList>() {
        return Ok(current);
    }
    let list = current.bind(py).downcast::<PyList>()?;

    if remaining.len() == 1 {
        list.call_method0("clear")?;
        return Ok(current);
    }

    for idx in 0..list.len() {
        let item: PyObject = list.get_item(idx)?.into();
        let updated = unset_recurse(py, module, registry, item, &remaining[1..])?;
        list.set_item(idx, updated)?;
    }
    Ok(current)
}

pub(crate) fn unset_root_index_token(
    py: Python<'_>,
    module: &Bound<'_, PyModule>,
    registry: &Bound<'_, PyAny>,
    current: PyObject,
    remaining: &[ParsedToken],
    index: isize,
) -> PyResult<PyObject> {
    if !current.bind(py).is_instance_of::<PyList>() {
        return Ok(current);
    }
    let list = current.bind(py).downcast::<PyList>()?;
    let in_bounds = index >= -(list.len() as isize) && index < list.len() as isize;

    if remaining.len() == 1 {
        if in_bounds {
            list.call_method1("pop", (index,))?;
        }
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

    Ok(current)
}

pub(crate) fn unset_root_slice_token(
    py: Python<'_>,
    module: &Bound<'_, PyModule>,
    registry: &Bound<'_, PyAny>,
    current: PyObject,
    remaining: &[ParsedToken],
    start: Option<isize>,
    end: Option<isize>,
) -> PyResult<PyObject> {
    if !current.bind(py).is_instance_of::<PyList>() {
        return Ok(current);
    }
    let list = current.bind(py).downcast::<PyList>()?;
    let indexes = compute_slice_indexes(list.len(), start, end);

    if remaining.len() == 1 {
        for idx in indexes.iter().rev() {
            list.call_method1("pop", (*idx as isize,))?;
        }
        return Ok(current);
    }

    for idx in indexes {
        let child: PyObject = list.get_item(idx)?.into();
        let updated = unset_recurse(py, module, registry, child, &remaining[1..])?;
        list.set_item(idx, updated)?;
    }

    Ok(current)
}

pub(crate) fn unset_root_filter_token(
    py: Python<'_>,
    module: &Bound<'_, PyModule>,
    registry: &Bound<'_, PyAny>,
    current: PyObject,
    remaining: &[ParsedToken],
    field: &str,
    operator: &str,
    value: &str,
) -> PyResult<PyObject> {
    if !current.bind(py).is_instance_of::<PyList>() {
        return Ok(current);
    }

    let list = current.bind(py).downcast::<PyList>()?;
    let matcher = compile_filter_matcher(py, module, registry, field, value)?;

    if remaining.len() == 1 {
        let filtered = PyList::empty_bound(py);
        for idx in 0..list.len() {
            let item = list.get_item(idx)?;
            let item_obj: PyObject = item.clone().into();
            if !filter_matches_compiled(py, module, registry, operator, &matcher, &item_obj, None)?
            {
                filtered.append(item)?;
            }
        }
        list.call_method0("clear")?;
        for item in filtered.iter() {
            list.append(item)?;
        }
        return Ok(current);
    }

    for idx in 0..list.len() {
        let item = list.get_item(idx)?;
        let item_obj: PyObject = item.clone().into();
        if !filter_matches_compiled(py, module, registry, operator, &matcher, &item_obj, None)? {
            continue;
        }
        let child: PyObject = item.into();
        let updated = unset_recurse(py, module, registry, child, &remaining[1..])?;
        list.set_item(idx, updated)?;
    }

    Ok(current)
}

pub(crate) fn unset_wildcard_token(
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

pub(crate) fn deep_unset_walk(
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

pub(crate) fn unset_deep_wildcard_token(
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

pub(crate) fn unset_index_token(
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

pub(crate) fn unset_slice_token(
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

pub(crate) fn unset_filter_token(
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
            if !filter_matches_compiled(py, module, registry, operator, &matcher, &item_obj, None)?
            {
                filtered.append(item)?;
            }
        }
        dict.set_item(list_key, filtered)?;
        return Ok(current);
    }

    for idx in 0..list.len() {
        let child: PyObject = list.get_item(idx)?.into();
        if !filter_matches_compiled(py, module, registry, operator, &matcher, &child, None)? {
            continue;
        }
        let updated = unset_recurse(py, module, registry, child, &remaining[1..])?;
        list.set_item(idx, updated)?;
    }

    dict.set_item(list_key, list_obj)?;
    Ok(current)
}
