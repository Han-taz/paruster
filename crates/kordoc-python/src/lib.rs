use kordoc_core::{
    MAX_INPUT_BYTES, OcrOption, PageNumber, PageSelection, ParseOptions, blocks_to_chunks,
    blocks_to_markdown, blocks_to_pages, detect_format, detect_ole2_format, detect_zip_format,
    is_hwpx_file, is_old_hwp_file, is_pdf_file, is_zip_file, try_parse_with_options,
};
use kordoc_ir::{
    BoundingBox, ChunkOptions, ErrorCode, FileType, ImageData, InlineStyle, IrBlock, IrBlockType,
    IrCell, IrSpan, IrTable, KordocError, ListType, ParseFailure, TableClassificationKind,
    TableClassificationReason, TableClassificationSummary,
};
use pyo3::exceptions::{PyTypeError, PyValueError};
use pyo3::prelude::*;
use pyo3::types::{PyBool, PyBytes, PyDict, PyFloat, PyInt, PyList, PyNone, PyString, PyTuple};
use serde::Deserialize;
use serde_json::Value;
use std::cell::RefCell;
use std::io::{self, Write};

const MAX_PAGE_SELECTION_ITEMS: usize = 100_000;
const MAX_OPTION_STRING_LENGTH: usize = 65_536;
const MAX_PROJECTION_JSON_BYTES: usize = 256 * 1024 * 1024;
const MAX_PROJECTION_NODES: usize = 32_000_000;
const MAX_PROJECTION_JSON_DEPTH: usize = 256;
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

#[pyfunction]
fn blocks_to_markdown_wire(py: Python<'_>, blocks: &Bound<'_, PyAny>) -> PyResult<String> {
    let blocks = projection_blocks(py, blocks)?;
    py.detach(|| blocks_to_markdown(&blocks))
        .map_err(core_to_py_error)
}

#[pyfunction(signature = (blocks, render=None))]
fn blocks_to_pages_wire(
    py: Python<'_>,
    blocks: &Bound<'_, PyAny>,
    render: Option<&Bound<'_, PyAny>>,
) -> PyResult<Py<PyAny>> {
    let blocks = projection_blocks(py, blocks)?;
    let Some(render) = render else {
        let pages = py
            .detach(|| blocks_to_pages(&blocks, None, blocks_to_markdown))
            .map_err(core_to_py_error)?;
        let serialized = capped_json_serialization(MAX_PROJECTION_JSON_BYTES, |writer| {
            serde_json::to_writer(writer, &pages)
        })?;
        return json_text_to_python(py, serialized);
    };
    if !render.is_callable() {
        return Err(PyTypeError::new_err("render must be callable"));
    }

    // This closure calls back into Python, so this branch intentionally stays attached to the GIL.
    let callback_error = RefCell::new(None);
    let pages = blocks_to_pages(&blocks, None, |page_blocks| {
        match render_page(py, render, page_blocks) {
            Ok(markdown) => Ok(markdown),
            Err(error) => {
                *callback_error.borrow_mut() = Some(error);
                Err(KordocError::new(
                    ErrorCode::ParseError,
                    "Page render callback failed",
                ))
            }
        }
    });
    if let Some(error) = callback_error.into_inner() {
        return Err(error);
    }
    let pages = pages.map_err(core_to_py_error)?;
    let serialized = capped_json_serialization(MAX_PROJECTION_JSON_BYTES, |writer| {
        serde_json::to_writer(writer, &pages)
    })?;
    json_text_to_python(py, serialized)
}

#[pyfunction(signature = (blocks, options=None))]
fn blocks_to_chunks_wire(
    py: Python<'_>,
    blocks: &Bound<'_, PyAny>,
    options: Option<&Bound<'_, PyAny>>,
) -> PyResult<Py<PyAny>> {
    let blocks = projection_blocks(py, blocks)?;
    let options = projection_chunk_options(py, options)?;
    let chunks = py
        .detach(|| blocks_to_chunks(&blocks, options))
        .map_err(core_to_py_error)?;
    let serialized = capped_json_serialization(MAX_PROJECTION_JSON_BYTES, |writer| {
        serde_json::to_writer(writer, &chunks)
    })?;
    json_text_to_python(py, serialized)
}

#[pyfunction]
fn classify_tables_wire(py: Python<'_>, blocks: &Bound<'_, PyAny>) -> PyResult<Py<PyAny>> {
    let mut blocks = projection_blocks(py, blocks)?;
    blocks = py
        .detach(move || {
            kordoc_core::table::classifier::classify_table_tree(&mut blocks)?;
            Ok::<_, KordocError>(blocks)
        })
        .map_err(core_to_py_error)?;
    let serialized = capped_json_serialization(MAX_PROJECTION_JSON_BYTES, |writer| {
        serde_json::to_writer(writer, &blocks)
    })?;
    json_text_to_python(py, serialized)
}

