//! Transactional HWPX section parsing and source-neutral IR lowering.

// H2a's private parser seam is intentionally not wired into crate entry points until H3.
#![allow(dead_code)]

use std::collections::BTreeSet;

use kordoc_ir::{
    ErrorCode, ExtractedImage, IrBlock, IrBlockType, IrSpan, KordocError, OutlineItem,
    PageEvidence, PageMode, PageSelection, ParseOptions, ParseWarning, WarningCode,
};

use crate::hwpx::budget::LoweringBudget;
use crate::hwpx::images::{ImageCache, image_placeholder, image_reference, resolve_image};
use crate::hwpx::package::Package;
use crate::hwpx::styles::StyleCatalog;
use crate::hwpx::tables::{CellBudget, lower_table_with_assets};
use crate::hwpx::xml::{XmlContent, XmlNode, parse};

const MAX_PAGE_EVIDENCE_ENTRIES: u32 = 100_000;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct SectionInput {
    pub(crate) path: String,
    pub(crate) bytes: Option<Vec<u8>>,
}

impl SectionInput {
    pub(crate) fn new(path: impl Into<String>, bytes: Vec<u8>) -> Self {
        Self {
            path: path.into(),
            bytes: Some(bytes),
        }
    }

    pub(crate) fn unavailable(path: impl Into<String>) -> Self {
        Self {
            path: path.into(),
            bytes: None,
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
    pub(crate) source_page_count: u32,
    pub(crate) usable_sections: usize,
    pub(crate) images: Vec<ExtractedImage>,
}

#[derive(Debug, Default)]
struct SectionDelta {
    blocks: Vec<IrBlock>,
    outline: Vec<(usize, OutlineItem)>,
    layout_positions: Vec<ParagraphLayout>,
    layout_usable: bool,
    multi_column: bool,
    warnings: Vec<ParseWarning>,
}

#[derive(Debug, Clone, Copy)]
struct LinePosition {
    vertical: f64,
    horizontal: f64,
}

#[derive(Debug, Clone, Default)]
struct ParagraphLayout {
    line_positions: Vec<LinePosition>,
    explicit_page_break: bool,
    has_lines: bool,
    block_index: Option<usize>,
    is_paragraph: bool,
    table_page_splits: usize,
}

#[derive(Debug)]
struct XmlLayoutCache {
    section_pages: Vec<Vec<u32>>,
    evidence_pages: BTreeSet<u32>,
}

#[derive(Debug, Clone)]
struct NoteNumberFormat {
    kind: String,
    user_char: String,
    prefix: String,
    suffix: String,
}

#[derive(Debug, Clone, Default)]
struct NoteNumberFormats {
    footnote: Option<NoteNumberFormat>,
    endnote: Option<NoteNumberFormat>,
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
    let mut lowering_budget = LoweringBudget::default();
    lower_sections_impl(
        inputs,
        styles,
        layout_cache,
        options,
        None,
        &mut lowering_budget,
    )
}

pub(crate) fn lower_sections_with_package(
    inputs: &[SectionInput],
    styles: &StyleCatalog,
    layout_cache: Option<&[Vec<u32>]>,
    options: &ParseOptions,
    package: &mut Package<'_>,
) -> Result<SectionOutput, KordocError> {
    let mut lowering_budget = LoweringBudget::default();
    lower_sections_impl(
        inputs,
        styles,
        layout_cache,
        options,
        Some(package),
        &mut lowering_budget,
    )
}

fn lower_sections_impl(
    inputs: &[SectionInput],
    styles: &StyleCatalog,
    layout_cache: Option<&[Vec<u32>]>,
    options: &ParseOptions,
    mut package: Option<&mut Package<'_>>,
    lowering_budget: &mut LoweringBudget,
) -> Result<SectionOutput, KordocError> {
    let mut output = SectionOutput::default();
    lowering_budget.charge_items::<Option<SectionDelta>>(inputs.len())?;
    let mut deltas: Vec<Option<SectionDelta>> = Vec::with_capacity(inputs.len());
    let mut cell_budget = CellBudget::default();
    let mut image_cache = ImageCache::default();
    for (index, input) in inputs.iter().enumerate() {
        let Some(bytes) = input.bytes.as_deref() else {
            deltas.push(None);
            continue;
        };
        match parse(bytes) {
            Ok(root) => {
                let image_checkpoint = image_cache.checkpoint();
                let lowering_checkpoint = lowering_budget.checkpoint();
                let mut section_cell_budget = cell_budget;
                let mut section_images = Vec::new();
                let mut section_warnings = Vec::new();
                let multi_column = find_descendant(&root, "colPr", 8)
                    .and_then(|columns| columns.attr("colCount"))
                    .and_then(|value| value.parse::<f64>().ok())
                    .is_some_and(|count| count > 1.0);
                let paragraph_count = root
                    .children
                    .iter()
                    .filter(|child| child.name == "p")
                    .count();
                lowering_budget.charge_items::<ParagraphLayout>(paragraph_count)?;
                let mut paragraph_layouts = Vec::new();
                paragraph_layouts
                    .try_reserve_exact(paragraph_count)
                    .map_err(|_| crate::hwpx::budget::output_limit())?;
                for paragraph in root.children.iter().filter(|child| child.name == "p") {
                    paragraph_layouts.push(paragraph_layout_position(paragraph, lowering_budget)?);
                }
                let mut delta = SectionDelta {
                    layout_usable: !paragraph_layouts.is_empty()
                        && paragraph_layouts.iter().all(|layout| layout.has_lines),
                    multi_column,
                    ..SectionDelta::default()
                };
                let note_formats = parse_note_number_formats(&root, lowering_budget)?;
                let lowering = lower_content(
                    &root,
                    styles,
                    &note_formats,
                    options,
                    options.keep_empty_paragraphs == Some(true),
                    &mut delta,
                    &mut section_cell_budget,
                    lowering_budget,
                    package.as_deref_mut(),
                    &mut image_cache,
                    &mut section_images,
                    &mut section_warnings,
                );
                match lowering {
                    Ok(()) => {
                        output.usable_sections += 1;
                        cell_budget = section_cell_budget;
                        output.images.extend(section_images);
                        delta.warnings = section_warnings;
                        deltas.push(Some(delta));
                    }
                    Err(error) if error.code == ErrorCode::Corrupted => {
                        image_cache.rollback(image_checkpoint);
                        lowering_budget.rollback(lowering_checkpoint);
                        let warning = ParseWarning {
                            page: u32::try_from(index + 1).ok(),
                            message: section_warning_message(
                                &input.path,
                                "lowered",
                                lowering_budget,
                            )?,
                            code: WarningCode::PartialParse,
                        };
                        lowering_budget.push(&mut output.warnings, warning)?;
                        deltas.push(None);
                    }
                    Err(error) => return Err(error),
                }
            }
            Err(error) if error.is_resource_limit() => {
                return Err(KordocError::new(
                    ErrorCode::DecompressionBomb,
                    error.message,
                ));
            }
            Err(_) => {
                let warning = ParseWarning {
                    page: u32::try_from(index + 1).ok(),
                    message: section_warning_message(&input.path, "parsed", lowering_budget)?,
                    code: WarningCode::PartialParse,
                };
                lowering_budget.push(&mut output.warnings, warning)?;
                deltas.push(None);
            }
        }
    }

    let xml_layout_cache = derive_xml_layout_cache(&deltas, lowering_budget)?;
    let supplied_layout_usable = layout_cache.is_some_and(|cache| {
        cache.len() == deltas.len()
            && deltas.iter().zip(cache).all(|(delta, pages)| {
                delta.as_ref().is_some_and(|delta| {
                    !pages.is_empty()
                        && delta.blocks.len() == pages.len()
                        && pages.iter().all(|page| *page > 0)
                })
            })
    });
    let selected_layout_cache = xml_layout_cache
        .as_ref()
        .map(|cache| cache.section_pages.as_slice())
        .or_else(|| supplied_layout_usable.then_some(layout_cache).flatten());
    let layout_usable = selected_layout_cache.is_some();

    let mut pages_seen = BTreeSet::new();
    if let Some(xml_layout_cache) = &xml_layout_cache {
        for page in xml_layout_cache.evidence_pages.iter().copied() {
            insert_page_evidence(&mut pages_seen, page, lowering_budget)?;
        }
    } else if supplied_layout_usable {
        let max_page = layout_cache
            .into_iter()
            .flatten()
            .flatten()
            .copied()
            .max()
            .unwrap_or(0);
        if max_page > MAX_PAGE_EVIDENCE_ENTRIES {
            return Err(KordocError::new(
                ErrorCode::DecompressionBomb,
                "layout cache page evidence exceeds the page limit",
            ));
        }
        for page in 1..=max_page {
            insert_page_evidence(&mut pages_seen, page, lowering_budget)?;
        }
    }
    for (index, delta) in deltas.into_iter().enumerate() {
        let fallback_page = u32::try_from(index + 1).map_err(|_| {
            KordocError::new(
                ErrorCode::DecompressionBomb,
                "section count exceeds page limit",
            )
        })?;
        if !layout_usable {
            insert_page_evidence(&mut pages_seen, fallback_page, lowering_budget)?;
        }
        let Some(mut delta) = delta else {
            continue;
        };
        let cache_pages = selected_layout_cache.map(|cache| &cache[index]);
        for (block_index, block) in delta.blocks.iter_mut().enumerate() {
            let page = cache_pages.map_or(fallback_page, |pages| pages[block_index]);
            assign_page_recursive(block, page);
            insert_page_evidence(&mut pages_seen, page, lowering_budget)?;
        }
        for (block_index, item) in &mut delta.outline {
            item.page_number = delta
                .blocks
                .get(*block_index)
                .and_then(|block| block.page_number);
        }
        for warning in &mut delta.warnings {
            if warning.code == WarningCode::SkippedImage
                && warning.page.is_none()
                && let Some(reference) = warning
                    .message
                    .strip_prefix("Optional HWPX image is missing or unsupported: ")
            {
                let mut marker = String::new();
                lowering_budget.append_str(&mut marker, "[Image: ")?;
                lowering_budget.append_str(&mut marker, reference)?;
                lowering_budget.append_str(&mut marker, "]")?;
                warning.page = find_block_page(&delta.blocks, &marker);
            }
        }
        lowering_budget.charge_items::<ParseWarning>(delta.warnings.len())?;
        output.warnings.extend(delta.warnings);
        lowering_budget.charge_items::<IrBlock>(delta.blocks.len())?;
        output.blocks.extend(delta.blocks);
        lowering_budget.charge_items::<OutlineItem>(delta.outline.len())?;
        output
            .outline
            .extend(delta.outline.into_iter().map(|(_, item)| item));
    }

    output.page_mode = Some(if layout_usable {
        PageMode::Layout
    } else {
        PageMode::Section
    });
    lowering_budget.charge_items::<PageEvidence>(pages_seen.len())?;
    output.page_evidence = pages_seen
        .into_iter()
        .map(|page_number| PageEvidence { page_number })
        .collect();
    output.source_page_count = u32::try_from(output.page_evidence.len()).map_err(|_| {
        KordocError::new(
            ErrorCode::DecompressionBomb,
            "HWPX page evidence exceeds its bound",
        )
    })?;
    apply_page_selection(&mut output, options.pages.as_ref(), lowering_budget)?;
    Ok(output)
}

fn find_block_page(blocks: &[IrBlock], text: &str) -> Option<u32> {
    for block in blocks {
        if block.text.as_deref() == Some(text) && block.page_number.is_some() {
            return block.page_number;
        }
        if let Some(children) = &block.children
            && let Some(page) = find_block_page(children, text)
        {
            return Some(page);
        }
        if let Some(table) = &block.table {
            for cell in table.cells.iter().flatten() {
                if let Some(page) = cell
                    .blocks
                    .as_deref()
                    .and_then(|nested| find_block_page(nested, text))
                {
                    return Some(page);
                }
            }
            if let Some(page) = table
                .caption_blocks
                .as_deref()
                .and_then(|nested| find_block_page(nested, text))
            {
                return Some(page);
            }
        }
    }
    None
}

fn insert_page_evidence(
    pages: &mut BTreeSet<u32>,
    page: u32,
    budget: &mut LoweringBudget,
) -> Result<(), KordocError> {
    if !pages.contains(&page) {
        if pages.len() >= MAX_PAGE_EVIDENCE_ENTRIES as usize {
            return Err(KordocError::new(
                ErrorCode::OutputTooLarge,
                "HWPX page evidence exceeds its allocation limit",
            ));
        }
        budget.charge_bytes(std::mem::size_of::<u32>() + 3 * std::mem::size_of::<usize>())?;
        pages.insert(page);
    }
    Ok(())
}

fn section_warning_message(
    path: &str,
    action: &str,
    budget: &mut LoweringBudget,
) -> Result<String, KordocError> {
    let mut message = String::new();
    budget.append_str(&mut message, "section ")?;
    budget.append_str(&mut message, path)?;
    budget.append_str(&mut message, " could not be ")?;
    budget.append_str(&mut message, action)?;
    Ok(message)
}

/// Resolves only image placeholders retained by source-page selection. This keeps excluded-page
/// package members outside both extraction and image-output accounting.
pub(crate) fn resolve_selected_images(
    output: &mut SectionOutput,
    package: &mut Package<'_>,
) -> Result<ImageCache, KordocError> {
    fn visit(
        blocks: &mut [IrBlock],
        package: &mut Package<'_>,
        cache: &mut ImageCache,
        images: &mut Vec<ExtractedImage>,
        warnings: &mut Vec<ParseWarning>,
    ) -> Result<(), KordocError> {
        for block in blocks {
            if block.kind == IrBlockType::Image && block.image_data.is_none() {
                let reference = block.text.as_deref().ok_or_else(|| {
                    KordocError::new(
                        ErrorCode::Corrupted,
                        "HWPX image placeholder has no reference",
                    )
                })?;
                let page = block.page_number;
                let mut resolved =
                    resolve_image(reference, page, package, cache, images, warnings)?;
                resolved.page_number = page;
                *block = resolved;
            }
            if let Some(children) = &mut block.children {
                visit(children, package, cache, images, warnings)?;
            }
            if let Some(table) = &mut block.table {
                for cell in table.cells.iter_mut().flatten() {
                    if let Some(children) = &mut cell.blocks {
                        visit(children, package, cache, images, warnings)?;
                    }
                }
                if let Some(children) = &mut table.caption_blocks {
                    visit(children, package, cache, images, warnings)?;
                }
            }
        }
        Ok(())
    }
    let mut cache = ImageCache::default();
    visit(
        &mut output.blocks,
        package,
        &mut cache,
        &mut output.images,
        &mut output.warnings,
    )?;
    Ok(cache)
}

fn apply_page_selection(
    output: &mut SectionOutput,
    selection: Option<&PageSelection>,
    budget: &mut LoweringBudget,
) -> Result<(), KordocError> {
    let Some(selection) = selection else {
        return Ok(());
    };
    let mut selected = BTreeSet::new();
    match selection {
        PageSelection::Numbers(numbers) => {
            for number in numbers {
                let value = number.get();
                if value >= 1.0 && value <= u32::MAX as f64 {
                    insert_page_evidence(&mut selected, value.round() as u32, budget)?;
                }
            }
        }
        PageSelection::Range(range) => {
            if let Some((start, end)) = parse_page_range(range) {
                for page in start..=end {
                    insert_page_evidence(&mut selected, page, budget)?;
                }
            }
        }
    }
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
    Ok(())
}

fn parse_page_range(value: &str) -> Option<(u32, u32)> {
    let (start, end) = value.split_once('-')?;
    let start = start.trim().parse::<u32>().ok()?;
    let end = end.trim().parse::<u32>().ok()?;
    if start == 0 || end < start || end.saturating_sub(start) > 10_000 {
        return None;
    }
    Some((start, end))
}

fn derive_xml_layout_cache(
    deltas: &[Option<SectionDelta>],
    budget: &mut LoweringBudget,
) -> Result<Option<XmlLayoutCache>, KordocError> {
    if deltas.is_empty() {
        return Ok(None);
    }
    budget.charge_items::<Vec<u32>>(deltas.len())?;
    let mut cache = Vec::with_capacity(deltas.len());
    let mut evidence_pages = BTreeSet::new();
    let mut last_page = 0u32;
    let mut saw_layout_hint = false;
    for delta in deltas {
        let Some(delta) = delta.as_ref() else {
            return Ok(None);
        };
        if !delta.layout_usable
            || delta
                .layout_positions
                .iter()
                .filter(|layout| layout.block_index.is_some())
                .count()
                != delta.blocks.len()
        {
            return Ok(None);
        }
        let Some(mut page) = last_page.checked_add(1) else {
            return Ok(None);
        };
        insert_page_evidence(&mut evidence_pages, page, budget)?;
        let mut previous_position = None;
        let mut previous_horizontal = None;
        budget.charge_items::<u32>(delta.blocks.len())?;
        let mut section_pages = vec![page; delta.blocks.len()];
        let mut saw_paragraph = false;
        let mut suppress_midpage_reset = false;
        for layout in &delta.layout_positions {
            let explicit_break = layout.explicit_page_break && saw_paragraph;
            if explicit_break {
                let Some(next) = page.checked_add(1) else {
                    return Ok(None);
                };
                page = next;
                insert_page_evidence(&mut evidence_pages, page, budget)?;
                suppress_midpage_reset = false;
            }
            let mut first_line = true;
            let mut broke_by_explicit = explicit_break;
            for position in &layout.line_positions {
                let line_reset = previous_position.is_some_and(|previous| {
                    if position.vertical < previous {
                        !delta.multi_column
                            || previous_horizontal
                                .is_none_or(|horizontal| position.horizontal <= horizontal)
                    } else {
                        first_line
                            && position.vertical == previous
                            && previous_horizontal
                                .is_none_or(|horizontal| position.horizontal <= horizontal)
                    }
                });
                let suppressed =
                    first_line && suppress_midpage_reset && position.vertical >= 2000.0;
                if line_reset && !(first_line && broke_by_explicit) && !suppressed {
                    let Some(next) = page.checked_add(1) else {
                        return Ok(None);
                    };
                    page = next;
                    insert_page_evidence(&mut evidence_pages, page, budget)?;
                }
                if first_line {
                    suppress_midpage_reset = false;
                    if let Some(block_index) = layout.block_index {
                        section_pages[block_index] = page;
                        insert_page_evidence(&mut evidence_pages, page, budget)?;
                    }
                    broke_by_explicit = false;
                }
                previous_position = Some(position.vertical);
                previous_horizontal = Some(position.horizontal);
                first_line = false;
            }
            if first_line && let Some(block_index) = layout.block_index {
                section_pages[block_index] = page;
                insert_page_evidence(&mut evidence_pages, page, budget)?;
            }
            saw_layout_hint |= explicit_break || !layout.line_positions.is_empty();
            if let Some(block_index) = layout.block_index {
                insert_page_evidence(&mut evidence_pages, section_pages[block_index], budget)?;
            }
            saw_paragraph |= layout.is_paragraph;
            if layout.table_page_splits > 0 {
                let Some(increment) = u32::try_from(layout.table_page_splits).ok() else {
                    return Ok(None);
                };
                let Some(next) = page.checked_add(increment) else {
                    return Ok(None);
                };
                page = next;
                insert_page_evidence(&mut evidence_pages, page, budget)?;
                suppress_midpage_reset = true;
            }
        }
        if !delta.blocks.is_empty() {
            insert_page_evidence(&mut evidence_pages, page, budget)?;
        }
        last_page = page;
        cache.push(section_pages);
    }
    Ok(saw_layout_hint.then_some(XmlLayoutCache {
        section_pages: cache,
        evidence_pages,
    }))
}

#[allow(clippy::too_many_arguments)]
fn lower_content(
    node: &XmlNode,
    styles: &StyleCatalog,
    note_formats: &NoteNumberFormats,
    options: &ParseOptions,
    keep_empty_paragraphs: bool,
    delta: &mut SectionDelta,
    cell_budget: &mut CellBudget,
    lowering_budget: &mut LoweringBudget,
    package: Option<&mut Package<'_>>,
    image_cache: &mut ImageCache,
    images: &mut Vec<ExtractedImage>,
    warnings: &mut Vec<ParseWarning>,
) -> Result<(), KordocError> {
    let mut package = package;
    match node.name.as_str() {
        "p" => lower_paragraph(
            node,
            styles,
            note_formats,
            options,
            keep_empty_paragraphs,
            delta,
            cell_budget,
            lowering_budget,
            package.as_deref_mut(),
            image_cache,
            images,
            warnings,
        ),
        "pic" | "img" | "imgRect" | "imgClip" => {
            if let (Some(reference), Some(package)) =
                (image_reference(node), package.as_deref_mut())
            {
                let block = resolve_image(reference, None, package, image_cache, images, warnings)?;
                let block_index = delta.blocks.len();
                lowering_budget.push(&mut delta.blocks, block)?;
                lowering_budget.push(
                    &mut delta.layout_positions,
                    ParagraphLayout {
                        block_index: Some(block_index),
                        ..ParagraphLayout::default()
                    },
                )?;
            }
            Ok(())
        }
        "tbl" => {
            let block_index = delta.blocks.len();
            let block = lower_table_with_assets(
                node,
                0,
                cell_budget,
                lowering_budget,
                options,
                package.as_deref_mut(),
                image_cache,
                images,
                warnings,
            )?;
            lowering_budget.push(&mut delta.blocks, block)?;
            lowering_budget.push(
                &mut delta.layout_positions,
                ParagraphLayout {
                    block_index: Some(block_index),
                    ..ParagraphLayout::default()
                },
            )?;
            Ok(())
        }
        "footNote" | "endNote" => {
            let text = lowering_budget.raw_xml_text(node)?;
            if !text.is_empty() {
                let block_index = delta.blocks.len();
                let duplicate = lowering_budget.copy_str(&text)?;
                lowering_budget.push(
                    &mut delta.blocks,
                    IrBlock {
                        text: Some(duplicate),
                        footnote_text: Some(text),
                        ..IrBlock::default()
                    },
                )?;
                lowering_budget.push(
                    &mut delta.layout_positions,
                    ParagraphLayout {
                        block_index: Some(block_index),
                        ..ParagraphLayout::default()
                    },
                )?;
            }
            Ok(())
        }
        _ => {
            for part in &node.content {
                if let XmlContent::Child(index) = part {
                    lower_content(
                        &node.children[*index],
                        styles,
                        note_formats,
                        options,
                        keep_empty_paragraphs,
                        delta,
                        cell_budget,
                        lowering_budget,
                        package.as_deref_mut(),
                        image_cache,
                        images,
                        warnings,
                    )?;
                }
            }
            Ok(())
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn lower_paragraph(
    node: &XmlNode,
    styles: &StyleCatalog,
    note_formats: &NoteNumberFormats,
    options: &ParseOptions,
    keep_empty_paragraphs: bool,
    delta: &mut SectionDelta,
    cell_budget: &mut CellBudget,
    lowering_budget: &mut LoweringBudget,
    mut package: Option<&mut Package<'_>>,
    image_cache: &mut ImageCache,
    images: &mut Vec<ExtractedImage>,
    warnings: &mut Vec<ParseWarning>,
) -> Result<(), KordocError> {
    let level = styles.paragraph_level(node);
    let mark_placeholder_fields =
        options.include_field_placeholders != Some(true) && !paragraph_contains_inline_table(node);
    let mut notes = Vec::new();
    let mut parts = Vec::new();
    let mut spans = Vec::new();
    collect_paragraph_parts(
        node,
        None,
        styles,
        note_formats,
        &mut spans,
        &mut notes,
        &mut parts,
        &mut Vec::new(),
        mark_placeholder_fields,
        lowering_budget,
    )?;
    if !spans.is_empty() {
        lowering_budget.push(&mut parts, ParagraphPart::Text(std::mem::take(&mut spans)))?;
    }
    if keep_empty_paragraphs && parts.is_empty() {
        lowering_budget.push(&mut parts, ParagraphPart::Text(Vec::new()))?;
    }
    let has_text = parts
        .iter()
        .any(|part| matches!(part, ParagraphPart::Text(spans) if !spans.is_empty()));
    if !has_text && !keep_empty_paragraphs && notes.is_empty() && parts.is_empty() {
        let layout = paragraph_layout_position(node, lowering_budget)?;
        lowering_budget.push(&mut delta.layout_positions, layout)?;
        return Ok(());
    }
    let layout_position = paragraph_layout_position(node, lowering_budget)?;
    let mut host_layout_pending = true;
    let mut first_text = true;
    for part in parts {
        match part {
            ParagraphPart::Text(spans) => {
                let mut text = String::new();
                for span in &spans {
                    lowering_budget.append_str(&mut text, &span.text)?;
                }
                if text.is_empty() && !keep_empty_paragraphs {
                    continue;
                }
                let block_index = delta.blocks.len();
                let heading = first_text.then_some(level).flatten();
                let block_text = lowering_budget.copy_str(&text)?;
                let footnote_text = if first_text && !notes.is_empty() {
                    Some(lowering_budget.join_strings(&notes, "\n")?)
                } else {
                    None
                };
                let keep_spans = spans.iter().any(|span| {
                    span.bold.is_some()
                        || span.italic.is_some()
                        || span.strike.is_some()
                        || span.underline.is_some()
                }) || !notes.is_empty()
                    || spans.iter().any(|span| span.placeholder == Some(true));
                let block = IrBlock {
                    kind: if heading.is_some() {
                        IrBlockType::Heading
                    } else {
                        IrBlockType::Paragraph
                    },
                    text: Some(block_text),
                    level: heading,
                    footnote_text,
                    spans: keep_spans.then_some(spans),
                    ..IrBlock::default()
                };
                if let Some(level) = heading {
                    lowering_budget.push(
                        &mut delta.outline,
                        (
                            block_index,
                            OutlineItem {
                                level,
                                text,
                                page_number: None,
                            },
                        ),
                    )?;
                }
                lowering_budget.push(&mut delta.blocks, block)?;
                let mut layout = if host_layout_pending {
                    ParagraphLayout {
                        block_index: Some(block_index),
                        ..clone_layout_position(&layout_position, lowering_budget)?
                    }
                } else {
                    ParagraphLayout {
                        block_index: Some(block_index),
                        ..ParagraphLayout::default()
                    }
                };
                layout.block_index = Some(block_index);
                lowering_budget.push(&mut delta.layout_positions, layout)?;
                host_layout_pending = false;
                first_text = false;
            }
            ParagraphPart::Table(table_node) => {
                let block_index = delta.blocks.len();
                let block = lower_table_with_assets(
                    table_node,
                    0,
                    cell_budget,
                    lowering_budget,
                    options,
                    package.as_deref_mut(),
                    image_cache,
                    images,
                    warnings,
                )?;
                lowering_budget.push(&mut delta.blocks, block)?;
                let layout = ParagraphLayout {
                    block_index: Some(block_index),
                    table_page_splits: table_intra_breaks(table_node, lowering_budget)?,
                    ..if host_layout_pending {
                        clone_layout_position(&layout_position, lowering_budget)?
                    } else {
                        ParagraphLayout::default()
                    }
                };
                lowering_budget.push(&mut delta.layout_positions, layout)?;
                host_layout_pending = false;
            }
            ParagraphPart::Image(reference) => {
                let block_index = delta.blocks.len();
                let reference = lowering_budget.copy_str(reference)?;
                let block = if let Some(package) = package.as_deref_mut() {
                    resolve_image(&reference, None, package, image_cache, images, warnings)?
                } else {
                    image_placeholder(reference)
                };
                lowering_budget.push(&mut delta.blocks, block)?;
                let layout = if host_layout_pending {
                    ParagraphLayout {
                        block_index: Some(block_index),
                        ..clone_layout_position(&layout_position, lowering_budget)?
                    }
                } else {
                    ParagraphLayout {
                        block_index: Some(block_index),
                        ..ParagraphLayout::default()
                    }
                };
                lowering_budget.push(&mut delta.layout_positions, layout)?;
                host_layout_pending = false;
            }
        }
    }
    if host_layout_pending {
        lowering_budget.push(&mut delta.layout_positions, layout_position)?;
    }
    Ok(())
}

enum ParagraphPart<'a> {
    Text(Vec<IrSpan>),
    Table(&'a XmlNode),
    Image(&'a str),
}

struct OpenField {
    guide: Option<String>,
    part_start: usize,
    span_start: usize,
}

pub(crate) fn click_here_guide(
    field: &XmlNode,
    budget: &mut LoweringBudget,
) -> Result<Option<String>, KordocError> {
    if !field
        .attr("type")
        .is_some_and(|kind| kind.eq_ignore_ascii_case("CLICK_HERE"))
        || field.attr("dirty") == Some("1")
    {
        return Ok(None);
    }
    let Some(parameters) = field.children.iter().find(|node| node.name == "parameters") else {
        return Ok(None);
    };
    let mut command_guide = None;
    for parameter in parameters
        .children
        .iter()
        .filter(|node| node.name == "stringParam")
    {
        let value = budget.raw_xml_text(parameter)?;
        match parameter.attr("name") {
            Some("Direction") => {
                return if value.is_empty() {
                    Ok(None)
                } else {
                    budget.copy_str(&value).map(Some)
                };
            }
            Some("Command") => {
                if let Some((_, remainder)) = value.split_once("Direction:wstring:")
                    && let Some((length, text)) = remainder.split_once(':')
                    && let Ok(length) = length.parse::<usize>()
                {
                    command_guide = match utf16_prefix(text, length) {
                        Some(guide) if !guide.is_empty() => Some(budget.copy_str(guide)?),
                        _ => None,
                    };
                }
            }
            _ => {}
        }
    }
    Ok(command_guide)
}

fn utf16_prefix(value: &str, units: usize) -> Option<&str> {
    if units == 0 {
        return Some("");
    }
    let mut consumed = 0usize;
    for (start, character) in value.char_indices() {
        let next = consumed.checked_add(character.len_utf16())?;
        if units < next {
            // JavaScript would return a lone surrogate here, which Rust's UTF-8 String cannot hold.
            return None;
        }
        consumed = next;
        if consumed == units {
            return Some(&value[..start + character.len_utf8()]);
        }
    }
    Some(value)
}

pub(crate) fn paragraph_contains_inline_table(node: &XmlNode) -> bool {
    fn visit(node: &XmlNode) -> bool {
        node.children.iter().any(|child| match child.name.as_str() {
            "tbl" => child.children.iter().any(|position| {
                position.name == "pos" && position.attr("treatAsChar") == Some("1")
            }),
            // These are consumed as inline data or field metadata, not emitted as table parts.
            "footNote" | "endNote" | "fieldBegin" => false,
            _ => visit(child),
        })
    }
    visit(node)
}

pub(crate) fn append_field_comparison_text(
    target: &mut String,
    source: &str,
    budget: &mut LoweringBudget,
) -> Result<(), KordocError> {
    let mut cursor = 0;
    for (start, _) in source.match_indices("\\$") {
        budget.append_str(target, &source[cursor..start])?;
        budget.append_str(target, "$")?;
        cursor = start + 2;
    }
    budget.append_str(target, &source[cursor..])
}

#[allow(clippy::too_many_arguments)]
fn collect_paragraph_parts<'a>(
    node: &'a XmlNode,
    style: Option<&kordoc_ir::InlineStyle>,
    styles: &StyleCatalog,
    note_formats: &NoteNumberFormats,
    spans: &mut Vec<IrSpan>,
    notes: &mut Vec<String>,
    parts: &mut Vec<ParagraphPart<'a>>,
    fields: &mut Vec<OpenField>,
    mark_placeholder_fields: bool,
    budget: &mut LoweringBudget,
) -> Result<(), KordocError> {
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
                    let owned_text = budget.copy_str(text)?;
                    budget.push(
                        spans,
                        IrSpan {
                            text: owned_text,
                            bold: active_style.and_then(|s| s.bold),
                            italic: active_style.and_then(|s| s.italic),
                            strike: active_style.and_then(|s| s.strike),
                            underline: active_style.and_then(|s| s.underline),
                            ..IrSpan::default()
                        },
                    )?;
                }
            }
            XmlContent::Child(index) => {
                let child = &node.children[*index];
                match child.name.as_str() {
                    "fieldBegin" => {
                        if mark_placeholder_fields {
                            budget.charge_items::<OpenField>(1)?;
                            let guide = click_here_guide(child, budget)?;
                            budget.push(
                                fields,
                                OpenField {
                                    guide,
                                    part_start: parts.len(),
                                    span_start: spans.len(),
                                },
                            )?;
                        }
                    }
                    "fieldEnd" => {
                        if mark_placeholder_fields
                            && let Some(field) = fields.pop()
                            && let Some(guide) = field.guide
                        {
                            let mut value = String::new();
                            let mut contains_nested_placeholder = false;
                            let mut first_text_part = true;
                            for part in parts.iter().skip(field.part_start) {
                                if let ParagraphPart::Text(prior_spans) = part {
                                    let skip = if first_text_part { field.span_start } else { 0 };
                                    first_text_part = false;
                                    for span in prior_spans.iter().skip(skip) {
                                        contains_nested_placeholder |=
                                            span.placeholder == Some(true);
                                        append_field_comparison_text(
                                            &mut value, &span.text, budget,
                                        )?;
                                    }
                                }
                            }
                            let current_span_start =
                                if first_text_part { field.span_start } else { 0 };
                            for span in spans.iter().skip(current_span_start) {
                                contains_nested_placeholder |= span.placeholder == Some(true);
                                append_field_comparison_text(&mut value, &span.text, budget)?;
                            }
                            if !contains_nested_placeholder
                                && !value.is_empty()
                                && (value == guide || value.trim_end() == guide)
                            {
                                first_text_part = true;
                                for part in parts.iter_mut().skip(field.part_start) {
                                    if let ParagraphPart::Text(prior_spans) = part {
                                        let skip =
                                            if first_text_part { field.span_start } else { 0 };
                                        first_text_part = false;
                                        for span in prior_spans.iter_mut().skip(skip) {
                                            span.placeholder = Some(true);
                                        }
                                    }
                                }
                                let current_span_start =
                                    if first_text_part { field.span_start } else { 0 };
                                for span in &mut spans[current_span_start..] {
                                    span.placeholder = Some(true);
                                }
                            }
                        }
                    }
                    "tbl" => {
                        if !spans.is_empty() {
                            budget.push(parts, ParagraphPart::Text(std::mem::take(spans)))?;
                        }
                        budget.push(parts, ParagraphPart::Table(child))?;
                    }
                    "pic" | "img" | "imgRect" | "imgClip" => {
                        if let Some(reference) = image_reference(child) {
                            if !spans.is_empty() {
                                budget.push(parts, ParagraphPart::Text(std::mem::take(spans)))?;
                            }
                            budget.push(parts, ParagraphPart::Image(reference))?;
                        }
                    }
                    "footNote" | "endNote" => {
                        append_note(child, active_style, note_formats, spans, notes, budget)?
                    }
                    _ => collect_paragraph_parts(
                        child,
                        active_style,
                        styles,
                        note_formats,
                        spans,
                        notes,
                        parts,
                        fields,
                        mark_placeholder_fields,
                        budget,
                    )?,
                }
            }
        }
    }
    Ok(())
}

fn append_inline_content(
    node: &XmlNode,
    style: Option<&kordoc_ir::InlineStyle>,
    styles: &StyleCatalog,
    note_formats: &NoteNumberFormats,
    spans: &mut Vec<IrSpan>,
    notes: &mut Vec<String>,
    budget: &mut LoweringBudget,
) -> Result<(), KordocError> {
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
                    let text = budget.copy_str(text)?;
                    budget.push(
                        spans,
                        IrSpan {
                            text,
                            bold: active_style.and_then(|style| style.bold),
                            italic: active_style.and_then(|style| style.italic),
                            strike: active_style.and_then(|style| style.strike),
                            underline: active_style.and_then(|style| style.underline),
                            ..IrSpan::default()
                        },
                    )?;
                }
            }
            XmlContent::Child(index) => {
                let child = &node.children[*index];
                match child.name.as_str() {
                    "footNote" | "endNote" => {
                        append_note(child, active_style, note_formats, spans, notes, budget)?
                    }
                    _ => append_inline_content(
                        child,
                        active_style,
                        styles,
                        note_formats,
                        spans,
                        notes,
                        budget,
                    )?,
                }
            }
        }
    }
    Ok(())
}

