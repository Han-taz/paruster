use pyo3::prelude::*;

#[pyfunction]
fn native_version() -> &'static str {
    env!("CARGO_PKG_VERSION")
}

#[pymodule]
fn _native(module: &Bound<'_, PyModule>) -> PyResult<()> {
    module.add_function(wrap_pyfunction!(native_version, module)?)?;
    Ok(())
}

#[cfg(test)]
mod contract_tests {
    use super::py_error;
    use kordoc_ir::ErrorCode;
    use pyo3::exceptions::PyValueError;
    use pyo3::prelude::*;

    #[test]
    fn coded_native_error_uses_two_string_value_error_arguments() {
        Python::attach(|py| {
            let error = py_error(ErrorCode::EmptyInput, "safe message");
            assert!(error.is_instance_of::<PyValueError>(py));
            let args: (String, String) = error.value(py).getattr("args")?.extract()?;
            assert_eq!(args, ("EMPTY_INPUT".into(), "safe message".into()));
            Ok::<(), PyErr>(())
        })
        .unwrap();
    }
}
