//! Bounded HWPX table lowering.

use std::collections::BTreeSet;

use crate::hwpx::budget::{LoweringBudget, output_limit};
use crate::hwpx::images::{ImageCache, image_placeholder, image_reference, resolve_image};
use crate::hwpx::package::Package;
use crate::hwpx::sections::{append_field_comparison_text, click_here_guide};
use crate::hwpx::xml::{XmlContent, XmlNode};
use kordoc_ir::{
    ErrorCode, IrBlock, IrBlockType, IrCell, IrSpan, IrTable, KordocError, ParseOptions,
};
use kordoc_ir::{ExtractedImage, ParseWarning};

pub(crate) const MAX_COLUMNS: usize = 200;
pub(crate) const MAX_LOGICAL_CELLS: usize = 2_000_000;
pub(crate) const MAX_LOGICAL_DEPTH: usize = 64;

#[derive(Clone, Copy, Default)]
pub(crate) struct CellBudget(usize);

fn limit(message: &'static str) -> KordocError {
    KordocError::new(ErrorCode::DecompressionBomb, message)
}

fn cell_text(
    node: &XmlNode,
    options: &ParseOptions,
    lowering: &mut LoweringBudget,
) -> Result<String, KordocError> {
    let mut paragraphs = Vec::new();
    for part in &node.content {
        if let XmlContent::Child(index) = part {
            let child = &node.children[*index];
            if child.name == "p" {
                let value = paragraph_text(child, options, lowering)?;
                if !value.trim().is_empty() {
                    lowering.push(&mut paragraphs, value)?;
                }
            } else if child.name == "tbl" {
                collect_table_text(child, options, lowering, &mut paragraphs)?;
            } else {
                collect_cell_text(child, options, lowering, &mut paragraphs)?;
            }
        }
    }
    lowering.join_strings(&paragraphs, "\n")
}

fn paragraph_text(
    node: &XmlNode,
    options: &ParseOptions,
    lowering: &mut LoweringBudget,
) -> Result<String, KordocError> {
    fn walk(
        node: &XmlNode,
        options: &ParseOptions,
        lowering: &mut LoweringBudget,
        text: &mut String,
        parts: &mut Vec<String>,
    ) -> Result<(), KordocError> {
        for part in &node.content {
            match part {
                XmlContent::Text { start, end } => {
                    lowering.append_str(text, &node.text[*start..*end])?;
                }
                XmlContent::Child(index) => {
                    let child = &node.children[*index];
                    match child.name.as_str() {
                        "fieldBegin" | "fieldEnd" => {}
                        "tbl" => {
                            if !text.is_empty() {
                                lowering.push(parts, std::mem::take(text))?;
                            }
                            collect_table_text(child, options, lowering, parts)?;
                        }
                        _ => walk(child, options, lowering, text, parts)?,
                    }
                }
            }
        }
        Ok(())
    }
    let mut text = String::new();
    let mut parts = Vec::new();
    walk(node, options, lowering, &mut text, &mut parts)?;
    if !text.is_empty() {
        lowering.push(&mut parts, text)?;
    }
    lowering.join_strings(&parts, "\n")
}