fn append_note(
    node: &XmlNode,
    style: Option<&kordoc_ir::InlineStyle>,
    note_formats: &NoteNumberFormats,
    spans: &mut Vec<IrSpan>,
    notes: &mut Vec<String>,
    budget: &mut LoweringBudget,
) -> Result<(), KordocError> {
    let mark = note_reference_mark(node, note_formats, budget)?;
    budget.push(
        spans,
        IrSpan {
            text: mark,
            bold: style.and_then(|style| style.bold),
            italic: style.and_then(|style| style.italic),
            strike: style.and_then(|style| style.strike),
            underline: style.and_then(|style| style.underline),
            ..IrSpan::default()
        },
    )?;
    let text = budget.raw_xml_text(node)?;
    let text = text.trim();
    if !text.is_empty() {
        let text = budget.copy_str(text)?;
        budget.push(notes, text)?;
    }
    Ok(())
}

fn parse_note_number_formats(
    root: &XmlNode,
    budget: &mut LoweringBudget,
) -> Result<NoteNumberFormats, KordocError> {
    fn read(
        root: &XmlNode,
        property: &str,
        budget: &mut LoweringBudget,
    ) -> Result<Option<NoteNumberFormat>, KordocError> {
        let Some(properties) = find_descendant(root, property, 200) else {
            return Ok(None);
        };
        let Some(format) = properties
            .children
            .iter()
            .find(|child| child.name == "autoNumFormat")
        else {
            return Ok(None);
        };
        budget.charge_items::<NoteNumberFormat>(1)?;
        Ok(Some(NoteNumberFormat {
            kind: budget.copy_str(format.attr("type").unwrap_or("DIGIT"))?,
            user_char: budget.copy_str(format.attr("userChar").unwrap_or_default())?,
            prefix: budget.copy_str(format.attr("prefixChar").unwrap_or_default())?,
            suffix: budget.copy_str(format.attr("suffixChar").unwrap_or_default())?,
        }))
    }

    Ok(NoteNumberFormats {
        footnote: read(root, "footNotePr", budget)?,
        endnote: read(root, "endNotePr", budget)?,
    })
}

