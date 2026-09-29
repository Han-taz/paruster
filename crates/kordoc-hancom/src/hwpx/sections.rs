//! Transactional HWPX section parsing and source-neutral IR lowering.

// H2a's private parser seam is intentionally not wired into crate entry points until H3.
#![allow(dead_code)]

use std::collections::BTreeSet;

use kordoc_ir::{
    ErrorCode, IrBlock, IrBlockType, IrSpan, KordocError, OutlineItem, PageEvidence, PageMode,
    PageSelection, ParseOptions, ParseWarning, WarningCode,
};

use crate::hwpx::styles::StyleCatalog;
use crate::hwpx::xml::{XmlContent, XmlNode, parse};

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct SectionInput {
    pub(crate) path: String,
    pub(crate) bytes: Vec<u8>,
}

impl SectionInput {
    pub(crate) fn new(path: impl Into<String>, bytes: Vec<u8>) -> Self {
        Self {
            path: path.into(),
            bytes,
        }
    }
}

#[derive(Debug, Clone, Default, PartialEq)]
pub(crate) struct SectionOutput {
    pub(crate) blocks: Vec<IrBlock>,
    pub(crate) outline: Vec<OutlineItem>,
    pub(crate) warnings: Vec<ParseWarning>,
    pub(crate) page_mode: Option<PageMode>,
    pub(crate) page_evidence: Vec<PageEvidence>,
}

#[derive(Debug, Default)]
struct SectionDelta {
    blocks: Vec<IrBlock>,
    outline: Vec<(usize, OutlineItem)>,
    layout_positions: Vec<Option<u32>>,
}

/// Orders already-discovered section member paths. A supplied spine is authoritative and must
/// resolve every reference; absent a usable spine, the numeric suffix controls source order.
pub(crate) fn order_section_paths(
    discovered: &[String],
    spine: Option<&[String]>,
) -> Result<Vec<String>, KordocError> {
    if let Some(spine) = spine.filter(|spine| !spine.is_empty()) {
        if spine.iter().any(|path| !discovered.contains(path)) {
            return Err(KordocError::new(
                ErrorCode::Corrupted,
                "HWPX spine references an unavailable section",
            ));
        }
        let mut seen = BTreeSet::new();
        if spine.iter().any(|path| !seen.insert(path.as_str())) {
            return Err(KordocError::new(
                ErrorCode::Corrupted,
                "HWPX spine contains a duplicate section",
            ));
        }
        return Ok(spine.to_vec());
    }

    let mut paths = discovered.to_vec();
    paths.sort_by(
        |left, right| match (section_number(left), section_number(right)) {
            (Some(left_number), Some(right_number)) => left_number.cmp(&right_number),
            (Some(_), None) => std::cmp::Ordering::Less,
            (None, Some(_)) => std::cmp::Ordering::Greater,
            (None, None) => left.cmp(right),
        },
    );
    Ok(paths)
}

fn section_number(path: &str) -> Option<u32> {
    path.strip_prefix("Contents/section")?
        .strip_suffix(".xml")?
        .parse()
        .ok()
}