#[derive(Default)]
struct ProjectionInputBudget {
    nodes: usize,
    json_bytes: usize,
}

impl ProjectionInputBudget {
    fn add_node(&mut self, bytes: usize) -> PyResult<()> {
        self.nodes = self
            .nodes
            .checked_add(1)
            .filter(|count| *count <= MAX_PROJECTION_NODES)
            .ok_or_else(projection_too_large)?;
        self.add_bytes(bytes)
    }

    fn add_bytes(&mut self, bytes: usize) -> PyResult<()> {
        self.json_bytes = self
            .json_bytes
            .checked_add(bytes)
            .filter(|size| *size <= MAX_PROJECTION_JSON_BYTES)
            .ok_or_else(projection_too_large)?;
        Ok(())
    }
}

fn projection_too_large() -> PyErr {
    py_error(
        ErrorCode::OutputTooLarge,
        "Projection input exceeds the configured safety limit",
    )
}

fn invalid_projection_input() -> PyErr {
    py_error(ErrorCode::ParseError, "Invalid projection input")
}

fn projection_blocks(py: Python<'_>, blocks: &Bound<'_, PyAny>) -> PyResult<Vec<IrBlock>> {
    if !(blocks.is_instance_of::<PyList>() || blocks.is_instance_of::<PyTuple>()) {
        return Err(invalid_projection_input());
    }
    let serialized = bounded_python_json(py, blocks)?;
    let mut deserializer = serde_json::Deserializer::from_str(&serialized);
    deserializer.disable_recursion_limit();
    Vec::<IrBlock>::deserialize(&mut deserializer)
        .and_then(|blocks| deserializer.end().map(|()| blocks))
        .map_err(|_| invalid_projection_input())
}

fn projection_chunk_options(
    py: Python<'_>,
    options: Option<&Bound<'_, PyAny>>,
) -> PyResult<ChunkOptions> {
    let Some(options) = options.filter(|value| !value.is_none()) else {
        return Ok(ChunkOptions::default());
    };
    if !options.is_instance_of::<PyDict>() {
        return Err(invalid_projection_input());
    }
    let serialized = bounded_python_json(py, options)?;
    let mut deserializer = serde_json::Deserializer::from_str(&serialized);
    deserializer.disable_recursion_limit();
    ChunkOptions::deserialize(&mut deserializer)
        .and_then(|options| deserializer.end().map(|()| options))
        .map_err(|_| invalid_projection_input())
}

fn bounded_python_json(py: Python<'_>, input: &Bound<'_, PyAny>) -> PyResult<String> {
    let mut budget = ProjectionInputBudget::default();
    validate_projection_value(input, 0, &mut budget)?;
    let serialized: String = py
        .import("json")?
        .call_method1("dumps", (input,))?
        .extract()
        .map_err(|_| invalid_projection_input())?;
    if serialized.len() > MAX_PROJECTION_JSON_BYTES {
        return Err(projection_too_large());
    }
    Ok(serialized)
}

fn validate_projection_value(
    value: &Bound<'_, PyAny>,
    depth: usize,
    budget: &mut ProjectionInputBudget,
) -> PyResult<()> {
    if depth > MAX_PROJECTION_JSON_DEPTH {
        return Err(projection_too_large());
    }
    if let Ok(object) = value.cast::<PyDict>() {
        budget.add_node(2)?; // braces
        for (index, (key, item)) in object.iter().enumerate() {
            let key = key
                .cast::<PyString>()
                .map_err(|_| invalid_projection_input())?;
            let key = key.to_str().map_err(|_| invalid_projection_input())?;
            budget.add_node(json_string_size_bound(key) + 1 + usize::from(index != 0))?;
            validate_projection_value(&item, depth + 1, budget)?;
        }
    } else if let Ok(sequence) = value.cast::<PyList>() {
        validate_projection_sequence(sequence.iter(), depth, budget)?;
    } else if let Ok(sequence) = value.cast::<PyTuple>() {
        validate_projection_sequence(sequence.iter(), depth, budget)?;
    } else if let Ok(text) = value.cast::<PyString>() {
        budget.add_node(json_string_size_bound(
            text.to_str().map_err(|_| invalid_projection_input())?,
        ))?;
    } else if value.is_instance_of::<PyBool>() || value.is_instance_of::<PyNone>() {
        budget.add_node(
            if value.is_none() || value.extract::<bool>().unwrap_or(false) {
                5
            } else {
                4
            },
        )?;
    } else if let Ok(integer) = value.cast::<PyInt>() {
        if integer.extract::<i64>().is_err() && integer.extract::<u64>().is_err() {
            return Err(invalid_projection_input());
        }
        budget.add_node(20)?;
    } else if let Ok(float) = value.cast::<PyFloat>() {
        if !float.extract::<f64>()?.is_finite() {
            return Err(invalid_projection_input());
        }
        budget.add_node(32)?;
    } else {
        return Err(invalid_projection_input());
    }
    Ok(())
}