fn note_reference_mark(
    node: &XmlNode,
    note_formats: &NoteNumberFormats,
    budget: &mut LoweringBudget,
) -> Result<String, KordocError> {
    let format = if node.name == "endNote" {
        note_formats.endnote.as_ref()
    } else {
        note_formats.footnote.as_ref()
    };
    let kind = format.map_or("DIGIT", |format| format.kind.as_str());
    let number = node
        .attr("number")
        .and_then(|value| value.parse::<u32>().ok())
        .unwrap_or(1);
    let core_decoration = if kind == "USER_CHAR" {
        budget.charge_bytes(8)?;
        note_decoration(node.attr("userChar"))
    } else {
        None
    };
    let numeric_core = if kind == "USER_CHAR" {
        None
    } else {
        budget.charge_bytes(64)?;
        Some(format_note_number(number, kind))
    };
    budget.charge_bytes(8)?;
    let prefix_decoration = note_decoration(node.attr("prefixChar"));
    budget.charge_bytes(8)?;
    let suffix_decoration = note_decoration(node.attr("suffixChar"));
    let prefix = prefix_decoration
        .as_deref()
        .or_else(|| format.map(|format| format.prefix.as_str()))
        .unwrap_or("");
    let core = core_decoration
        .as_deref()
        .or_else(|| {
            format
                .filter(|_| kind == "USER_CHAR")
                .map(|format| format.user_char.as_str())
        })
        .or(numeric_core.as_deref())
        .unwrap_or("");
    let suffix = suffix_decoration
        .as_deref()
        .or_else(|| format.map(|format| format.suffix.as_str()))
        .unwrap_or(if format.is_none() { ")" } else { "" });
    let mut mark = String::new();
    budget.append_str(&mut mark, prefix)?;
    budget.append_str(&mut mark, core)?;
    budget.append_str(&mut mark, suffix)?;
    Ok(mark)
}

