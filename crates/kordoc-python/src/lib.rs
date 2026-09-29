use kordoc_core::{
    MAX_INPUT_BYTES, OcrOption, PageNumber, PageSelection, ParseOptions, detect_format,
    detect_ole2_format, detect_zip_format, is_hwpx_file, is_old_hwp_file, is_pdf_file, is_zip_file,
    try_parse_with_options,
};
use kordoc_ir::{ErrorCode, FileType, ParseFailure};
use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;
use pyo3::types::{PyBool, PyDict, PyList, PyString, PyTuple};
use serde_json::Value;

const MAX_PAGE_SELECTION_ITEMS: usize = 100_000;
const MAX_OPTION_STRING_LENGTH: usize = 65_536;
const OPTION_KEYS: &[&str] = &[
    "pages",
    "ocr",
    "removeHeaderFooter",
    "scriptTags",
    "plain",
    "htmlTables",
    "keepTrailingEmptyCols",
    "classifyTables",
    "keepEmptyParagraphs",
    "includeFieldPlaceholders",
    "password",
    "formulaOcr",
    "dedupeRunningHeaders",
    "inlineImages",
    "images",
    "tables",
];
const BOOLEAN_OPTION_KEYS: &[&str] = &[
    "removeHeaderFooter",
    "scriptTags",
    "plain",
    "htmlTables",
    "keepTrailingEmptyCols",
    "classifyTables",
    "keepEmptyParagraphs",
    "includeFieldPlaceholders",
    "formulaOcr",
    "dedupeRunningHeaders",
    "inlineImages",
    "images",
    "tables",
];

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