/// Parses independent sections into temporary deltas. A section's blocks, outline, and page state
/// become visible only after its complete XML and semantic lowering succeeds.
pub(crate) fn lower_sections(
    inputs: &[SectionInput],
    styles: &StyleCatalog,
    layout_cache: Option<&[Vec<u32>]>,
    options: &ParseOptions,
) -> Result<SectionOutput, KordocError> {
    let mut output = SectionOutput::default();
    let mut deltas: Vec<Option<SectionDelta>> = Vec::with_capacity(inputs.len());
    for (index, input) in inputs.iter().enumerate() {
        match parse(&input.bytes) {
            Ok(root) => {
                let mut delta = SectionDelta::default();
                lower_content(
                    &root,
                    styles,
                    options.keep_empty_paragraphs == Some(true),
                    &mut delta,
                );
                deltas.push(Some(delta));
            }
            Err(error) if error.is_resource_limit() => {
                return Err(KordocError::new(
                    ErrorCode::DecompressionBomb,
                    error.message,
                ));
            }
            Err(_) => {
                output.warnings.push(ParseWarning {
                    page: u32::try_from(index + 1).ok(),
                    message: format!("section {} could not be parsed", input.path),
                    code: WarningCode::PartialParse,
                });
                deltas.push(None);
            }
        }
    }

    let xml_layout_cache = derive_xml_layout_cache(&deltas);
    let supplied_layout_usable = layout_cache.is_some_and(|cache| {
        cache.len() == deltas.len()
            && deltas.iter().zip(cache).all(|(delta, pages)| {
                delta.as_ref().is_some_and(|delta| {
                    delta.blocks.len() == pages.len() && pages.iter().all(|page| *page > 0)
                })
            })
    });
    let selected_layout_cache = xml_layout_cache
        .as_deref()
        .or_else(|| supplied_layout_usable.then_some(layout_cache).flatten());
    let layout_usable = selected_layout_cache.is_some();

    let mut pages_seen = BTreeSet::new();
    for (index, delta) in deltas.into_iter().enumerate() {
        let Some(mut delta) = delta else {
            continue;
        };
        let fallback_page = u32::try_from(index + 1).map_err(|_| {
            KordocError::new(
                ErrorCode::DecompressionBomb,
                "section count exceeds page limit",
            )
        })?;
        let cache_pages = selected_layout_cache.map(|cache| &cache[index]);
        for (block_index, block) in delta.blocks.iter_mut().enumerate() {
            let page = cache_pages.map_or(fallback_page, |pages| pages[block_index]);
            assign_page_recursive(block, page);
            pages_seen.insert(page);
        }
        for (block_index, item) in &mut delta.outline {
            item.page_number = delta
                .blocks
                .get(*block_index)
                .and_then(|block| block.page_number);
        }
        output.blocks.extend(delta.blocks);
        output
            .outline
            .extend(delta.outline.into_iter().map(|(_, item)| item));
        if !layout_usable {
            pages_seen.insert(fallback_page);
        }
    }

    output.page_mode = Some(if layout_usable {
        PageMode::Layout
    } else {
        PageMode::Section
    });
    output.page_evidence = pages_seen
        .into_iter()
        .map(|page_number| PageEvidence { page_number })
        .collect();
    apply_page_selection(&mut output, options.pages.as_ref());
    Ok(output)
}

fn apply_page_selection(output: &mut SectionOutput, selection: Option<&PageSelection>) {
    let Some(selection) = selection else { return };
    let selected: BTreeSet<u32> = match selection {
        PageSelection::Numbers(numbers) => numbers
            .iter()
            .filter_map(|number| {
                let value = number.get();
                if value >= 1.0 && value <= u32::MAX as f64 {
                    Some(value.round() as u32)
                } else {
                    None
                }
            })
            .collect(),
        PageSelection::Range(range) => parse_page_range(range).unwrap_or_default(),
    };
    output.blocks.retain(|block| {
        block
            .page_number
            .is_some_and(|page| selected.contains(&page))
    });
    output.outline.retain(|item| {
        item.page_number
            .is_some_and(|page| selected.contains(&page))
    });
    output
        .page_evidence
        .retain(|page| selected.contains(&page.page_number));
}

fn parse_page_range(value: &str) -> Option<BTreeSet<u32>> {
    let (start, end) = value.split_once('-')?;
    let start = start.trim().parse::<u32>().ok()?;
    let end = end.trim().parse::<u32>().ok()?;
    if start == 0 || end < start || end.saturating_sub(start) > 10_000 {
        return None;
    }
    Some((start..=end).collect())
}

fn derive_xml_layout_cache(deltas: &[Option<SectionDelta>]) -> Option<Vec<Vec<u32>>> {
    if deltas.is_empty() {
        return None;
    }
    let mut previous_position = None;
    let mut page = 1u32;
    let mut cache = Vec::with_capacity(deltas.len());
    for delta in deltas {
        let delta = delta.as_ref()?;
        if delta.layout_positions.len() != delta.blocks.len() {
            return None;
        }
        let mut section_pages = Vec::with_capacity(delta.blocks.len());
        for position in &delta.layout_positions {
            let position = (*position)?;
            if previous_position.is_some_and(|previous| position < previous) {
                page = page.checked_add(1)?;
            }
            previous_position = Some(position);
            section_pages.push(page);
        }
        cache.push(section_pages);
    }
    Some(cache)
}

fn lower_content(
    node: &XmlNode,
    styles: &StyleCatalog,
    keep_empty_paragraphs: bool,
    delta: &mut SectionDelta,
) {
    match node.name.as_str() {
        "p" => lower_paragraph(node, styles, keep_empty_paragraphs, delta),
        "footNote" | "endNote" => {
            let text = node.text_content();
            if !text.is_empty() {
                delta.blocks.push(IrBlock {
                    text: Some(text.clone()),
                    footnote_text: Some(text),
                    ..IrBlock::default()
                });
                delta.layout_positions.push(None);
            }
        }
        _ => {
            for part in &node.content {
                if let XmlContent::Child(index) = part {
                    lower_content(&node.children[*index], styles, keep_empty_paragraphs, delta);
                }
            }
        }
    }
}