fn note_decoration(value: Option<&str>) -> Option<String> {
    let value = value?;
    let codepoint = value.parse::<u32>().ok()?;
    if codepoint == 0 {
        return Some(String::new());
    }
    char::from_u32(codepoint).map(|character| character.to_string())
}

fn format_note_number(number: u32, kind: &str) -> String {
    if number == 0 && kind == "DIGIT" {
        return "0".to_owned();
    }
    let number = number.max(1);
    let index = number - 1;
    match kind {
        "CIRCLED_DIGIT" => circled_number(index),
        "HANGUL_SYLLABLE" => hangul_ordinal(index),
        "CIRCLED_HANGUL_SYLLABLE" => {
            if index < 14 {
                char::from_u32(0x326e + index).unwrap_or(' ').to_string()
            } else {
                hangul_ordinal(index)
            }
        }
        "HANGUL_JAMO" => sequence_character("ㄱㄴㄷㄹㅁㅂㅅㅇㅈㅊㅋㅌㅍㅎ", index),
        "CIRCLED_HANGUL_JAMO" => {
            if index < 14 {
                char::from_u32(0x3260 + index).unwrap_or(' ').to_string()
            } else {
                sequence_character("ㄱㄴㄷㄹㅁㅂㅅㅇㅈㅊㅋㅌㅍㅎ", index)
            }
        }
        "LATIN_CAPITAL" => sequence_character("ABCDEFGHIJKLMNOPQRSTUVWXYZ", index),
        "LATIN_SMALL" => sequence_character("abcdefghijklmnopqrstuvwxyz", index),
        "CIRCLED_LATIN_CAPITAL" => {
            if index < 26 {
                char::from_u32(0x24b6 + index).unwrap_or(' ').to_string()
            } else {
                sequence_character("ABCDEFGHIJKLMNOPQRSTUVWXYZ", index)
            }
        }
        "CIRCLED_LATIN_SMALL" => {
            if index < 26 {
                char::from_u32(0x24d0 + index).unwrap_or(' ').to_string()
            } else {
                sequence_character("abcdefghijklmnopqrstuvwxyz", index)
            }
        }
        "ROMAN_CAPITAL" => roman_numeral(number, true),
        "ROMAN_SMALL" => roman_numeral(number, false),
        _ => number.to_string(),
    }
}

