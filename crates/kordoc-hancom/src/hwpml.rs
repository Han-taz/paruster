//! Private bounded text-only HWPML parsing.

use std::{borrow::Cow, collections::HashMap};

use kordoc_ir::{
    DocumentMetadata, ErrorCode, IrBlock, IrBlockType, KordocError, OutlineItem, PageSelection,
    ParseOptions, ParsedDocument,
};

use crate::hwpx::{XmlContent, XmlNode, budget::LoweringBudget, parse_xml_critical};

const MAX_HWPML_BYTES: usize = 50 * 1024 * 1024;

pub(crate) fn parse(bytes: &[u8], options: &ParseOptions) -> Result<ParsedDocument, KordocError> {
    let mut budget = LoweringBudget::default();
    parse_with_budget(bytes, options, &mut budget)
}

fn parse_with_budget(
    bytes: &[u8],
    options: &ParseOptions,
    budget: &mut LoweringBudget,
) -> Result<ParsedDocument, KordocError> {
    parse_inner(bytes, options, budget).map_err(normalize_output_error)
}

fn parse_inner(
    bytes: &[u8],
    options: &ParseOptions,
    budget: &mut LoweringBudget,
) -> Result<ParsedDocument, KordocError> {
    check_input_size(bytes.len())?;
    let normalized = normalize_nbsp(bytes, budget)?;
    let root = parse_xml_critical(normalized.as_ref())?;
    if root.name != "HWPML" {
        return Err(KordocError::new(
            ErrorCode::UnsupportedFormat,
            "XML root is not HWPML",
        ));
    }

    let mut parsed = ParsedDocument::default();
    if let Some(summary) = direct_child(&root, "DOCSUMMARY") {
        let mut metadata = DocumentMetadata::default();
        if let Some(title) = direct_child(summary, "TITLE") {
            metadata.title = metadata_value(title, budget)?;
        }
        if let Some(author) = direct_child(summary, "AUTHOR") {
            metadata.author = metadata_value(author, budget)?;
        }
        if let Some(date) = direct_child(summary, "DATE") {
            metadata.created_at = metadata_value(date, budget)?;
        }
        if metadata.title.is_some() || metadata.author.is_some() || metadata.created_at.is_some() {
            parsed.metadata = Some(metadata);
        }
    }

    let Some(body) = direct_child(&root, "BODY") else {
        return Ok(parsed);
    };
    let section_count = body
        .children
        .iter()
        .filter(|child| child.name == "SECTION")
        .count();
    let page_filter = build_page_filter(options.pages.as_ref(), section_count, budget)?;
    let headings = build_heading_map(&root, budget)?;

    let mut section_number = 0usize;
    for section in body.children.iter().filter(|child| child.name == "SECTION") {
        section_number += 1;
        if page_filter
            .as_ref()
            .is_some_and(|selected| !selected[section_number - 1])
        {
            continue;
        }
        let page_number = u32::try_from(section_number).map_err(|_| {
            KordocError::new(
                ErrorCode::DecompressionBomb,
                "HWPML section count exceeds the page number bound",
            )
        })?;
        walk_section(section, page_number, &headings, &mut parsed, budget)?;
    }

    Ok(parsed)
}

fn normalize_output_error(error: KordocError) -> KordocError {
    if error.code == ErrorCode::OutputTooLarge {
        KordocError::new(
            ErrorCode::OutputTooLarge,
            "HWPML lowering exceeds its allocation limit",
        )
    } else {
        error
    }
}

fn check_input_size(bytes: usize) -> Result<(), KordocError> {
    if bytes > MAX_HWPML_BYTES {
        return Err(KordocError::new(
            ErrorCode::DecompressionBomb,
            "HWPML input exceeds the configured byte limit",
        ));
    }
    Ok(())
}

fn normalize_nbsp<'a>(
    bytes: &'a [u8],
    budget: &mut LoweringBudget,
) -> Result<Cow<'a, [u8]>, KordocError> {
    const LEGACY: &[u8] = b"&nbsp;";
    const NUMERIC: &[u8] = b"&#160;";
    if !bytes.windows(LEGACY.len()).any(|window| window == LEGACY) {
        return Ok(Cow::Borrowed(bytes));
    }

    budget.charge_bytes(bytes.len())?;
    let mut normalized = Vec::new();
    normalized
        .try_reserve_exact(bytes.len())
        .map_err(|_| allocation_error())?;
    normalized.extend_from_slice(bytes);
    for start in 0..=normalized.len().saturating_sub(LEGACY.len()) {
        if &normalized[start..start + LEGACY.len()] == LEGACY {
            normalized[start..start + NUMERIC.len()].copy_from_slice(NUMERIC);
        }
    }
    Ok(Cow::Owned(normalized))
}