fn lower_paragraph(
    node: &XmlNode,
    styles: &StyleCatalog,
    keep_empty_paragraphs: bool,
    delta: &mut SectionDelta,
) {
    let level = styles.paragraph_level(node);
    let mut spans = Vec::new();
    let mut notes = Vec::new();
    for part in &node.content {
        match part {
            XmlContent::Text { start, end } => {
                let text = &node.text[*start..*end];
                if !text.trim().is_empty() {
                    spans.push(IrSpan {
                        text: text.to_owned(),
                        ..IrSpan::default()
                    });
                }
            }
            XmlContent::Child(index) => {
                let child = &node.children[*index];
                match child.name.as_str() {
                    "run" => append_inline_content(child, None, styles, &mut spans, &mut notes),
                    "footNote" | "endNote" => push_note(child, &mut notes),
                    "ctrl" => append_inline_content(child, None, styles, &mut spans, &mut notes),
                    _ => {}
                }
            }
        }
    }
    if spans.is_empty() && !keep_empty_paragraphs && notes.is_empty() {
        return;
    }
    let text = spans
        .iter()
        .map(|span| span.text.as_str())
        .collect::<String>();
    let layout_position = paragraph_layout_position(node);
    let block_index = delta.blocks.len();
    let block = IrBlock {
        kind: if level.is_some() {
            IrBlockType::Heading
        } else {
            IrBlockType::Paragraph
        },
        text: Some(text.clone()),
        level,
        footnote_text: (!notes.is_empty()).then(|| notes.join("\n")),
        spans: Some(spans),
        ..IrBlock::default()
    };
    if let Some(level) = level {
        delta.outline.push((
            block_index,
            OutlineItem {
                level,
                text,
                page_number: None,
            },
        ));
    }
    delta.blocks.push(block);
    delta.layout_positions.push(layout_position);
}

fn append_inline_content(
    node: &XmlNode,
    style: Option<&kordoc_ir::InlineStyle>,
    styles: &StyleCatalog,
    spans: &mut Vec<IrSpan>,
    notes: &mut Vec<String>,
) {
    let active_style = if node.name == "run" {
        styles.character_style(node).or(style)
    } else {
        style
    };
    for part in &node.content {
        match part {
            XmlContent::Text { start, end } => {
                let text = &node.text[*start..*end];
                if !text.is_empty() {
                    spans.push(IrSpan {
                        text: text.to_owned(),
                        bold: active_style.and_then(|style| style.bold),
                        italic: active_style.and_then(|style| style.italic),
                        strike: active_style.and_then(|style| style.strike),
                        underline: active_style.and_then(|style| style.underline),
                        ..IrSpan::default()
                    });
                }
            }
            XmlContent::Child(index) => {
                let child = &node.children[*index];
                match child.name.as_str() {
                    "footNote" | "endNote" => push_note(child, notes),
                    _ => append_inline_content(child, active_style, styles, spans, notes),
                }
            }
        }
    }
}

fn push_note(node: &XmlNode, notes: &mut Vec<String>) {
    let text = node.text_content();
    let text = text.trim();
    if !text.is_empty() {
        notes.push(text.to_owned());
    }
}

fn paragraph_layout_position(node: &XmlNode) -> Option<u32> {
    let line_segments = node
        .children
        .iter()
        .find(|child| child.name == "linesegarray")?;
    line_segments
        .descendants("lineseg")
        .into_iter()
        .find_map(|line| line.attr("vertpos").and_then(|value| value.parse().ok()))
}

pub(crate) fn assign_page_recursive(block: &mut IrBlock, page: u32) {
    block.page_number = Some(page);
    if let Some(children) = &mut block.children {
        for child in children {
            assign_page_recursive(child, page);
        }
    }
    if let Some(table) = &mut block.table {
        for row in &mut table.cells {
            for cell in row {
                if let Some(blocks) = &mut cell.blocks {
                    for child in blocks {
                        assign_page_recursive(child, page);
                    }
                }
            }
        }
        if let Some(captions) = &mut table.caption_blocks {
            for caption in captions {
                assign_page_recursive(caption, page);
            }
        }
    }
}

#[cfg(test)]
mod tests;