fn validate_projection_sequence<'py>(
    items: impl Iterator<Item = Bound<'py, PyAny>>,
    depth: usize,
    budget: &mut ProjectionInputBudget,
) -> PyResult<()> {
    budget.add_node(2)?; // brackets / parentheses
    for (index, item) in items.enumerate() {
        if index != 0 {
            budget.add_bytes(1)?; // comma
        }
        validate_projection_value(&item, depth + 1, budget)?;
    }
    Ok(())
}

fn json_string_size_bound(value: &str) -> usize {
    // json.dumps defaults to ensure_ascii=True: BMP non-ASCII is six bytes and supplementary
    // codepoints become two six-byte surrogate escapes.
    if value.len() > MAX_PROJECTION_JSON_BYTES {
        return MAX_PROJECTION_JSON_BYTES + 1;
    }
    2 + value
        .chars()
        .map(|ch| match ch {
            '"' | '\\' | '\u{08}' | '\u{0c}' | '\n' | '\r' | '\t' => 2,
            ch if ch <= '\u{1f}' || ('\u{80}'..='\u{ffff}').contains(&ch) => 6,
            ch if ch > '\u{ffff}' => 12,
            _ => 1,
        })
        .sum::<usize>()
}

fn render_page(py: Python<'_>, render: &Bound<'_, PyAny>, blocks: &[IrBlock]) -> PyResult<String> {
    let argument = page_blocks_argument(py, blocks)?;
    let result = render.call1((argument,))?;
    let result = result
        .cast::<PyString>()
        .map_err(|_| PyTypeError::new_err("render callback must return str"))?;
    let text = result.to_str()?;
    if text.len() > MAX_PROJECTION_JSON_BYTES {
        return Err(projection_too_large());
    }
    Ok(text.to_owned())
}

fn page_blocks_argument(py: Python<'_>, blocks: &[IrBlock]) -> PyResult<Py<PyAny>> {
    // The input was already preflighted. Rechecking the model serialization here bounds the
    // callback payload while the direct converter below keeps image bytes as one PyBytes object.
    let _bounded_serialized = capped_json_serialization(MAX_PROJECTION_JSON_BYTES, |writer| {
        serde_json::to_writer(writer, blocks)
    })?;
    let mut budget = ProjectionInputBudget::default();
    let frozen = blocks
        .iter()
        .map(|block| block_to_python(py, block, 0, &mut budget))
        .collect::<PyResult<Vec<_>>>()?;
    let frozen = PyTuple::new(py, frozen)?;
    Ok(frozen.into_any().unbind())
}

fn immutable_mapping<'py>(
    py: Python<'py>,
    dict: &Bound<'py, PyDict>,
) -> PyResult<Bound<'py, PyAny>> {
    py.import("types")?
        .getattr("MappingProxyType")?
        .call1((dict,))
}

fn callback_budget(depth: usize, budget: &mut ProjectionInputBudget) -> PyResult<()> {
    if depth > MAX_PROJECTION_JSON_DEPTH {
        return Err(projection_too_large());
    }
    budget.add_node(1)
}