fn direct_child<'a>(node: &'a XmlNode, name: &str) -> Option<&'a XmlNode> {
    node.children.iter().find(|child| child.name == name)
}

fn metadata_value(
    node: &XmlNode,
    budget: &mut LoweringBudget,
) -> Result<Option<String>, KordocError> {
    let mut text = String::new();
    append_xml_text(node, &mut text, budget)?;
    let trimmed = text.trim();
    if trimmed.is_empty() {
        Ok(None)
    } else {
        budget.copy_str(trimmed).map(Some)
    }
}

fn append_xml_text(
    node: &XmlNode,
    target: &mut String,
    budget: &mut LoweringBudget,
) -> Result<(), KordocError> {
    for content in &node.content {
        match content {
            XmlContent::Text { start, end } => {
                budget.append_str(target, &node.text[*start..*end])?;
            }
            XmlContent::Child(index) => append_xml_text(&node.children[*index], target, budget)?,
        }
    }
    Ok(())
}

fn build_heading_map(
    root: &XmlNode,
    budget: &mut LoweringBudget,
) -> Result<HashMap<String, Option<u32>>, KordocError> {
    let Some(head) = direct_child(root, "HEAD") else {
        return Ok(HashMap::new());
    };
    let Some(mapping_table) = direct_child(head, "MAPPINGTABLE") else {
        return Ok(HashMap::new());
    };
    let Some(shape_list) = direct_child(mapping_table, "PARASHAPELIST") else {
        return Ok(HashMap::new());
    };

    let shape_count = shape_list
        .children
        .iter()
        .filter(|child| child.name == "PARASHAPE")
        .count();
    let entries = shape_count.checked_mul(2).ok_or_else(allocation_error)?;
    budget.charge_items::<(String, Option<u32>)>(entries)?;
    budget.charge_items::<usize>(entries)?;

    let mut map = HashMap::new();
    map.try_reserve(shape_count)
        .map_err(|_| allocation_error())?;
    for shape in shape_list
        .children
        .iter()
        .filter(|child| child.name == "PARASHAPE")
    {
        let id = budget.copy_str(shape.attr("Id").unwrap_or_default())?;
        let level = if shape.attr("HeadingType") == Some("Outline") {
            Some(heading_level(shape.attr("Level").unwrap_or("0")))
        } else {
            None
        };
        map.insert(id, level);
    }
    Ok(map)
}

fn heading_level(source_level: &str) -> u32 {
    let level = parse_integer_prefix(source_level).unwrap_or(0).max(0);
    u32::try_from(level.saturating_add(1).min(6)).unwrap_or(6)
}

fn parse_integer_prefix(value: &str) -> Option<i64> {
    let value = value.trim_start();
    let bytes = value.as_bytes();
    let (negative, start) = match bytes.first() {
        Some(b'-') => (true, 1),
        Some(b'+') => (false, 1),
        _ => (false, 0),
    };
    let mut number = 0i64;
    let mut found = false;
    for byte in bytes
        .get(start..)?
        .iter()
        .copied()
        .take_while(u8::is_ascii_digit)
    {
        found = true;
        number = number
            .saturating_mul(10)
            .saturating_add(i64::from(byte - b'0'));
    }
    if !found {
        None
    } else if negative {
        Some(number.saturating_neg())
    } else {
        Some(number)
    }
}

fn build_page_filter(
    selection: Option<&PageSelection>,
    section_count: usize,
    budget: &mut LoweringBudget,
) -> Result<Option<Vec<bool>>, KordocError> {
    let Some(selection) = selection else {
        return Ok(None);
    };
    budget.charge_items::<bool>(section_count)?;
    let mut selected = Vec::new();
    selected
        .try_reserve_exact(section_count)
        .map_err(|_| allocation_error())?;
    selected.resize(section_count, false);
    match selection {
        PageSelection::Numbers(numbers) => {
            for number in numbers {
                let value = number.get().round();
                if value >= 1.0 && value <= section_count as f64 {
                    selected[value as usize - 1] = true;
                }
            }
        }
        PageSelection::Range(specification) => {
            mark_range_selection(specification, &mut selected);
        }
    }
    Ok(Some(selected))
}

fn mark_range_selection(specification: &str, selected: &mut [bool]) {
    for part in specification
        .split(',')
        .map(str::trim)
        .filter(|part| !part.is_empty())
    {
        if let Some((start, end)) = parse_range_part(part, selected.len()) {
            for page in start..=end {
                selected[page - 1] = true;
            }
        } else if is_range_token(part) {
            // A syntactically valid but reversed/out-of-range range is an empty selection;
            // do not reinterpret its leading digits as a single page.
        } else if let Some(page) = parse_integer_prefix(part)
            && page >= 1
            && usize::try_from(page).is_ok_and(|page| page <= selected.len())
        {
            selected[page as usize - 1] = true;
        }
    }
}