fn cell_placeholder_spans(
    node: &XmlNode,
    options: &ParseOptions,
    lowering: &mut LoweringBudget,
) -> Result<Option<Vec<IrSpan>>, KordocError> {
    fn has_table(node: &XmlNode) -> bool {
        node.children
            .iter()
            .any(|child| child.name == "tbl" || has_table(child))
    }
    fn has_click_here_field(node: &XmlNode) -> bool {
        node.children.iter().any(|child| {
            (child.name == "fieldBegin"
                && child
                    .attr("type")
                    .is_some_and(|kind| kind.eq_ignore_ascii_case("CLICK_HERE"))
                && child.attr("dirty") != Some("1"))
                || has_click_here_field(child)
        })
    }
    if options.include_field_placeholders == Some(true)
        || has_table(node)
        || !has_click_here_field(node)
    {
        return Ok(None);
    }

    fn walk(
        node: &XmlNode,
        lowering: &mut LoweringBudget,
        spans: &mut Vec<IrSpan>,
        fields: &mut Vec<(Option<String>, usize)>,
    ) -> Result<(), KordocError> {
        for part in &node.content {
            match part {
                XmlContent::Text { start, end } => {
                    let text = &node.text[*start..*end];
                    if !text.is_empty() {
                        let text = lowering.copy_str(text)?;
                        lowering.push(
                            spans,
                            IrSpan {
                                text,
                                ..IrSpan::default()
                            },
                        )?;
                    }
                }
                XmlContent::Child(index) => {
                    let child = &node.children[*index];
                    match child.name.as_str() {
                        "fieldBegin" => {
                            lowering.charge_items::<(Option<String>, usize)>(1)?;
                            let guide = click_here_guide(child, lowering)?;
                            lowering.push(fields, (guide, spans.len()))?;
                        }
                        "fieldEnd" => {
                            if let Some((Some(guide), start)) = fields.pop() {
                                let mut value = String::new();
                                let mut contains_nested_placeholder = false;
                                for span in spans.iter().skip(start) {
                                    contains_nested_placeholder |= span.placeholder == Some(true);
                                    append_field_comparison_text(&mut value, &span.text, lowering)?;
                                }
                                if !contains_nested_placeholder
                                    && !value.is_empty()
                                    && (value == guide || value.trim_end() == guide)
                                {
                                    for span in &mut spans[start..] {
                                        span.placeholder = Some(true);
                                    }
                                }
                            }
                        }
                        _ => walk(child, lowering, spans, fields)?,
                    }
                }
            }
        }
        Ok(())
    }

    let mut spans = Vec::new();
    walk(node, lowering, &mut spans, &mut Vec::new())?;
    if spans.iter().any(|span| span.placeholder == Some(true)) {
        Ok(Some(spans))
    } else {
        Ok(None)
    }
}

fn collect_cell_text(
    node: &XmlNode,
    options: &ParseOptions,
    lowering: &mut LoweringBudget,
    out: &mut Vec<String>,
) -> Result<(), KordocError> {
    for part in &node.content {
        if let XmlContent::Child(index) = part {
            let child = &node.children[*index];
            match child.name.as_str() {
                "p" => {
                    let value = paragraph_text(child, options, lowering)?;
                    if !value.trim().is_empty() {
                        lowering.push(out, value)?;
                    }
                }
                "tbl" => collect_table_text(child, options, lowering, out)?,
                _ => collect_cell_text(child, options, lowering, out)?,
            }
        }
    }
    Ok(())
}

fn collect_table_text(
    node: &XmlNode,
    options: &ParseOptions,
    lowering: &mut LoweringBudget,
    out: &mut Vec<String>,
) -> Result<(), KordocError> {
    for child in &node.children {
        if child.name == "caption" {
            collect_cell_text(child, options, lowering, out)?;
        } else if child.name == "tr" {
            for cell in child.children.iter().filter(|node| node.name == "tc") {
                if let Some(sub) = cell.children.iter().find(|node| node.name == "subList") {
                    collect_cell_text(sub, options, lowering, out)?;
                }
            }
        }
    }
    Ok(())
}