fn block_to_python<'py>(
    py: Python<'py>,
    block: &IrBlock,
    depth: usize,
    budget: &mut ProjectionInputBudget,
) -> PyResult<Bound<'py, PyAny>> {
    callback_budget(depth, budget)?;
    let dict = PyDict::new(py);
    dict.set_item("type", block_kind_name(block.kind))?;
    if let Some(value) = &block.text {
        dict.set_item("text", value)?;
    }
    if let Some(value) = &block.table {
        dict.set_item("table", table_to_python(py, value, depth + 1, budget)?)?;
    }
    if let Some(value) = block.level {
        dict.set_item("level", value)?;
    }
    if let Some(value) = block.page_number {
        dict.set_item("pageNumber", value)?;
    }
    if let Some(value) = &block.bbox {
        dict.set_item("bbox", bbox_to_python(py, value, depth + 1, budget)?)?;
    }
    if let Some(value) = &block.style {
        dict.set_item("style", style_to_python(py, value, depth + 1, budget)?)?;
    }
    if let Some(value) = block.list_type {
        dict.set_item(
            "listType",
            match value {
                ListType::Ordered => "ordered",
                ListType::Unordered => "unordered",
            },
        )?;
    }
    if let Some(value) = &block.children {
        dict.set_item("children", blocks_to_tuple(py, value, depth + 1, budget)?)?;
    }
    if let Some(value) = &block.href {
        dict.set_item("href", value)?;
    }
    if let Some(value) = &block.footnote_text {
        dict.set_item("footnoteText", value)?;
    }
    if let Some(value) = &block.image_data {
        dict.set_item("imageData", image_to_python(py, value, depth + 1, budget)?)?;
    }
    if let Some(value) = &block.spans {
        dict.set_item("spans", spans_to_tuple(py, value, depth + 1, budget)?)?;
    }
    if let Some(value) = block.quote {
        dict.set_item("quote", value)?;
    }
    if let Some(value) = block.indent {
        dict.set_item("indent", value)?;
    }
    if let Some(value) = block.list_depth {
        dict.set_item("listDepth", value)?;
    }
    immutable_mapping(py, &dict)
}

fn block_kind_name(value: IrBlockType) -> &'static str {
    match value {
        IrBlockType::Paragraph => "paragraph",
        IrBlockType::Table => "table",
        IrBlockType::Heading => "heading",
        IrBlockType::List => "list",
        IrBlockType::Image => "image",
        IrBlockType::Separator => "separator",
    }
}

fn blocks_to_tuple<'py>(
    py: Python<'py>,
    blocks: &[IrBlock],
    depth: usize,
    budget: &mut ProjectionInputBudget,
) -> PyResult<Bound<'py, PyAny>> {
    callback_budget(depth, budget)?;
    let values = blocks
        .iter()
        .map(|block| block_to_python(py, block, depth + 1, budget))
        .collect::<PyResult<Vec<_>>>()?;
    Ok(PyTuple::new(py, values)?.into_any())
}

fn table_to_python<'py>(
    py: Python<'py>,
    table: &IrTable,
    depth: usize,
    budget: &mut ProjectionInputBudget,
) -> PyResult<Bound<'py, PyAny>> {
    callback_budget(depth, budget)?;
    let dict = PyDict::new(py);
    dict.set_item("rows", table.rows)?;
    dict.set_item("cols", table.cols)?;
    let rows = table
        .cells
        .iter()
        .map(|row| {
            let cells = row
                .iter()
                .map(|cell| cell_to_python(py, cell, depth + 1, budget))
                .collect::<PyResult<Vec<_>>>()?;
            Ok(PyTuple::new(py, cells)?.into_any())
        })
        .collect::<PyResult<Vec<_>>>()?;
    dict.set_item("cells", PyTuple::new(py, rows)?)?;
    if let Some(value) = table.render_as_table {
        dict.set_item("renderAsTable", value)?;
    }
    dict.set_item("hasHeader", table.has_header)?;
    if let Some(value) = &table.classification {
        dict.set_item(
            "classification",
            classification_to_python(py, value, depth + 1, budget)?,
        )?;
    }
    if let Some(value) = &table.source_id {
        dict.set_item("sourceId", value)?;
    }
    if let Some(value) = &table.regions {
        dict.set_item("regions", bboxes_to_tuple(py, value, depth + 1, budget)?)?;
    }
    if let Some(value) = &table.caption {
        dict.set_item("caption", value)?;
    }
    if let Some(value) = &table.caption_blocks {
        dict.set_item(
            "captionBlocks",
            blocks_to_tuple(py, value, depth + 1, budget)?,
        )?;
    }
    immutable_mapping(py, &dict)
}

fn cell_to_python<'py>(
    py: Python<'py>,
    cell: &IrCell,
    depth: usize,
    budget: &mut ProjectionInputBudget,
) -> PyResult<Bound<'py, PyAny>> {
    callback_budget(depth, budget)?;
    let dict = PyDict::new(py);
    dict.set_item("text", &cell.text)?;
    dict.set_item("colSpan", cell.col_span)?;
    dict.set_item("rowSpan", cell.row_span)?;
    if let Some(value) = &cell.blocks {
        dict.set_item("blocks", blocks_to_tuple(py, value, depth + 1, budget)?)?;
    }
    if let Some(value) = cell.is_header {
        dict.set_item("isHeader", value)?;
    }
    immutable_mapping(py, &dict)
}

