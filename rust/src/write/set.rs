//! Recursive in-place set operations for every path token kind.

use super::shared::*;
use crate::*;
pub(crate) fn set_recurse(
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
        TokenKind::RootMap => set_root_map_token(
            py,
            module,
            registry,
            current,
            remaining,
            new_value,
            write_options,
            root_data,
        ),
        TokenKind::RootIndex { index } => set_root_index_token(
            py,
            module,
            registry,
            current,
            remaining,
            *index,
            new_value,
            write_options,
            root_data,
        ),
        TokenKind::RootSlice { start, end } => set_root_slice_token(
            py,
            module,
            registry,
            current,
            remaining,
            *start,
            *end,
            new_value,
            write_options,
            root_data,
        ),
        TokenKind::RootFilter {
            field,
            operator,
            value,
        } => set_root_filter_token(
            py,
            module,
            registry,
            current,
            remaining,
            field,
            operator,
            value,
            new_value,
            write_options,
            root_data,
        ),
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

pub(crate) fn set_get_token(
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

pub(crate) fn set_map_token(
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

pub(crate) fn set_root_map_token(
    py: Python<'_>,
    module: &Bound<'_, PyModule>,
    registry: &Bound<'_, PyAny>,
    current: PyObject,
    remaining: &[ParsedToken],
    new_value: &PyObject,
    write_options: WriteOptions,
    root_data: &PyObject,
) -> PyResult<PyObject> {
    let next_kind = remaining.get(1).map(|token| &token.kind);
    if !current.bind(py).is_instance_of::<PyList>() {
        return Ok(current);
    }
    let list = current.bind(py).downcast::<PyList>()?;

    if remaining.len() == 1 {
        for idx in 0..list.len() {
            let existing: PyObject = list.get_item(idx)?.into();
            let resolved =
                resolve_new_value(py, module, registry, Some(existing), new_value, root_data)?;
            list.set_item(idx, resolved)?;
        }
        return Ok(current);
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

    Ok(current)
}

pub(crate) fn set_root_index_token(
    py: Python<'_>,
    module: &Bound<'_, PyModule>,
    registry: &Bound<'_, PyAny>,
    current: PyObject,
    remaining: &[ParsedToken],
    index: isize,
    new_value: &PyObject,
    write_options: WriteOptions,
    root_data: &PyObject,
) -> PyResult<PyObject> {
    let next_kind = remaining.get(1).map(|token| &token.kind);
    if !current.bind(py).is_instance_of::<PyList>() {
        return Ok(current);
    }
    let list = current.bind(py).downcast::<PyList>()?;

    let idx = index;
    if idx < 0 {
        if idx < -(list.len() as isize) {
            return Ok(current);
        }
    } else {
        if !write_options.create_missing {
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
        return Ok(current);
    }

    let mut item: PyObject = list.get_item(target_index)?.into();
    if next_kind.is_some() && !is_dict_or_list(&item.bind(py)) {
        if !write_options.overwrite_incompatible {
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
    Ok(current)
}

pub(crate) fn set_root_slice_token(
    py: Python<'_>,
    module: &Bound<'_, PyModule>,
    registry: &Bound<'_, PyAny>,
    current: PyObject,
    remaining: &[ParsedToken],
    start: Option<isize>,
    end: Option<isize>,
    new_value: &PyObject,
    write_options: WriteOptions,
    root_data: &PyObject,
) -> PyResult<PyObject> {
    let next_kind = remaining.get(1).map(|token| &token.kind);
    if !current.bind(py).is_instance_of::<PyList>() {
        return Ok(current);
    }
    let list = current.bind(py).downcast::<PyList>()?;
    let indexes = compute_slice_indexes(list.len(), start, end);

    if remaining.len() == 1 {
        for idx in indexes {
            let existing = list.get_item(idx)?.into();
            let resolved =
                resolve_new_value(py, module, registry, Some(existing), new_value, root_data)?;
            list.set_item(idx, resolved)?;
        }
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

    Ok(current)
}

pub(crate) fn set_root_filter_token(
    py: Python<'_>,
    module: &Bound<'_, PyModule>,
    registry: &Bound<'_, PyAny>,
    current: PyObject,
    remaining: &[ParsedToken],
    field: &str,
    operator: &str,
    value: &str,
    new_value: &PyObject,
    write_options: WriteOptions,
    root_data: &PyObject,
) -> PyResult<PyObject> {
    if !current.bind(py).is_instance_of::<PyList>() {
        return Ok(current);
    }
    let list = current.bind(py).downcast::<PyList>()?;
    let matcher = compile_filter_matcher(py, module, registry, field, value)?;

    let mut matches: Vec<bool> = Vec::with_capacity(list.len());
    for idx in 0..list.len() {
        let item: PyObject = list.get_item(idx)?.into();
        matches.push(filter_matches_compiled(
            py,
            module,
            registry,
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
            let field_key = match &matcher.field_resolver {
                FieldValueResolver::Key(key) => key.as_str(),
                _ => field,
            };
            new_item.set_item(field_key, value)?;
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

    Ok(current)
}

pub(crate) fn set_wildcard_token(
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

pub(crate) fn deep_set_walk(
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

pub(crate) fn set_deep_wildcard_token(
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

pub(crate) fn set_index_token(
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

pub(crate) fn set_slice_token(
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

pub(crate) fn set_filter_token(
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
            module,
            registry,
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
            let field_key = match &matcher.field_resolver {
                FieldValueResolver::Key(key) => key.as_str(),
                _ => field,
            };
            new_item.set_item(field_key, value)?;
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