fn circled_number(index: u32) -> String {
    let codepoint = match index {
        0..20 => 0x2460 + index,
        20..35 => 0x3251 + (index - 20),
        35..50 => 0x32b1 + (index - 35),
        _ => return format!("({})", index + 1),
    };
    char::from_u32(codepoint).unwrap_or(' ').to_string()
}

fn sequence_character(sequence: &str, index: u32) -> String {
    let chars: Vec<char> = sequence.chars().collect();
    chars[(index as usize) % chars.len()].to_string()
}

fn hangul_ordinal(index: u32) -> String {
    const INITIALS: [u32; 14] = [0, 2, 3, 5, 6, 7, 9, 11, 12, 14, 15, 16, 17, 18];
    const MEDIALS: [u32; 6] = [0, 4, 8, 13, 18, 20];
    let vowel_index = ((index / INITIALS.len() as u32) as usize).min(MEDIALS.len() - 1);
    let initial = INITIALS[(index % INITIALS.len() as u32) as usize];
    char::from_u32(0xac00 + initial * 588 + MEDIALS[vowel_index] * 28)
        .unwrap_or(' ')
        .to_string()
}

fn roman_numeral(number: u32, uppercase: bool) -> String {
    if number > 3999 {
        return number.to_string();
    }
    let values = [1000, 900, 500, 400, 100, 90, 50, 40, 10, 9, 5, 4, 1];
    let upper = [
        "M", "CM", "D", "CD", "C", "XC", "L", "XL", "X", "IX", "V", "IV", "I",
    ];
    let lower = [
        "m", "cm", "d", "cd", "c", "xc", "l", "xl", "x", "ix", "v", "iv", "i",
    ];
    let symbols = if uppercase { &upper } else { &lower };
    let mut remaining = number;
    let mut output = String::new();
    for (value, symbol) in values.into_iter().zip(symbols) {
        while remaining >= value {
            output.push_str(symbol);
            remaining -= value;
        }
    }
    output
}

