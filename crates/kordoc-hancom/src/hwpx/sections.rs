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

const MAX_PAGE_EVIDENCE_ENTRIES: u32 = 100_000;

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
    layout_positions: Vec<ParagraphLayout>,
    layout_usable: bool,
    multi_column: bool,
}

#[derive(Debug, Clone, Copy)]
struct LinePosition {
    vertical: f64,
    horizontal: f64,
}

#[derive(Debug, Default)]
struct ParagraphLayout {
    line_positions: Vec<LinePosition>,
    explicit_page_break: bool,
    has_lines: bool,
    block_index: Option<usize>,
    is_paragraph: bool,
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
    let mut output = SectionOutput::default();
    let mut deltas: Vec<Option<SectionDelta>> = Vec::with_capacity(inputs.len());
    for (index, input) in inputs.iter().enumerate() {
        match parse(&input.bytes) {
            Ok(root) => {
                let multi_column = find_descendant(&root, "colPr", 8)
                    .and_then(|columns| columns.attr("colCount"))
                    .and_then(|value| value.parse::<f64>().ok())
                    .is_some_and(|count| count > 1.0);
                let top_level_paragraphs = root.children.iter().filter(|child| child.name == "p");
                let paragraph_layouts: Vec<_> = top_level_paragraphs
                    .map(paragraph_layout_position)
                    .collect();
                let mut delta = SectionDelta {
                    layout_usable: !paragraph_layouts.is_empty()
                        && paragraph_layouts.iter().all(|layout| layout.has_lines),
                    multi_column,
                    ..SectionDelta::default()
                };
                let note_formats = parse_note_number_formats(&root);
                lower_content(
                    &root,
                    styles,
                    &note_formats,
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
        .as_ref()
        .map(|cache| cache.section_pages.as_slice())
        .or_else(|| supplied_layout_usable.then_some(layout_cache).flatten());
    let layout_usable = selected_layout_cache.is_some();

    let mut pages_seen = BTreeSet::new();
    if let Some(xml_layout_cache) = &xml_layout_cache {
        pages_seen.extend(xml_layout_cache.evidence_pages.iter().copied());
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
        pages_seen.extend(1..=max_page);
    }
    for (index, delta) in deltas.into_iter().enumerate() {
        let fallback_page = u32::try_from(index + 1).map_err(|_| {
            KordocError::new(
                ErrorCode::DecompressionBomb,
                "section count exceeds page limit",
            )
        })?;
        if !layout_usable {
            pages_seen.insert(fallback_page);
        }
        let Some(mut delta) = delta else {
            continue;
        };
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

fn derive_xml_layout_cache(deltas: &[Option<SectionDelta>]) -> Option<XmlLayoutCache> {
    if deltas.is_empty() {
        return None;
    }
    let mut cache = Vec::with_capacity(deltas.len());
    let mut evidence_pages = BTreeSet::new();
    let mut last_page = 0u32;
    let mut saw_layout_hint = false;
    for delta in deltas {
        let delta = delta.as_ref()?;
        if !delta.layout_usable
            || delta
                .layout_positions
                .iter()
                .filter(|layout| layout.block_index.is_some())
                .count()
                != delta.blocks.len()
        {
            return None;
        }
        let mut page = last_page.checked_add(1)?;
        evidence_pages.insert(page);
        let mut previous_position = None;
        let mut previous_horizontal = None;
        let mut section_pages = vec![page; delta.blocks.len()];
        let mut saw_paragraph = false;
        for layout in &delta.layout_positions {
            let explicit_break = layout.explicit_page_break && saw_paragraph;
            if explicit_break {
                page = page.checked_add(1)?;
                evidence_pages.insert(page);
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
                if line_reset && !(first_line && broke_by_explicit) {
                    page = page.checked_add(1)?;
                    evidence_pages.insert(page);
                }
                if first_line {
                    if let Some(block_index) = layout.block_index {
                        section_pages[block_index] = page;
                        evidence_pages.insert(page);
                    }
                    broke_by_explicit = false;
                }
                previous_position = Some(position.vertical);
                previous_horizontal = Some(position.horizontal);
                first_line = false;
            }
            if first_line && let Some(block_index) = layout.block_index {
                section_pages[block_index] = page;
                evidence_pages.insert(page);
            }
            saw_layout_hint |= explicit_break || !layout.line_positions.is_empty();
            if let Some(block_index) = layout.block_index {
                evidence_pages.insert(section_pages[block_index]);
            }
            saw_paragraph |= layout.is_paragraph;
        }
        if !delta.blocks.is_empty() {
            evidence_pages.insert(page);
        }
        last_page = page;
        cache.push(section_pages);
    }
    saw_layout_hint.then_some(XmlLayoutCache {
        section_pages: cache,
        evidence_pages,
    })
}

fn lower_content(
    node: &XmlNode,
    styles: &StyleCatalog,
    note_formats: &NoteNumberFormats,
    keep_empty_paragraphs: bool,
    delta: &mut SectionDelta,
) {
    match node.name.as_str() {
        "p" => lower_paragraph(node, styles, note_formats, keep_empty_paragraphs, delta),
        "footNote" | "endNote" => {
            let text = node.text_content();
            if !text.is_empty() {
                let block_index = delta.blocks.len();
                delta.blocks.push(IrBlock {
                    text: Some(text.clone()),
                    footnote_text: Some(text),
                    ..IrBlock::default()
                });
                delta.layout_positions.push(ParagraphLayout {
                    block_index: Some(block_index),
                    ..ParagraphLayout::default()
                });
            }
        }
        _ => {
            for part in &node.content {
                if let XmlContent::Child(index) = part {
                    lower_content(
                        &node.children[*index],
                        styles,
                        note_formats,
                        keep_empty_paragraphs,
                        delta,
                    );
                }
            }
        }
    }
}

fn lower_paragraph(
    node: &XmlNode,
    styles: &StyleCatalog,
    note_formats: &NoteNumberFormats,
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
                    "run" => append_inline_content(
                        child,
                        None,
                        styles,
                        note_formats,
                        &mut spans,
                        &mut notes,
                    ),
                    "footNote" | "endNote" => {
                        append_note(child, None, note_formats, &mut spans, &mut notes)
                    }
                    "ctrl" => append_inline_content(
                        child,
                        None,
                        styles,
                        note_formats,
                        &mut spans,
                        &mut notes,
                    ),
                    _ => {}
                }
            }
        }
    }
    if spans.is_empty() && !keep_empty_paragraphs && notes.is_empty() {
        delta.layout_positions.push(paragraph_layout_position(node));
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
    delta.layout_positions.push(ParagraphLayout {
        block_index: Some(block_index),
        ..layout_position
    });
}

fn append_inline_content(
    node: &XmlNode,
    style: Option<&kordoc_ir::InlineStyle>,
    styles: &StyleCatalog,
    note_formats: &NoteNumberFormats,
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
                    "footNote" | "endNote" => {
                        append_note(child, active_style, note_formats, spans, notes)
                    }
                    _ => append_inline_content(
                        child,
                        active_style,
                        styles,
                        note_formats,
                        spans,
                        notes,
                    ),
                }
            }
        }
    }
}