fn spans_to_tuple<'py>(
    py: Python<'py>,
    spans: &[IrSpan],
    depth: usize,
    budget: &mut ProjectionInputBudget,
) -> PyResult<Bound<'py, PyAny>> {
    callback_budget(depth, budget)?;
    let values = spans
        .iter()
        .map(|span| {
            callback_budget(depth + 1, budget)?;
            let dict = PyDict::new(py);
            dict.set_item("text", &span.text)?;
            if let Some(value) = span.bold {
                dict.set_item("bold", value)?;
            }
            if let Some(value) = span.italic {
                dict.set_item("italic", value)?;
            }
            if let Some(value) = span.strike {
                dict.set_item("strike", value)?;
            }
            if let Some(value) = span.underline {
                dict.set_item("underline", value)?;
            }
            if let Some(value) = span.code {
                dict.set_item("code", value)?;
            }
            if let Some(value) = span.placeholder {
                dict.set_item("placeholder", value)?;
            }
            immutable_mapping(py, &dict)
        })
        .collect::<PyResult<Vec<_>>>()?;
    Ok(PyTuple::new(py, values)?.into_any())
}

fn image_to_python<'py>(
    py: Python<'py>,
    image: &ImageData,
    depth: usize,
    budget: &mut ProjectionInputBudget,
) -> PyResult<Bound<'py, PyAny>> {
    callback_budget(depth, budget)?;
    let dict = PyDict::new(py);
    dict.set_item("data", PyBytes::new(py, &image.data))?;
    dict.set_item("mimeType", &image.mime_type)?;
    if let Some(value) = &image.filename {
        dict.set_item("filename", value)?;
    }
    immutable_mapping(py, &dict)
}

fn bbox_to_python<'py>(
    py: Python<'py>,
    bbox: &BoundingBox,
    depth: usize,
    budget: &mut ProjectionInputBudget,
) -> PyResult<Bound<'py, PyAny>> {
    callback_budget(depth, budget)?;
    let dict = PyDict::new(py);
    dict.set_item("page", bbox.page)?;
    dict.set_item("x", bbox.x)?;
    dict.set_item("y", bbox.y)?;
    dict.set_item("width", bbox.width)?;
    dict.set_item("height", bbox.height)?;
    immutable_mapping(py, &dict)
}

fn bboxes_to_tuple<'py>(
    py: Python<'py>,
    bboxes: &[BoundingBox],
    depth: usize,
    budget: &mut ProjectionInputBudget,
) -> PyResult<Bound<'py, PyAny>> {
    callback_budget(depth, budget)?;
    let values = bboxes
        .iter()
        .map(|bbox| bbox_to_python(py, bbox, depth + 1, budget))
        .collect::<PyResult<Vec<_>>>()?;
    Ok(PyTuple::new(py, values)?.into_any())
}

fn style_to_python<'py>(
    py: Python<'py>,
    style: &InlineStyle,
    depth: usize,
    budget: &mut ProjectionInputBudget,
) -> PyResult<Bound<'py, PyAny>> {
    callback_budget(depth, budget)?;
    let dict = PyDict::new(py);
    if let Some(value) = style.bold {
        dict.set_item("bold", value)?;
    }
    if let Some(value) = style.italic {
        dict.set_item("italic", value)?;
    }
    if let Some(value) = style.strike {
        dict.set_item("strike", value)?;
    }
    if let Some(value) = style.underline {
        dict.set_item("underline", value)?;
    }
    if let Some(value) = style.font_size {
        dict.set_item("fontSize", value)?;
    }
    if let Some(value) = &style.font_name {
        dict.set_item("fontName", value)?;
    }
    immutable_mapping(py, &dict)
}

fn classification_to_python<'py>(
    py: Python<'py>,
    classification: &TableClassificationSummary,
    depth: usize,
    budget: &mut ProjectionInputBudget,
) -> PyResult<Bound<'py, PyAny>> {
    callback_budget(depth, budget)?;
    let dict = PyDict::new(py);
    let kind = match classification.kind {
        TableClassificationKind::SemanticTable => "semantic-table",
        TableClassificationKind::NonTabularLayout => "non-tabular-layout",
        TableClassificationKind::Uncertain => "uncertain",
    };
    dict.set_item("kind", kind)?;
    dict.set_item("confidence", classification.confidence)?;
    dict.set_item("semanticScore", classification.semantic_score)?;
    dict.set_item("nonTabularScore", classification.non_tabular_score)?;
    let reasons = classification
        .reasons
        .iter()
        .map(|reason| match reason {
            TableClassificationReason::RepeatedRowSchema => "repeated-row-schema",
            TableClassificationReason::GridRegularity => "grid-regularity",
            TableClassificationReason::HighActiveDensity => "high-active-density",
            TableClassificationReason::ColumnTypeConsistency => "column-type-consistency",
            TableClassificationReason::NestedStructureWrapper => "nested-structure-wrapper",
            TableClassificationReason::SpanIrregularity => "span-irregularity",
            TableClassificationReason::SpacerBands => "spacer-bands",
            TableClassificationReason::ExtremeSparsity => "extreme-sparsity",
            TableClassificationReason::DiagramContextKeyword => "diagram-context-keyword",
            TableClassificationReason::LowEvidence => "low-evidence",
            TableClassificationReason::AmbiguousScores => "ambiguous-scores",
        })
        .collect::<Vec<_>>();
    dict.set_item("reasons", PyTuple::new(py, reasons)?)?;
    immutable_mapping(py, &dict)
}