#[pyfunction(signature = (data, options=None))]
fn try_parse_bytes(
    py: Python<'_>,
    data: &[u8],
    options: Option<&Bound<'_, PyAny>>,
) -> PyResult<Py<PyAny>> {
    let options = parse_options(py, options)?;
    let result = py.detach(|| try_parse_with_options(data, &options));
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
fn detect_zip_format_bytes(py: Python<'_>, data: &[u8]) -> &'static str {
    file_type_name(py.detach(|| detect_zip_format(data)))
}

#[pyfunction]
fn detect_ole2_format_bytes(py: Python<'_>, data: &[u8]) -> &'static str {
    file_type_name(py.detach(|| detect_ole2_format(data)))
}

#[pyfunction]
fn is_zip_file_bytes(data: &[u8]) -> bool {
    is_zip_file(data)
}

#[pyfunction]
fn is_hwpx_file_bytes(data: &[u8]) -> bool {
    is_hwpx_file(data)
}

#[pyfunction]
fn is_old_hwp_file_bytes(data: &[u8]) -> bool {
    is_old_hwp_file(data)
}

#[pyfunction]
fn is_pdf_file_bytes(data: &[u8]) -> bool {
    is_pdf_file(data)
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

fn parse_options(py: Python<'_>, options: Option<&Bound<'_, PyAny>>) -> PyResult<ParseOptions> {
    let Some(options) = options else {
        return Ok(ParseOptions::default());
    };
    validate_option_budget(options)?;
    let serialized: String = py
        .import("json")?
        .call_method1("dumps", (options,))?
        .extract()?;
    let value: Value = serde_json::from_str(&serialized).map_err(|_| invalid_options())?;
    let Value::Object(mut object) = value else {
        return Err(invalid_options());
    };

    if object
        .keys()
        .any(|key| !OPTION_KEYS.contains(&key.as_str()))
    {
        return Err(invalid_options());
    }

    Ok(ParseOptions {
        pages: take_pages(&mut object)?,
        ocr: take_ocr(&mut object)?,
        remove_header_footer: take_bool(&mut object, "removeHeaderFooter")?,
        script_tags: take_bool(&mut object, "scriptTags")?,
        plain: take_bool(&mut object, "plain")?,
        html_tables: take_bool(&mut object, "htmlTables")?,
        keep_trailing_empty_cols: take_bool(&mut object, "keepTrailingEmptyCols")?,
        classify_tables: take_bool(&mut object, "classifyTables")?,
        keep_empty_paragraphs: take_bool(&mut object, "keepEmptyParagraphs")?,
        include_field_placeholders: take_bool(&mut object, "includeFieldPlaceholders")?,
        password: take_string(&mut object, "password")?,
        formula_ocr: take_bool(&mut object, "formulaOcr")?,
        dedupe_running_headers: take_bool(&mut object, "dedupeRunningHeaders")?,
        inline_images: take_bool(&mut object, "inlineImages")?,
        images: take_bool(&mut object, "images")?,
        tables: take_bool(&mut object, "tables")?,
    })
}

fn validate_option_budget(options: &Bound<'_, PyAny>) -> PyResult<()> {
    let object = options.cast::<PyDict>().map_err(|_| invalid_options())?;
    if object.len() > OPTION_KEYS.len() {
        return Err(invalid_options());
    }
    for (key, value) in object.iter() {
        let key: String = key.extract().map_err(|_| invalid_options())?;
        if !OPTION_KEYS.contains(&key.as_str()) {
            return Err(invalid_options());
        }
        if BOOLEAN_OPTION_KEYS.contains(&key.as_str()) {
            if !value.is_instance_of::<PyBool>() {
                return Err(invalid_options());
            }
        } else if key == "password" {
            if !value.is_instance_of::<PyString>() || value.len()? > MAX_OPTION_STRING_LENGTH {
                return Err(invalid_options());
            }
        } else if key == "ocr" {
            if !value.is_instance_of::<PyBool>()
                && (!value.is_instance_of::<PyString>()
                    || value.extract::<String>().map_err(|_| invalid_options())? != "force")
            {
                return Err(invalid_options());
            }
        } else if value.is_instance_of::<PyString>() {
            if value.len()? > MAX_OPTION_STRING_LENGTH {
                return Err(invalid_options());
            }
        } else if value.is_instance_of::<PyList>() || value.is_instance_of::<PyTuple>() {
            if value.len()? > MAX_PAGE_SELECTION_ITEMS {
                return Err(invalid_options());
            }
            for item in value.try_iter()? {
                let item = item?;
                if item.is_instance_of::<PyBool>()
                    || !item
                        .extract::<f64>()
                        .map_err(|_| invalid_options())?
                        .is_finite()
                {
                    return Err(invalid_options());
                }
            }
        } else {
            return Err(invalid_options());
        }
    }
    Ok(())
}

fn take_bool(object: &mut serde_json::Map<String, Value>, key: &str) -> PyResult<Option<bool>> {
    object
        .remove(key)
        .map(|value| value.as_bool().ok_or_else(invalid_options))
        .transpose()
}

fn take_string(object: &mut serde_json::Map<String, Value>, key: &str) -> PyResult<Option<String>> {
    object
        .remove(key)
        .map(|value| {
            value
                .as_str()
                .map(ToOwned::to_owned)
                .ok_or_else(invalid_options)
        })
        .transpose()
}

fn take_pages(object: &mut serde_json::Map<String, Value>) -> PyResult<Option<PageSelection>> {
    let Some(value) = object.remove("pages") else {
        return Ok(None);
    };
    match value {
        Value::String(range) => Ok(Some(PageSelection::Range(range))),
        Value::Array(values) => values
            .into_iter()
            .map(|value| {
                value
                    .as_f64()
                    .and_then(PageNumber::new)
                    .ok_or_else(invalid_options)
            })
            .collect::<PyResult<Vec<_>>>()
            .map(PageSelection::Numbers)
            .map(Some),
        _ => Err(invalid_options()),
    }
}

fn take_ocr(object: &mut serde_json::Map<String, Value>) -> PyResult<Option<OcrOption>> {
    let Some(value) = object.remove("ocr") else {
        return Ok(None);
    };
    match value {
        Value::Bool(value) => Ok(Some(OcrOption::Boolean(value))),
        Value::String(value) if value == "force" => Ok(Some(OcrOption::Force)),
        _ => Err(invalid_options()),
    }
}

fn invalid_options() -> PyErr {
    PyValueError::new_err((ErrorCode::ParseError.as_str(), "Invalid parser options"))
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
    module.add_function(wrap_pyfunction!(detect_zip_format_bytes, module)?)?;
    module.add_function(wrap_pyfunction!(detect_ole2_format_bytes, module)?)?;
    module.add_function(wrap_pyfunction!(is_zip_file_bytes, module)?)?;
    module.add_function(wrap_pyfunction!(is_hwpx_file_bytes, module)?)?;
    module.add_function(wrap_pyfunction!(is_old_hwp_file_bytes, module)?)?;
    module.add_function(wrap_pyfunction!(is_pdf_file_bytes, module)?)?;
    module.add_function(wrap_pyfunction!(try_parse_bytes, module)?)?;
    module.add_function(wrap_pyfunction!(max_input_bytes, module)?)?;
    Ok(())
}