#[cfg(test)]
pub(crate) fn lower_table(
    node: &XmlNode,
    depth: usize,
    budget: &mut CellBudget,
) -> Result<IrBlock, KordocError> {
    lower_table_with_assets(
        node,
        depth,
        budget,
        &mut LoweringBudget::default(),
        &ParseOptions::default(),
        None,
        &mut ImageCache::default(),
        &mut Vec::new(),
        &mut Vec::new(),
    )
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn lower_table_with_assets(
    node: &XmlNode,
    depth: usize,
    budget: &mut CellBudget,
    lowering: &mut LoweringBudget,
    options: &ParseOptions,
    mut package: Option<&mut Package<'_>>,
    image_cache: &mut ImageCache,
    images: &mut Vec<ExtractedImage>,
    warnings: &mut Vec<ParseWarning>,
) -> Result<IrBlock, KordocError> {
    if depth >= MAX_LOGICAL_DEPTH {
        return Err(limit("HWPX logical table nesting exceeds its bound"));
    }
    let row_count = node
        .children
        .iter()
        .filter(|child| child.name == "tr")
        .count();
    lowering.charge_items::<Vec<(usize, usize, IrCell)>>(row_count)?;
    let mut anchors = Vec::new();
    anchors
        .try_reserve_exact(row_count)
        .map_err(|_| output_limit())?;
    let mut max_rows = row_count;
    let mut max_cols = 0usize;
    let mut caption = Vec::new();
    let mut caption_blocks = Vec::new();
    for child in &node.children {
        if child.name == "caption" {
            let value = lowering.raw_xml_text(child)?;
            let value = lowering.copy_str(value.trim())?;
            if !value.is_empty() {
                lowering.push(&mut caption, value)?;
            }
            for block in lower_nested(
                child,
                depth + 1,
                budget,
                lowering,
                options,
                package.as_deref_mut(),
                image_cache,
                images,
                warnings,
            )? {
                lowering.push(&mut caption_blocks, block)?;
            }
        }
    }
    for (row_index, row) in node
        .children
        .iter()
        .filter(|child| child.name == "tr")
        .enumerate()
    {
        let cell_count = row
            .children
            .iter()
            .filter(|child| child.name == "tc")
            .count();
        lowering.charge_items::<(usize, usize, IrCell)>(cell_count)?;
        let mut out_row = Vec::new();
        out_row
            .try_reserve_exact(cell_count)
            .map_err(|_| output_limit())?;
        let mut logical_cols = 0usize;
        for tc in row.children.iter().filter(|child| child.name == "tc") {
            let (row_span, col_span) =
                if let Some(span) = tc.children.iter().find(|n| n.name == "cellSpan") {
                    (attr_span(span, "rowSpan")?, attr_span(span, "colSpan")?)
                } else {
                    (1, 1)
                };
            if col_span > MAX_COLUMNS {
                return Err(limit("HWPX table cell exceeds 200 columns"));
            }
            let addr = tc.children.iter().find(|n| n.name == "cellAddr");
            let col = addr
                .and_then(|n| n.attr("colAddr"))
                .and_then(|v| v.parse::<usize>().ok())
                .unwrap_or(logical_cols);
            let row_addr = addr
                .and_then(|n| n.attr("rowAddr"))
                .and_then(|v| v.parse::<usize>().ok())
                .unwrap_or(row_index);
            let end = col
                .checked_add(col_span)
                .ok_or_else(|| limit("HWPX table column count overflows"))?;
            if end > MAX_COLUMNS {
                return Err(limit("HWPX table exceeds 200 columns"));
            }
            let rows_required = row_addr
                .checked_add(row_span)
                .ok_or_else(|| limit("HWPX table row count overflows"))?;
            max_rows = max_rows.max(rows_required);
            logical_cols = logical_cols.max(end);
            let sub = tc.children.iter().find(|n| n.name == "subList");
            let text = match sub {
                Some(sub) => cell_text(sub, options, lowering)?,
                None => String::new(),
            };
            let nested = if let Some(sub) = sub {
                lower_nested(
                    sub,
                    depth + 1,
                    budget,
                    lowering,
                    options,
                    package.as_deref_mut(),
                    image_cache,
                    images,
                    warnings,
                )?
            } else {
                Vec::new()
            };
            out_row.push((
                row_addr,
                col,
                IrCell {
                    text,
                    col_span: col_span as u32,
                    row_span: row_span as u32,
                    blocks: nested
                        .iter()
                        .any(|block| block.kind != IrBlockType::Paragraph || block.spans.is_some())
                        .then_some(nested),
                    is_header: (tc.attr("header").is_some_and(|v| v == "1" || v == "true"))
                        .then_some(true),
                },
            ));
        }
        max_cols = max_cols.max(logical_cols);
        anchors.push(out_row);
    }
    if max_rows > MAX_LOGICAL_CELLS {
        return Err(limit("HWPX table row count exceeds the logical cell bound"));
    }
    let logical = max_rows
        .checked_mul(max_cols)
        .ok_or_else(|| limit("HWPX logical cell count overflows"))?;
    budget.0 = budget
        .0
        .checked_add(logical)
        .ok_or_else(|| limit("HWPX logical cell count overflows"))?;
    if budget.0 > MAX_LOGICAL_CELLS {
        return Err(limit("HWPX tables exceed 2,000,000 logical cells"));
    }
    let empty_cell = IrCell {
        col_span: 1,
        row_span: 1,
        ..IrCell::default()
    };
    lowering.charge_items::<IrCell>(logical)?;
    lowering.charge_items::<Vec<IrCell>>(max_rows)?;
    let mut cells = Vec::new();
    cells
        .try_reserve_exact(max_rows)
        .map_err(|_| output_limit())?;
    for _ in 0..max_rows {
        let mut row = Vec::new();
        row.try_reserve_exact(max_cols)
            .map_err(|_| output_limit())?;
        for _ in 0..max_cols {
            row.push(empty_cell.clone());
        }
        cells.push(row);
    }
    let has_header = anchors
        .iter()
        .flatten()
        .any(|(_, _, cell)| cell.is_header == Some(true));
    let mut anchor_cols = BTreeSet::new();
    for row in anchors {
        for (r, c, cell) in row {
            anchor_cols.insert(c);
            if r < max_rows && c < max_cols {
                cells[r][c] = cell;
            }
        }
    }
    let mut effective_cols = max_cols;
    while effective_cols > 0
        && cells
            .iter()
            .all(|row| row[effective_cols - 1].text.trim().is_empty())
        && !(options.keep_trailing_empty_cols == Some(true)
            && anchor_cols.contains(&(effective_cols - 1)))
    {
        effective_cols -= 1;
    }
    if effective_cols > 0 && effective_cols < max_cols {
        for row in &mut cells {
            row.truncate(effective_cols);
            for (column, cell) in row.iter_mut().enumerate() {
                cell.col_span = cell.col_span.min((effective_cols - column) as u32);
            }
        }
        max_cols = effective_cols;
    }
    let table = IrTable {
        rows: max_rows as u32,
        cols: max_cols as u32,
        cells,
        has_header,
        source_id: node
            .attr("id")
            .map(|id| lowering.copy_str(id))
            .transpose()?,
        caption: if caption.is_empty() {
            None
        } else {
            Some(lowering.join_strings(&caption, "\n")?)
        },
        caption_blocks: caption_blocks
            .iter()
            .any(|block| block.kind != IrBlockType::Paragraph)
            .then_some(caption_blocks),
        ..IrTable::default()
    };
    Ok(IrBlock {
        kind: IrBlockType::Table,
        table: Some(table),
        ..IrBlock::default()
    })
}

fn attr_span(node: &XmlNode, key: &str) -> Result<usize, KordocError> {
    match node.attr(key).and_then(|value| value.parse::<usize>().ok()) {
        Some(value) if value > 0 => Ok(value),
        Some(_) => Ok(1),
        None => Err(KordocError::new(
            ErrorCode::Corrupted,
            "HWPX table span is malformed",
        )),
    }
}

#[allow(clippy::too_many_arguments)]
fn lower_nested(
    node: &XmlNode,
    depth: usize,
    budget: &mut CellBudget,
    lowering: &mut LoweringBudget,
    options: &ParseOptions,
    mut package: Option<&mut Package<'_>>,
    image_cache: &mut ImageCache,
    images: &mut Vec<ExtractedImage>,
    warnings: &mut Vec<ParseWarning>,
) -> Result<Vec<IrBlock>, KordocError> {
    let mut out = Vec::new();
    for part in &node.content {
        if let XmlContent::Child(i) = part {
            let child = &node.children[*i];
            if child.name == "tbl" {
                let block = lower_table_with_assets(
                    child,
                    depth,
                    budget,
                    lowering,
                    options,
                    package.as_deref_mut(),
                    image_cache,
                    images,
                    warnings,
                )?;
                lowering.push(&mut out, block)?;
            } else if child.name == "p" {
                for block in lower_nested_paragraph(
                    child,
                    depth,
                    budget,
                    lowering,
                    options,
                    package.as_deref_mut(),
                    image_cache,
                    images,
                    warnings,
                )? {
                    lowering.push(&mut out, block)?;
                }
            } else {
                for block in lower_nested(
                    child,
                    depth,
                    budget,
                    lowering,
                    options,
                    package.as_deref_mut(),
                    image_cache,
                    images,
                    warnings,
                )? {
                    lowering.push(&mut out, block)?;
                }
            }
        }
    }
    Ok(out)
}

#[allow(clippy::too_many_arguments)]
fn lower_nested_paragraph(
    node: &XmlNode,
    depth: usize,
    budget: &mut CellBudget,
    lowering: &mut LoweringBudget,
    options: &ParseOptions,
    mut package: Option<&mut Package<'_>>,
    image_cache: &mut ImageCache,
    images: &mut Vec<ExtractedImage>,
    warnings: &mut Vec<ParseWarning>,
) -> Result<Vec<IrBlock>, KordocError> {
    #[allow(clippy::too_many_arguments)]
    fn walk(
        node: &XmlNode,
        depth: usize,
        budget: &mut CellBudget,
        lowering: &mut LoweringBudget,
        options: &ParseOptions,
        package: &mut Option<&mut Package<'_>>,
        image_cache: &mut ImageCache,
        images: &mut Vec<ExtractedImage>,
        warnings: &mut Vec<ParseWarning>,
        text: &mut String,
        out: &mut Vec<IrBlock>,
    ) -> Result<(), KordocError> {
        for part in &node.content {
            match part {
                XmlContent::Text { start, end } => {
                    lowering.append_str(text, &node.text[*start..*end])?
                }
                XmlContent::Child(index) => {
                    let child = &node.children[*index];
                    if child.name == "tbl" {
                        if !text.trim().is_empty() {
                            lowering.push(out, IrBlock::paragraph(std::mem::take(text)))?;
                        } else {
                            text.clear();
                        }
                        let block = lower_table_with_assets(
                            child,
                            depth,
                            budget,
                            lowering,
                            options,
                            package.as_deref_mut(),
                            image_cache,
                            images,
                            warnings,
                        )?;
                        lowering.push(out, block)?;
                    } else if matches!(child.name.as_str(), "pic" | "img" | "imgRect" | "imgClip") {
                        if !text.trim().is_empty() {
                            lowering.push(out, IrBlock::paragraph(std::mem::take(text)))?;
                        } else {
                            text.clear();
                        }
                        if let Some(reference) = image_reference(child) {
                            let block = if let Some(package) = package.as_deref_mut() {
                                resolve_image(
                                    reference,
                                    None,
                                    package,
                                    image_cache,
                                    images,
                                    warnings,
                                )?
                            } else {
                                image_placeholder(lowering.copy_str(reference)?)
                            };
                            lowering.push(out, block)?;
                        }
                    } else {
                        walk(
                            child,
                            depth,
                            budget,
                            lowering,
                            options,
                            package,
                            image_cache,
                            images,
                            warnings,
                            text,
                            out,
                        )?;
                    }
                }
            }
        }
        Ok(())
    }
    let mut out = Vec::new();
    let mut text = String::new();
    walk(
        node,
        depth,
        budget,
        lowering,
        options,
        &mut package,
        image_cache,
        images,
        warnings,
        &mut text,
        &mut out,
    )?;
    if !text.trim().is_empty() {
        lowering.push(&mut out, IrBlock::paragraph(text))?;
    }
    if let Some(spans) = cell_placeholder_spans(node, options, lowering)?
        && let Some(block) = out
            .iter_mut()
            .find(|block| block.kind == IrBlockType::Paragraph)
    {
        block.spans = Some(spans);
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::hwpx::budget::LoweringBudget;
    use crate::hwpx::xml::parse;

    #[test]
    fn plain_cell_paragraph_placeholder_preflight_uses_no_lowering_budget() {
        let root = parse(b"<p><run><t>Plain cell text</t></run></p>").unwrap();
        let paragraph = root.children.first().unwrap();
        let spans = cell_placeholder_spans(
            paragraph,
            &ParseOptions::default(),
            &mut LoweringBudget::with_limit(0),
        )
        .unwrap();

        assert!(spans.is_none());
    }

    fn lower_with_budget(
        node: &XmlNode,
        lowering: &mut LoweringBudget,
    ) -> Result<IrBlock, KordocError> {
        lower_table_with_assets(
            node,
            0,
            &mut CellBudget::default(),
            lowering,
            &ParseOptions::default(),
            None,
            &mut ImageCache::default(),
            &mut Vec::new(),
            &mut Vec::new(),
        )
    }

    fn lower_table_with_limit(node: &XmlNode, limit: usize) -> Result<IrBlock, KordocError> {
        lower_with_budget(node, &mut LoweringBudget::with_limit(limit))
    }

    #[test]
    fn keeps_merged_cell_topology() {
        let root = parse(br#"<tbl><tr><tc><cellAddr rowAddr="0" colAddr="0"/><cellSpan rowSpan="2" colSpan="2"/><subList><p><run><t>A</t></run></p></subList></tc><tc><cellAddr rowAddr="0" colAddr="2"/><subList><p><run><t>B</t></run></p></subList></tc></tr><tr><tc><cellAddr rowAddr="1" colAddr="2"/><subList/></tc></tr></tbl>"#).unwrap();
        let block = lower_table(&root, 0, &mut CellBudget::default()).unwrap();
        let table = block.table.unwrap();
        assert_eq!((table.rows, table.cols), (2, 3));
        assert_eq!(
            (table.cells[0][0].row_span, table.cells[0][0].col_span),
            (2, 2)
        );
        assert_eq!(
            (table.cells[1][0].col_span, table.cells[1][0].row_span),
            (1, 1)
        );
    }

    #[test]
    fn flattens_nested_cell_text_and_retains_ordered_blocks() {
        let root = parse(br#"<tbl><tr><tc><cellAddr rowAddr="0" colAddr="0"/><subList><p><run><t>Outer cell</t></run><tbl><caption><subList><p><run><t>Synthetic caption</t></run></p></subList></caption><tr><tc><cellAddr rowAddr="0" colAddr="0"/><subList><p><run><t>Inner cell</t></run><tbl><tr><tc><cellAddr rowAddr="0" colAddr="0"/><subList><p><run><t>Deep cell</t></run></p></subList></tc></tr></tbl></p></subList></tc></tr></tbl></p></subList></tc></tr></tbl>"#).unwrap();
        let block = lower_table(&root, 0, &mut CellBudget::default()).unwrap();
        let table = block.table.unwrap();
        let cell = &table.cells[0][0];

        assert_eq!(
            cell.text,
            "Outer cell\nSynthetic caption\nInner cell\nDeep cell"
        );
        let blocks = cell.blocks.as_ref().unwrap();
        assert_eq!(blocks.len(), 2);
        assert_eq!(blocks[0].kind, IrBlockType::Paragraph);
        assert_eq!(blocks[0].text.as_deref(), Some("Outer cell"));
        assert_eq!(blocks[1].kind, IrBlockType::Table);
    }

    #[test]
    fn table_lowering_budget_accepts_exact_minimum_and_rejects_one_byte_less() {
        let root = parse(br#"<tbl id="id"><tr><tc><cellAddr rowAddr="0" colAddr="0"/><subList><p><run><t>cell text</t></run></p></subList></tc></tr></tbl>"#).unwrap();
        let minimum = (0..=16_384)
            .find(|limit| lower_table_with_limit(&root, *limit).is_ok())
            .expect("small one-cell table should fit within 16 KiB");

        assert!(lower_table_with_limit(&root, minimum).is_ok());
        assert_eq!(
            lower_table_with_limit(&root, minimum - 1).unwrap_err().code,
            ErrorCode::OutputTooLarge
        );
    }

    #[test]
    fn repeated_nested_table_text_fails_the_lowering_allocation_budget() {
        let repeated = (0..32)
            .map(|_| {
                "<tbl><tr><tc><cellAddr rowAddr=\"0\" colAddr=\"0\"/><subList><p><run><t>"
                    .to_owned()
                    + &"x".repeat(128)
                    + "</t></run></p></subList></tc></tr></tbl>"
            })
            .collect::<String>();
        let source = format!(
            "<tbl><tr><tc><cellAddr rowAddr=\"0\" colAddr=\"0\"/><subList>{repeated}</subList></tc></tr></tbl>"
        );
        let root = parse(source.as_bytes()).unwrap();

        assert_eq!(
            lower_table_with_limit(&root, 512).unwrap_err().code,
            ErrorCode::OutputTooLarge
        );
    }

    #[test]
    fn enforces_column_limit_and_does_not_truncate_row_span() {
        let wide = parse(
            br#"<tbl><tr><tc><cellAddr rowAddr="0" colAddr="200"/><subList/></tc></tr></tbl>"#,
        )
        .unwrap();
        assert_eq!(
            lower_table(&wide, 0, &mut CellBudget::default())
                .unwrap_err()
                .code,
            ErrorCode::DecompressionBomb
        );
        let tall = parse(br#"<tbl><tr><tc><cellAddr rowAddr="0" colAddr="0"/><cellSpan rowSpan="201" colSpan="1"/><subList/></tc></tr></tbl>"#).unwrap();
        let table = lower_table(&tall, 0, &mut CellBudget::default())
            .unwrap()
            .table
            .unwrap();
        assert_eq!(table.cells[0][0].row_span, 201);
        assert_eq!(table.rows, 201);
    }

    #[test]
    fn enforces_aggregate_cell_and_nesting_limits_at_boundaries() {
        let one_cell =
            parse(br#"<tbl><tr><tc><cellAddr rowAddr="0" colAddr="0"/><subList/></tc></tr></tbl>"#)
                .unwrap();
        let mut budget = CellBudget(1_999_999);
        assert!(lower_table(&one_cell, 0, &mut budget).is_ok());
        assert_eq!(budget.0, MAX_LOGICAL_CELLS);
        assert_eq!(
            lower_table(&one_cell, 0, &mut budget).unwrap_err().code,
            ErrorCode::DecompressionBomb
        );

        assert!(lower_table(&one_cell, MAX_LOGICAL_DEPTH - 1, &mut CellBudget::default()).is_ok());
        assert_eq!(
            lower_table(&one_cell, MAX_LOGICAL_DEPTH, &mut CellBudget::default())
                .unwrap_err()
                .code,
            ErrorCode::DecompressionBomb
        );

        let far_row = parse(
            br#"<tbl><tr><tc><cellAddr rowAddr="2000000" colAddr="0"/><subList/></tc></tr></tbl>"#,
        )
        .unwrap();
        assert_eq!(
            lower_table(&far_row, 0, &mut CellBudget::default())
                .unwrap_err()
                .code,
            ErrorCode::DecompressionBomb
        );
    }
}