fn core_to_py_error(error: KordocError) -> PyErr {
    py_error(error.code, &error.message)
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
    let serialized = capped_json_serialization(MAX_PROJECTION_JSON_BYTES, |writer| {
        serde_json::to_writer(writer, &value)
    })?;
    json_text_to_python(py, serialized)
}

fn json_text_to_python(py: Python<'_>, serialized: String) -> PyResult<Py<PyAny>> {
    if serialized.len() > MAX_PROJECTION_JSON_BYTES {
        return Err(projection_too_large());
    }
    Ok(py
        .import("json")?
        .call_method1("loads", (serialized,))?
        .unbind())
}

struct CappedJsonWriter {
    bytes: Vec<u8>,
    limit: usize,
    exceeded: bool,
}

impl CappedJsonWriter {
    fn new(limit: usize) -> Self {
        Self {
            bytes: Vec::new(),
            limit,
            exceeded: false,
        }
    }

    fn finish(self, result: Result<(), serde_json::Error>) -> PyResult<String> {
        if self.exceeded {
            return Err(projection_too_large());
        }
        result.map_err(to_py_error)?;
        String::from_utf8(self.bytes).map_err(|_| {
            to_py_error(serde_json::Error::io(io::Error::new(
                io::ErrorKind::InvalidData,
                "serialized JSON was not UTF-8",
            )))
        })
    }
}

impl Write for CappedJsonWriter {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        let Some(next_len) = self.bytes.len().checked_add(bytes.len()) else {
            self.exceeded = true;
            return Err(io::Error::other("JSON output limit exceeded"));
        };
        if next_len > self.limit {
            self.exceeded = true;
            return Err(io::Error::other("JSON output limit exceeded"));
        }
        self.bytes.extend_from_slice(bytes);
        Ok(bytes.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

fn capped_json_serialization(
    limit: usize,
    serialize: impl FnOnce(&mut CappedJsonWriter) -> Result<(), serde_json::Error>,
) -> PyResult<String> {
    let mut writer = CappedJsonWriter::new(limit);
    let result = serialize(&mut writer);
    writer.finish(result)
}

#[pymodule]
fn _native(module: &Bound<'_, PyModule>) -> PyResult<()> {
    module.add_function(wrap_pyfunction!(native_version, module)?)?;
    module.add_function(wrap_pyfunction!(blocks_to_markdown_wire, module)?)?;
    module.add_function(wrap_pyfunction!(blocks_to_pages_wire, module)?)?;
    module.add_function(wrap_pyfunction!(blocks_to_chunks_wire, module)?)?;
    module.add_function(wrap_pyfunction!(classify_tables_wire, module)?)?;
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

#[cfg(test)]
mod projection_tests {
    use super::*;
    use pyo3::exceptions::{PyRuntimeError, PyTypeError};
    use std::ffi::CString;
    use std::sync::Once;

    static PYTHON: Once = Once::new();

    fn with_python<T>(f: impl for<'py> FnOnce(Python<'py>) -> T) -> T {
        PYTHON.call_once(Python::initialize);
        Python::attach(f)
    }

    fn python_value<'py>(py: Python<'py>, source: &str) -> Bound<'py, PyAny> {
        let source = CString::new(source).expect("Python source has no NUL bytes");
        py.eval(&source, None, None)
            .expect("valid Python test value")
    }

