//! Bounded HWPX table lowering.

use crate::hwpx::images::{ImageCache, image_placeholder, image_reference, resolve_image};
use crate::hwpx::package::Package;
use crate::hwpx::xml::{XmlContent, XmlNode};
use kordoc_ir::{ErrorCode, IrBlock, IrBlockType, IrCell, IrTable, KordocError};
use kordoc_ir::{ExtractedImage, ParseWarning};

pub(crate) const MAX_COLUMNS: usize = 200;
pub(crate) const MAX_LOGICAL_CELLS: usize = 2_000_000;
pub(crate) const MAX_LOGICAL_DEPTH: usize = 64;

#[derive(Clone, Copy, Default)]
pub(crate) struct CellBudget(usize);

fn limit(message: &'static str) -> KordocError {
    KordocError::new(ErrorCode::DecompressionBomb, message)
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
        None,
        &mut ImageCache::default(),
        &mut Vec::new(),
        &mut Vec::new(),
    )
}

pub(crate) fn lower_table_with_assets(
    node: &XmlNode,
    depth: usize,
    budget: &mut CellBudget,
    mut package: Option<&mut Package<'_>>,
    image_cache: &mut ImageCache,
    images: &mut Vec<ExtractedImage>,
    warnings: &mut Vec<ParseWarning>,
) -> Result<IrBlock, KordocError> {
    if depth >= MAX_LOGICAL_DEPTH {
        return Err(limit("HWPX logical table nesting exceeds its bound"));
    }
    let rows: Vec<_> = node
        .children
        .iter()
        .filter(|child| child.name == "tr")
        .collect();
    let mut anchors = Vec::with_capacity(rows.len());
    let mut max_rows = rows.len();
    let mut max_cols = 0usize;
    let mut caption = Vec::new();
    let mut caption_blocks = Vec::new();
    for child in &node.children {
        if child.name == "caption" {
            let value = child.text_content().trim().to_owned();
            if !value.is_empty() {
                caption.push(value);
            }
            caption_blocks.extend(lower_nested(
                child,
                depth + 1,
                budget,
                package.as_deref_mut(),
                image_cache,
                images,
                warnings,
            )?);
        }
    }
    for (row_index, row) in rows.into_iter().enumerate() {
        let mut out_row = Vec::new();
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
            let text = sub.map_or_else(String::new, XmlNode::text_content);
            let nested = if let Some(sub) = sub {
                lower_nested(
                    sub,
                    depth + 1,
                    budget,
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
                        .any(|block| block.kind != IrBlockType::Paragraph)
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
    let mut cells = vec![vec![empty_cell; max_cols]; max_rows];
    let has_header = anchors
        .iter()
        .flatten()
        .any(|(_, _, cell)| cell.is_header == Some(true));
    for row in anchors {
        for (r, c, cell) in row {
            if r < max_rows && c < max_cols {
                cells[r][c] = cell;
            }
        }
    }
    let table = IrTable {
        rows: max_rows as u32,
        cols: max_cols as u32,
        cells,
        has_header,
        source_id: node.attr("id").map(str::to_owned),
        caption: (!caption.is_empty()).then(|| caption.join("\n")),
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

fn lower_nested(
    node: &XmlNode,
    depth: usize,
    budget: &mut CellBudget,
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
                out.push(lower_table_with_assets(
                    child,
                    depth,
                    budget,
                    package.as_deref_mut(),
                    image_cache,
                    images,
                    warnings,
                )?);
            } else if child.name == "p" {
                out.extend(lower_nested_paragraph(
                    child,
                    depth,
                    budget,
                    package.as_deref_mut(),
                    image_cache,
                    images,
                    warnings,
                )?);
            } else {
                out.extend(lower_nested(
                    child,
                    depth,
                    budget,
                    package.as_deref_mut(),
                    image_cache,
                    images,
                    warnings,
                )?);
            }
        }
    }
    Ok(out)
}

fn lower_nested_paragraph(
    node: &XmlNode,
    depth: usize,
    budget: &mut CellBudget,
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
        package: &mut Option<&mut Package<'_>>,
        image_cache: &mut ImageCache,
        images: &mut Vec<ExtractedImage>,
        warnings: &mut Vec<ParseWarning>,
        text: &mut String,
        out: &mut Vec<IrBlock>,
    ) -> Result<(), KordocError> {
        for part in &node.content {
            match part {
                XmlContent::Text { start, end } => text.push_str(&node.text[*start..*end]),
                XmlContent::Child(index) => {
                    let child = &node.children[*index];
                    if child.name == "tbl" {
                        if !text.trim().is_empty() {
                            out.push(IrBlock::paragraph(std::mem::take(text)));
                        } else {
                            text.clear();
                        }
                        out.push(lower_table_with_assets(
                            child,
                            depth,
                            budget,
                            package.as_deref_mut(),
                            image_cache,
                            images,
                            warnings,
                        )?);
                    } else if matches!(child.name.as_str(), "pic" | "img" | "imgRect" | "imgClip") {
                        if !text.trim().is_empty() {
                            out.push(IrBlock::paragraph(std::mem::take(text)));
                        } else {
                            text.clear();
                        }
                        if let Some(reference) = image_reference(child) {
                            let block = if let Some(package) = package.as_deref_mut() {
                                resolve_image(
                                    &reference,
                                    None,
                                    package,
                                    image_cache,
                                    images,
                                    warnings,
                                )?
                            } else {
                                image_placeholder(reference)
                            };
                            out.push(block);
                        }
                    } else {
                        walk(
                            child,
                            depth,
                            budget,
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
        &mut package,
        image_cache,
        images,
        warnings,
        &mut text,
        &mut out,
    )?;
    if !text.trim().is_empty() {
        out.push(IrBlock::paragraph(text));
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::hwpx::xml::parse;

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