fn paragraph_layout_position(
    node: &XmlNode,
    budget: &mut LoweringBudget,
) -> Result<ParagraphLayout, KordocError> {
    let linesegarray = node
        .children
        .iter()
        .find(|child| child.name == "linesegarray");
    let line_count = linesegarray
        .map(|segments| {
            segments
                .children
                .iter()
                .filter(|line| line.name == "lineseg")
                .count()
        })
        .unwrap_or(0);
    budget.charge_items::<LinePosition>(line_count)?;
    budget.charge_items::<ParagraphLayout>(1)?;
    let mut line_positions = Vec::new();
    line_positions
        .try_reserve_exact(line_count)
        .map_err(|_| crate::hwpx::budget::output_limit())?;
    if let Some(lines) = linesegarray {
        for line in lines.children.iter().filter(|line| line.name == "lineseg") {
            line_positions.push(LinePosition {
                vertical: numeric_attribute(line, "vertpos"),
                horizontal: numeric_attribute(line, "horzpos"),
            });
        }
    }
    Ok(ParagraphLayout {
        line_positions,
        explicit_page_break: node.attr("pageBreak") == Some("1"),
        has_lines: linesegarray
            .is_some_and(|array| array.children.iter().any(|child| child.name == "lineseg")),
        is_paragraph: true,
        ..ParagraphLayout::default()
    })
}