    #[test]
    fn blocks_to_markdown_wire_renders_valid_ir_and_rejects_untrusted_shapes() {
        with_python(|py| {
            let blocks = python_value(
                py,
                "[{'type': 'heading', 'level': 2, 'text': 'Title'}, {'type': 'paragraph', 'text': 'Body'}]",
            );
            let rendered = blocks_to_markdown_wire(py, &blocks).unwrap();
            assert_eq!(rendered, "## Title\n\nBody");

            for invalid in [
                "None",
                "True",
                "[None]",
                "[42]",
                "[{'type':'heading','level':True,'text':'x'}]",
                "[{'type':'paragraph','text':None}]",
                "[{'type':'paragraph','text':'x','extra':1}]",
            ] {
                let value = python_value(py, invalid);
                let error = blocks_to_markdown_wire(py, &value).unwrap_err();
                assert!(
                    error.is_instance_of::<PyValueError>(py),
                    "{invalid}: {error}"
                );
                let args = error.value(py).getattr("args").unwrap();
                let args = args.extract::<(String, String)>().unwrap();
                assert_eq!(args.0, ErrorCode::ParseError.as_str(), "{invalid}");
            }
        });
    }

    #[test]
    fn projection_preflight_rejects_deep_input_before_serialization() {
        with_python(|py| {
            let locals = PyDict::new(py);
            py.run(
                c"block = {'type':'paragraph','text':'deep'}\nfor _ in range(300):\n    block = {'type':'paragraph','children':[block]}\nblocks = [block]",
                None,
                Some(&locals),
            )
            .unwrap();
            let blocks = locals.get_item("blocks").unwrap().unwrap();
            let error = blocks_to_markdown_wire(py, &blocks).unwrap_err();
            let args = error.value(py).getattr("args").unwrap();
            let args = args.extract::<(String, String)>().unwrap();
            assert_eq!(args.0, ErrorCode::OutputTooLarge.as_str());
        });
    }

    #[test]
    fn raw_projection_nesting_allows_sixty_four_ir_levels_and_core_rejects_more() {
        with_python(|py| {
            let locals = PyDict::new(py);
            py.run(
                c"block = {'type':'paragraph','text':'deep'}\nfor _ in range(64):\n    block = {'type':'list','text':'item','children':[block]}\nvalid_blocks = [block]\nfor _ in range(1):\n    block = {'type':'list','text':'item','children':[block]}\ntoo_deep_blocks = [block]",
                None,
                Some(&locals),
            )
            .unwrap();
            let valid = locals.get_item("valid_blocks").unwrap().unwrap();
            let valid_result = blocks_to_markdown_wire(py, &valid);
            assert!(
                valid_result.is_ok(),
                "valid depth failed: {:?}",
                valid_result
                    .err()
                    .map(|error| error.value(py).str().unwrap().to_string())
            );

            let too_deep = locals.get_item("too_deep_blocks").unwrap().unwrap();
            let error = blocks_to_markdown_wire(py, &too_deep).unwrap_err();
            let args = error.value(py).getattr("args").unwrap();
            let args = args.extract::<(String, String)>().unwrap();
            assert_eq!(args.0, ErrorCode::OutputTooLarge.as_str());
        });
    }

    #[test]
    fn capped_projection_serializer_never_buffers_over_limit() {
        with_python(|py| {
            let mut writer = CappedJsonWriter::new(4);
            let result = serde_json::to_writer(&mut writer, "serialization payload");
            assert!(result.is_err());
            assert!(writer.bytes.len() <= 4);
            let error = writer.finish(result).unwrap_err();
            let args = error.value(py).getattr("args").unwrap();
            let args = args.extract::<(String, String)>().unwrap();
            assert_eq!(args.0, ErrorCode::OutputTooLarge.as_str());
        });
    }

    #[test]
    fn blocks_to_pages_wire_omits_pages_and_calls_synchronous_renderer_with_frozen_blocks() {
        with_python(|py| {
            let pageless = python_value(py, "[{'type':'paragraph','text':'plain'}]");
            let omitted = blocks_to_pages_wire(py, &pageless, None).unwrap();
            assert!(omitted.is_none(py));

            let blocks = python_value(
                py,
                "[{'type':'paragraph','text':'one','pageNumber':1}, {'type':'paragraph','text':'three','pageNumber':3}]",
            );
            let locals = PyDict::new(py);
            py.run(
                c"def renderer(page):\n    assert type(page) is tuple\n    assert type(page[0]).__name__ == 'mappingproxy'\n    assert page[0]['text'] in ('one', 'three')\n    try:\n        page[0]['text'] = 'mutated'\n    except TypeError:\n        return 'rendered'\n    raise AssertionError('block mapping was mutable')",
                None,
                Some(&locals),
            )
            .unwrap();
            let renderer = locals.get_item("renderer").unwrap().unwrap();
            let pages = blocks_to_pages_wire(py, &blocks, Some(&renderer)).unwrap();
            let json = py
                .import("json")
                .unwrap()
                .call_method1("dumps", (&pages,))
                .unwrap()
                .extract::<String>()
                .unwrap();
            let pages: Value = serde_json::from_str(&json).unwrap();
            assert_eq!(
                pages,
                serde_json::json!([
                    {"pageNumber": 1, "markdown": "rendered"},
                    {"pageNumber": 2, "markdown": ""},
                    {"pageNumber": 3, "markdown": "rendered"}
                ])
            );
        });
    }

