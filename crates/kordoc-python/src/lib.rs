use kordoc_core::{MAX_INPUT_BYTES, detect_format, try_parse};
use kordoc_ir::{ErrorCode, FileType, ParseFailure};
use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;
use serde_json::Value;

#[pyfunction]
fn native_version() -> &'static str {
    env!("CARGO_PKG_VERSION")
}

#[pyfunction]
fn detect_format_bytes(py: Python<'_>, data: &[u8]) -> PyResult<&'static str> {
    let result = py.detach(|| detect_format(data));
    result
        .map(file_type_name)
        .map_err(|error| py_error(error.code, &error.message))
}

#[pyfunction]
fn try_parse_bytes(py: Python<'_>, data: &[u8]) -> PyResult<Py<PyAny>> {
    let result = py.detach(|| try_parse(data));
    let value = match result {
        Ok(success) => serde_json::to_value(success).map_err(to_py_error)?,
        Err(error) => {
            let failure = ParseFailure {
                file_type: error.file_type,
                success: false,
                error: error.message,
                code: Some(error.code),
                ..ParseFailure::default()
            };
            serde_json::to_value(failure).map_err(to_py_error)?
        }
    };
    json_to_python(py, value)
}

#[pyfunction]
fn max_input_bytes() -> usize {
    MAX_INPUT_BYTES
}

fn py_error(code: ErrorCode, message: &str) -> PyErr {
    PyValueError::new_err((code.as_str(), message.to_owned()))
}

fn to_py_error(_: serde_json::Error) -> PyErr {
    PyValueError::new_err((
        ErrorCode::ParseError.as_str(),
        "Failed to serialize parser result",
    ))
}

fn file_type_name(file_type: FileType) -> &'static str {
    match file_type {
        FileType::Hwpx => "hwpx",
        FileType::Hwp => "hwp",
        FileType::Hwp3 => "hwp3",
        FileType::Hwpml => "hwpml",
        FileType::Pdf => "pdf",
        FileType::Xlsx => "xlsx",
        FileType::Xls => "xls",
        FileType::Docx => "docx",
        FileType::Pptx => "pptx",
        FileType::Image => "image",
        FileType::Unknown => "unknown",
    }
}

fn json_to_python(py: Python<'_>, value: Value) -> PyResult<Py<PyAny>> {
    let serialized = serde_json::to_string(&value).map_err(to_py_error)?;
    Ok(py
        .import("json")?
        .call_method1("loads", (serialized,))?
        .unbind())
}

#[pymodule]
fn _native(module: &Bound<'_, PyModule>) -> PyResult<()> {
    module.add_function(wrap_pyfunction!(native_version, module)?)?;
    module.add_function(wrap_pyfunction!(detect_format_bytes, module)?)?;
    module.add_function(wrap_pyfunction!(try_parse_bytes, module)?)?;
    module.add_function(wrap_pyfunction!(max_input_bytes, module)?)?;
    Ok(())
}
