//! PyO3 extension boundary for `dictwalk`.
//!
//! The implementation is split by responsibility below; this module intentionally
//! contains only shared imports and the Python-facing `DictWalk` API.

pub(crate) use pyo3::basic::CompareOp;
pub(crate) use pyo3::exceptions::{PyKeyError, PyRuntimeError, PyTypeError, PyValueError};
pub(crate) use pyo3::prelude::*;
pub(crate) use pyo3::types::{PyAny, PyDict, PyFloat, PyInt, PyList, PyModule, PyString, PyTuple};
pub(crate) use regex::Regex;
pub(crate) use std::cmp::Ordering;
pub(crate) use std::sync::LazyLock;

mod errors;
pub(crate) use errors::*;

mod filters;
pub(crate) use filters::*;

mod path;
pub(crate) use path::*;

mod read;
pub(crate) use read::*;

mod write;
pub(crate) use write::*;

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
        validate_read_path_root_token(py, &base_path, &tokens)?;
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
        validate_read_path_root_token(py, path, &tokens)?;
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

        validate_write_path_root_selector(py, path, &tokens)?;

        if path_uses_bare_root_token(&tokens) {
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

        validate_write_path_root_selector(py, path, &tokens)?;

        if path_uses_bare_root_token(&tokens) {
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
