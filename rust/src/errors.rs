//! Construction of the Python exception types exposed by `dictwalk.errors`.

use crate::*;

pub(crate) fn make_error(py: Python<'_>, class_name: &str, message: &str) -> PyErr {
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

pub(crate) fn make_parse_error(
    py: Python<'_>,
    path: &str,
    token: Option<&str>,
    message: &str,
) -> PyErr {
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

pub(crate) fn make_resolution_error(
    py: Python<'_>,
    path: &str,
    token: Option<&str>,
    message: &str,
) -> PyErr {
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

pub(crate) fn load_registry(py: Python<'_>) -> PyResult<Bound<'_, PyAny>> {
    Ok(py.None().into_bound(py))
}