fn append_note(
    node: &XmlNode,
    style: Option<&kordoc_ir::InlineStyle>,
    note_formats: &NoteNumberFormats,
    spans: &mut Vec<IrSpan>,
    notes: &mut Vec<String>,
) {
    spans.push(IrSpan {
        text: note_reference_mark(node, note_formats),
        bold: style.and_then(|style| style.bold),
        italic: style.and_then(|style| style.italic),
        strike: style.and_then(|style| style.strike),
        underline: style.and_then(|style| style.underline),
        ..IrSpan::default()
    });
    let text = node.text_content();
    let text = text.trim();
    if !text.is_empty() {
        notes.push(text.to_owned());
    }
}

fn parse_note_number_formats(root: &XmlNode) -> NoteNumberFormats {
    fn read(root: &XmlNode, property: &str) -> Option<NoteNumberFormat> {
        let properties = root.descendants(property).into_iter().next()?;
        let format = properties
            .children
            .iter()
            .find(|child| child.name == "autoNumFormat")?;
        Some(NoteNumberFormat {
            kind: format.attr("type").unwrap_or("DIGIT").to_owned(),
            user_char: format.attr("userChar").unwrap_or_default().to_owned(),
            prefix: format.attr("prefixChar").unwrap_or_default().to_owned(),
            suffix: format.attr("suffixChar").unwrap_or_default().to_owned(),
        })
    }

    NoteNumberFormats {
        footnote: read(root, "footNotePr"),
        endnote: read(root, "endNotePr"),
    }
}

fn note_reference_mark(node: &XmlNode, note_formats: &NoteNumberFormats) -> String {
    let default = NoteNumberFormat {
        kind: "DIGIT".to_owned(),
        user_char: String::new(),
        prefix: String::new(),
        suffix: ")".to_owned(),
    };
    let format = if node.name == "endNote" {
        note_formats.endnote.as_ref()
    } else {
        note_formats.footnote.as_ref()
    }
    .unwrap_or(&default);
    let number = node
        .attr("number")
        .and_then(|value| value.parse::<u32>().ok())
        .unwrap_or(1);
    let core = if format.kind == "USER_CHAR" {
        note_decoration(node.attr("userChar")).unwrap_or_else(|| format.user_char.clone())
    } else {
        format_note_number(number, &format.kind)
    };
    let prefix = note_decoration(node.attr("prefixChar")).unwrap_or_else(|| format.prefix.clone());
    let suffix = note_decoration(node.attr("suffixChar")).unwrap_or_else(|| format.suffix.clone());
    format!("{prefix}{core}{suffix}")
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

fn paragraph_layout_position(node: &XmlNode) -> ParagraphLayout {
    let linesegarray = node
        .children
        .iter()
        .find(|child| child.name == "linesegarray");
    let line_positions = linesegarray
        .map(|line_segments| {
            line_segments
                .children
                .iter()
                .filter(|line| line.name == "lineseg")
                .map(|line| LinePosition {
                    vertical: numeric_attribute(line, "vertpos"),
                    horizontal: numeric_attribute(line, "horzpos"),
                })
                .collect()
        })
        .unwrap_or_default();
    ParagraphLayout {
        line_positions,
        explicit_page_break: node.attr("pageBreak") == Some("1"),
        has_lines: linesegarray
            .is_some_and(|array| array.children.iter().any(|child| child.name == "lineseg")),
        is_paragraph: true,
        ..ParagraphLayout::default()
    }
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