fn is_range_token(part: &str) -> bool {
    let Some((start, end)) = part.split_once('-') else {
        return false;
    };
    !start.trim().is_empty()
        && !end.trim().is_empty()
        && start.trim().bytes().all(|byte| byte.is_ascii_digit())
        && end.trim().bytes().all(|byte| byte.is_ascii_digit())
}

fn parse_range_part(part: &str, max_page: usize) -> Option<(usize, usize)> {
    let (start, end) = part.split_once('-')?;
    let start = parse_unsigned_digits(start.trim())?.max(1);
    let end = parse_unsigned_digits(end.trim())?.min(max_page as u64);
    let start = usize::try_from(start).ok()?;
    let end = usize::try_from(end).ok()?;
    (start <= end && start > 0).then_some((start, end))
}

fn parse_unsigned_digits(value: &str) -> Option<u64> {
    if value.is_empty() || !value.bytes().all(|byte| byte.is_ascii_digit()) {
        return None;
    }
    Some(value.bytes().fold(0u64, |number, byte| {
        number
            .saturating_mul(10)
            .saturating_add(u64::from(byte - b'0'))
    }))
}

fn walk_section(
    node: &XmlNode,
    page_number: u32,
    headings: &HashMap<String, Option<u32>>,
    parsed: &mut ParsedDocument,
    budget: &mut LoweringBudget,
) -> Result<(), KordocError> {
    for content in &node.content {
        let XmlContent::Child(index) = content else {
            continue;
        };
        let child = &node.children[*index];
        match child.name.as_str() {
            "HEADER" | "FOOTER" => continue,
            "P" => add_paragraph(child, page_number, headings, parsed, budget)?,
            "TABLE" => return Err(unsupported_tables()),
            _ => walk_section(child, page_number, headings, parsed, budget)?,
        }
    }
    Ok(())
}

fn add_paragraph(
    node: &XmlNode,
    page_number: u32,
    headings: &HashMap<String, Option<u32>>,
    parsed: &mut ParsedDocument,
    budget: &mut LoweringBudget,
) -> Result<(), KordocError> {
    reject_paragraph_tables(node)?;
    let mut text = String::new();
    append_paragraph_chars(node, &mut text, budget)?;
    let trimmed = text.trim();
    if trimmed.is_empty() {
        return Ok(());
    }

    let shape_id = node.attr("ParaShape").unwrap_or_default();
    let level = headings.get(shape_id).copied().flatten();
    let block_text = budget.copy_str(trimmed)?;
    let outline_text = level.map(|_| budget.copy_str(trimmed)).transpose()?;
    budget.push(
        &mut parsed.blocks,
        IrBlock {
            kind: if level.is_some() {
                IrBlockType::Heading
            } else {
                IrBlockType::Paragraph
            },
            text: Some(block_text),
            level,
            page_number: Some(page_number),
            ..IrBlock::default()
        },
    )?;
    if let (Some(level), Some(text)) = (level, outline_text) {
        budget.push(
            parsed.outline.get_or_insert_with(Vec::new),
            OutlineItem {
                level,
                text,
                page_number: Some(page_number),
            },
        )?;
    }
    Ok(())
}

fn reject_paragraph_tables(node: &XmlNode) -> Result<(), KordocError> {
    for child in &node.children {
        match child.name.as_str() {
            "TABLE" => return Err(unsupported_tables()),
            // Match the selected CHAR traversal: inline headers/footers are visited, while
            // these wrappers suppress all content in the paragraph collector.
            "PICTURE" | "SHAPEOBJECT" | "AUTONUM" => continue,
            _ => reject_paragraph_tables(child)?,
        }
    }
    Ok(())
}

fn append_paragraph_chars(
    node: &XmlNode,
    target: &mut String,
    budget: &mut LoweringBudget,
) -> Result<(), KordocError> {
    for content in &node.content {
        let XmlContent::Child(index) = content else {
            continue;
        };
        let child = &node.children[*index];
        match child.name.as_str() {
            "CHAR" => append_xml_text(child, target, budget)?,
            "TABLE" | "PICTURE" | "SHAPEOBJECT" | "AUTONUM" => continue,
            _ => append_paragraph_chars(child, target, budget)?,
        }
    }
    Ok(())
}

fn unsupported_tables() -> KordocError {
    KordocError::new(
        ErrorCode::UnsupportedFormat,
        "HWPML tables are not supported by the text-only parser",
    )
}

fn allocation_error() -> KordocError {
    KordocError::new(
        ErrorCode::OutputTooLarge,
        "HWPML lowering exceeds its allocation limit",
    )
}

#[cfg(test)]
#[path = "hwpml/tests.rs"]
mod tests;