fn clone_layout_position(
    layout: &ParagraphLayout,
    budget: &mut LoweringBudget,
) -> Result<ParagraphLayout, KordocError> {
    budget.charge_items::<ParagraphLayout>(1)?;
    budget.charge_items::<LinePosition>(layout.line_positions.len())?;
    Ok(layout.clone())
}

fn table_intra_breaks(table: &XmlNode, budget: &mut LoweringBudget) -> Result<usize, KordocError> {
    let cell_count = table
        .children
        .iter()
        .filter(|node| node.name == "tr")
        .map(|row| row.children.iter().filter(|node| node.name == "tc").count())
        .sum::<usize>();
    let row_storage = std::mem::size_of::<(usize, usize)>() + 3 * std::mem::size_of::<usize>();
    budget.charge_bytes(
        cell_count
            .checked_mul(row_storage)
            .ok_or_else(crate::hwpx::budget::output_limit)?,
    )?;
    let mut by_row = std::collections::BTreeMap::<usize, usize>::new();
    for row in table.children.iter().filter(|node| node.name == "tr") {
        for cell in row.children.iter().filter(|node| node.name == "tc") {
            let row_index = cell
                .children
                .iter()
                .find(|node| node.name == "cellAddr")
                .and_then(|node| node.attr("rowAddr"))
                .and_then(|value| value.parse().ok())
                .unwrap_or(0);
            let Some(sublist) = cell.children.iter().find(|node| node.name == "subList") else {
                continue;
            };
            let mut previous = None;
            let mut resets = 0;
            for paragraph in sublist.children.iter().filter(|node| node.name == "p") {
                for line in paragraph
                    .children
                    .iter()
                    .find(|node| node.name == "linesegarray")
                    .into_iter()
                    .flat_map(|array| array.children.iter().filter(|line| line.name == "lineseg"))
                {
                    let vertical = numeric_attribute(line, "vertpos");
                    if previous.is_some_and(|previous| vertical < previous) {
                        resets += 1;
                    }
                    previous = Some(vertical);
                }
            }
            by_row
                .entry(row_index)
                .and_modify(|max| *max = (*max).max(resets))
                .or_insert(resets);
        }
    }
    Ok(by_row.values().sum())
}

fn numeric_attribute(node: &XmlNode, name: &str) -> f64 {
    node.attr(name)
        .and_then(|value| value.parse::<f64>().ok())
        .filter(|value| value.is_finite())
        .unwrap_or(0.0)
}

fn find_descendant<'a>(node: &'a XmlNode, name: &str, max_depth: usize) -> Option<&'a XmlNode> {
    if max_depth == 0 {
        return None;
    }
    for child in &node.children {
        if child.name == name {
            return Some(child);
        }
        if let Some(found) = find_descendant(child, name, max_depth - 1) {
            return Some(found);
        }
    }
    None
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