    #[test]
    fn blocks_to_pages_wire_propagates_callback_exception_and_rejects_non_string() {
        with_python(|py| {
            let blocks = python_value(py, "[{'type':'paragraph','text':'one','pageNumber':1}]");
            let raises = python_value(
                py,
                "lambda _page: (_ for _ in ()).throw(RuntimeError('callback sentinel'))",
            );
            let error = blocks_to_pages_wire(py, &blocks, Some(&raises)).unwrap_err();
            assert!(error.is_instance_of::<PyRuntimeError>(py));
            assert_eq!(
                error.value(py).str().unwrap().to_str().unwrap(),
                "callback sentinel"
            );

            let non_string = python_value(py, "lambda _page: 12");
            let error = blocks_to_pages_wire(py, &blocks, Some(&non_string)).unwrap_err();
            assert!(error.is_instance_of::<PyTypeError>(py));
        });
    }

    #[test]
    fn page_callback_receives_image_bytes_without_integer_expansion() {
        with_python(|py| {
            let blocks = python_value(
                py,
                "[{'type':'image','pageNumber':1,'imageData':{'data':[0,1,255,128],'mimeType':'image/png'}}]",
            );
            let locals = PyDict::new(py);
            py.run(
                c"def renderer(page):\n    image = page[0]['imageData']\n    assert isinstance(image['data'], bytes)\n    assert image['data'] == bytes([0, 1, 255, 128])\n    return 'image bytes'",
                None,
                Some(&locals),
            )
            .unwrap();
            let renderer = locals.get_item("renderer").unwrap().unwrap();
            let pages = blocks_to_pages_wire(py, &blocks, Some(&renderer)).unwrap();
            let json = py
                .import("json")
                .unwrap()
                .call_method1("dumps", (&pages,))
                .unwrap()
                .extract::<String>()
                .unwrap();
            let pages: Value = serde_json::from_str(&json).unwrap();
            assert_eq!(pages[0]["markdown"], "image bytes");
        });
    }

    #[test]
    fn chunks_and_opt_in_classification_return_camel_case_wire_values() {
        with_python(|py| {
            let blocks = python_value(
                py,
                "[{'type':'heading','level':1,'text':'Section'}, {'type':'paragraph','text':'Body'}, {'type':'table','table':{'rows':1,'cols':1,'hasHeader':False,'cells':[[{'text':'A','colSpan':1,'rowSpan':1}]]}}]",
            );
            let options = python_value(py, "{'granularity':'block','includeTableCells':True}");
            let chunks = blocks_to_chunks_wire(py, &blocks, Some(&options)).unwrap();
            let chunk_json = py
                .import("json")
                .unwrap()
                .call_method1("dumps", (&chunks,))
                .unwrap()
                .extract::<String>()
                .unwrap();
            let chunks: Value = serde_json::from_str(&chunk_json).unwrap();
            assert_eq!(chunks[0]["id"], "c0001");
            assert_eq!(chunks[0]["type"], "heading");
            assert_eq!(chunks[0]["blockRange"], serde_json::json!([0, 0]));
            assert_eq!(chunks[2]["table"]["cells"], serde_json::json!([["A"]]));

            for invalid in [
                "{'unknown': 1}",
                "{'includeTableCells': None}",
                "{'granularity': True}",
            ] {
                let options = python_value(py, invalid);
                let error = blocks_to_chunks_wire(py, &blocks, Some(&options)).unwrap_err();
                assert!(
                    error.is_instance_of::<PyValueError>(py),
                    "{invalid}: {error}"
                );
                let args = error.value(py).getattr("args").unwrap();
                let args = args.extract::<(String, String)>().unwrap();
                assert_eq!(args.0, ErrorCode::ParseError.as_str(), "{invalid}");
            }

            let classified = classify_tables_wire(py, &blocks).unwrap();
            let classified_json = py
                .import("json")
                .unwrap()
                .call_method1("dumps", (&classified,))
                .unwrap()
                .extract::<String>()
                .unwrap();
            let classified: Value = serde_json::from_str(&classified_json).unwrap();
            assert!(classified[2]["table"].get("classification").is_some());
        });
    }
}
