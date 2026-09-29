//! Deterministic, bounded table classification and depth-first table traversal.

use std::collections::HashMap;

use kordoc_ir::{
    ClassifyContext, ErrorCode, IrBlock, IrBlockType, IrCell, IrTable, KordocError,
    TableClassificationKind, TableClassificationReason, TableClassificationSummary,
};

pub const MAX_TABLE_DEPTH: usize = 64;
pub const MAX_TABLE_CELLS: u64 = 2_000_000;
const MAX_TABLE_COUNT: usize = 100_000;
const KEYWORD_GATE: f64 = 0.3;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
enum CellType {
    Empty,
    Number,
    Date,
    Short,
    Long,
}

#[derive(Clone, Copy)]
struct Anchor<'a> {
    row: usize,
    col: usize,
    cell: &'a IrCell,
    kind: CellType,
}

fn output_too_large(message: impl Into<String>) -> KordocError {
    KordocError::new(ErrorCode::OutputTooLarge, message)
}

fn checked_slots(table: &IrTable) -> Result<usize, KordocError> {
    let slots = u64::from(table.rows)
        .checked_mul(u64::from(table.cols))
        .ok_or_else(|| output_too_large("table cell count overflow"))?;
    if slots > MAX_TABLE_CELLS {
        return Err(output_too_large("table exceeds the 2,000,000 cell limit"));
    }
    usize::try_from(slots).map_err(|_| output_too_large("table cell count is not addressable"))
}

fn cell_type(cell: &IrCell) -> CellType {
    let text = cell.text.trim();
    if text.is_empty()
        && !cell
            .blocks
            .as_ref()
            .is_some_and(|blocks| !blocks.is_empty())
    {
        return CellType::Empty;
    }
    if text.is_empty() {
        return CellType::Short;
    }
    if is_number(text) {
        return CellType::Number;
    }
    if is_date(text) {
        return CellType::Date;
    }
    if text.chars().count() <= 12 {
        CellType::Short
    } else {
        CellType::Long
    }
}

fn is_number(text: &str) -> bool {
    let text = text.trim();
    let text = text
        .strip_suffix('%')
        .or_else(|| text.strip_suffix("백만원"))
        .or_else(|| text.strip_suffix("천원"))
        .or_else(|| text.strip_suffix("㎡"))
        .or_else(|| text.strip_suffix("㎢"))
        .or_else(|| text.strip_suffix("시간"))
        .or_else(|| text.strip_suffix("원"))
        .or_else(|| text.strip_suffix("명"))
        .or_else(|| text.strip_suffix("건"))
        .or_else(|| text.strip_suffix("개"))
        .or_else(|| text.strip_suffix("kg"))
        .or_else(|| text.strip_suffix("km"))
        .or_else(|| text.strip_suffix('일'))
        .unwrap_or(text)
        .trim();
    let text = text
        .strip_prefix('-')
        .or_else(|| text.strip_prefix('+'))
        .unwrap_or(text)
        .trim();
    let mut parts = text.split('.');
    let integer = parts.next().unwrap_or_default();
    let fractional = parts.next();
    if parts.next().is_some()
        || integer.is_empty()
        || !integer.chars().any(|ch| ch.is_ascii_digit())
    {
        return false;
    }
    if !integer.chars().all(|ch| ch.is_ascii_digit() || ch == ',') {
        return false;
    }
    fractional.is_none_or(|value| !value.is_empty() && value.chars().all(|ch| ch.is_ascii_digit()))
}

fn is_date(text: &str) -> bool {
    let chars: Vec<char> = text.chars().collect();
    if chars.len() < 4 || chars.len() > 18 {
        return false;
    }
    let first_sep = chars
        .iter()
        .position(|c| matches!(c, '.' | '-' | '/' | '년'));
    let Some(first_sep) = first_sep else {
        return false;
    };
    if !(2..=4).contains(&first_sep) || !chars[..first_sep].iter().all(char::is_ascii_digit) {
        return false;
    }
    let rest: String = chars[first_sep + 1..].iter().collect();
    let month_end = rest.find(['.', '-', '/', '월', '일']).unwrap_or(rest.len());
    let month_text = rest[..month_end].trim();
    !month_text.is_empty()
        && month_text
            .chars()
            .all(|c| c.is_ascii_digit() || c.is_whitespace())
}

fn anchors(table: &IrTable) -> Result<Vec<Anchor<'_>>, KordocError> {
    let slots = checked_slots(table)?;
    let rows = usize::try_from(table.rows)
        .map_err(|_| output_too_large("table row count is not addressable"))?;
    let cols = usize::try_from(table.cols)
        .map_err(|_| output_too_large("table column count is not addressable"))?;
    let mut covered = vec![false; slots];
    let mut output = Vec::new();
    for row in 0..rows {
        for col in 0..cols {
            let index = row * cols + col;
            if covered[index] {
                continue;
            }
            let Some(cell) = table.cells.get(row).and_then(|line| line.get(col)) else {
                continue;
            };
            if cell.row_span > table.rows || cell.col_span > table.cols {
                return Err(output_too_large("cell span exceeds table dimensions"));
            }
            let row_end = row.saturating_add(cell.row_span as usize).min(rows);
            let col_end = col.saturating_add(cell.col_span as usize).min(cols);
            for covered_row in row..row_end {
                for covered_col in col..col_end {
                    if covered_row != row || covered_col != col {
                        covered[covered_row * cols + covered_col] = true;
                    }
                }
            }
            output.push(Anchor {
                row,
                col,
                cell,
                kind: cell_type(cell),
            });
        }
    }
    Ok(output)
}

fn is_wrapper(table: &IrTable) -> bool {
    table.rows == 1
        && table.cols == 1
        && table
            .cells
            .first()
            .and_then(|row| row.first())
            .is_some_and(|cell| {
                cell.blocks.as_ref().is_some_and(|blocks| {
                    blocks
                        .iter()
                        .any(|block| block.kind == IrBlockType::Table && block.table.is_some())
                })
            })
}

fn round2(value: f64) -> f64 {
    (value * 100.0).round() / 100.0
}

fn summary(
    kind: TableClassificationKind,
    semantic: f64,
    non_tabular: f64,
    reasons: Vec<TableClassificationReason>,
) -> TableClassificationSummary {
    TableClassificationSummary {
        kind,
        confidence: round2((semantic - non_tabular).abs().min(1.0)),
        semantic_score: round2(semantic),
        non_tabular_score: round2(non_tabular),
        reasons,
    }
}

pub fn classify_table(
    table: &IrTable,
    context: Option<&ClassifyContext>,
) -> Result<TableClassificationSummary, KordocError> {
    classify_table_inner(table, context, None)
}

#[cfg(test)]
fn classify_table_with_inspection_count(
    table: &IrTable,
    context: Option<&ClassifyContext>,
) -> Result<(TableClassificationSummary, u64), KordocError> {
    let mut inspections = 0;
    let summary = classify_table_inner(table, context, Some(&mut inspections))?;
    Ok((summary, inspections))
}

fn record_inspections(counter: &mut Option<&mut u64>, amount: usize) {
    if let Some(counter) = counter.as_deref_mut() {
        *counter += amount as u64;
    }
}

fn classify_table_inner(
    table: &IrTable,
    context: Option<&ClassifyContext>,
    mut inspections: Option<&mut u64>,
) -> Result<TableClassificationSummary, KordocError> {
    checked_slots(table)?;
    let mut reasons = Vec::new();
    if is_wrapper(table) {
        reasons.push(TableClassificationReason::NestedStructureWrapper);
        return Ok(summary(
            TableClassificationKind::NonTabularLayout,
            0.05,
            0.85,
            reasons,
        ));
    }
    let anchors = anchors(table)?;
    let slots = u64::from(table.rows) * u64::from(table.cols);
    record_inspections(&mut inspections, slots as usize);
    if slots <= 4 || anchors.len() <= 2 {
        reasons.push(TableClassificationReason::LowEvidence);
        return Ok(summary(
            TableClassificationKind::Uncertain,
            0.2,
            0.2,
            reasons,
        ));
    }

    let rows = table.rows as usize;
    let cols = table.cols as usize;
    let mut row_buckets = vec![Vec::new(); rows];
    let mut col_buckets = vec![Vec::new(); cols];
    let mut row_has_content = vec![false; rows];
    let mut col_has_content = vec![false; cols];
    let mut anchors_per_row = vec![0usize; rows];
    let mut active_slots = 0u64;
    let mut body_count = 0usize;
    let mut body_merged = 0usize;
    let mut grid_regular_count = 0usize;
    let mut nested_cells = 0usize;
    for (anchor_index, anchor) in anchors.iter().enumerate() {
        record_inspections(&mut inspections, 1);
        row_buckets[anchor.row].push(anchor_index);
        col_buckets[anchor.col].push(anchor_index);
        anchors_per_row[anchor.row] += 1;
        if anchor.row > 0 {
            body_count += 1;
            if anchor.cell.col_span > 1 || anchor.cell.row_span > 1 {
                body_merged += 1;
            }
        }
        if (anchor.cell.col_span == 1 && anchor.cell.row_span == 1)
            || (anchor.row == 0 && anchor.cell.row_span == 1)
        {
            grid_regular_count += 1;
        }
        if anchor.cell.blocks.as_ref().is_some_and(|blocks| {
            blocks
                .iter()
                .any(|block| block.kind == IrBlockType::Table && block.table.is_some())
        }) {
            nested_cells += 1;
        }
        if anchor.kind != CellType::Empty {
            active_slots += u64::from(anchor.cell.col_span) * u64::from(anchor.cell.row_span);
            let row_end = anchor
                .row
                .saturating_add(anchor.cell.row_span as usize)
                .min(rows);
            let col_end = anchor
                .col
                .saturating_add(anchor.cell.col_span as usize)
                .min(cols);
            row_has_content[anchor.row..row_end].fill(true);
            col_has_content[anchor.col..col_end].fill(true);
        }
    }
    let active_ratio = active_slots as f64 / slots as f64;
    let spacer_rows = row_has_content.iter().filter(|value| !**value).count();
    let spacer_cols = col_has_content.iter().filter(|value| !**value).count();
    let spacer_bands = (spacer_rows + spacer_cols) as f64 / (rows + cols) as f64;

    let counts: Vec<f64> = anchors_per_row
        .into_iter()
        .filter(|count| *count > 0)
        .map(|count| count as f64)
        .collect();
    let mean_count = counts.iter().sum::<f64>() / counts.len().max(1) as f64;
    let row_variance = if counts.len() > 1 {
        counts
            .iter()
            .map(|count| (count - mean_count).abs())
            .sum::<f64>()
            / counts.len() as f64
            / mean_count.max(1.0)
    } else {
        0.0
    };
    let span_irregularity = (((if body_count == 0 {
        0.0
    } else {
        body_merged as f64 / body_count as f64
    }) * 0.7)
        + row_variance.min(1.0) * 0.6)
        .min(1.0);
    let grid_regularity = if anchors.is_empty() {
        0.0
    } else {
        grid_regular_count as f64 / anchors.len() as f64
    };

    let mut repeated_row_schema: f64 = 0.0;
    if table.rows >= 3 {
        let mut signatures: HashMap<String, usize> = HashMap::new();
        let mut count = 0;
        for row_anchors in row_buckets.iter().skip(1) {
            let mut row_signature = Vec::new();
            for anchor_index in row_anchors {
                record_inspections(&mut inspections, 1);
                let anchor = &anchors[*anchor_index];
                if anchor.kind == CellType::Empty {
                    continue;
                }
                let kind = if anchor.kind == CellType::Long {
                    CellType::Short
                } else {
                    anchor.kind
                };
                row_signature.push(format!("{}:{}", anchor.col, cell_type_name(kind)));
            }
            if !row_signature.is_empty() {
                *signatures.entry(row_signature.join("|")).or_default() += 1;
                count += 1;
            }
        }
        if let Some(max_frequency) = signatures.values().max() {
            repeated_row_schema = *max_frequency as f64 / count as f64;
            if count < 2 {
                repeated_row_schema *= 0.5;
            }
        }
    }

    let mut column_consistency = 0.0;
    let mut consistent_columns = Vec::new();
    for col_anchors in &col_buckets {
        let mut kinds = Vec::new();
        for anchor_index in col_anchors {
            record_inspections(&mut inspections, 1);
            let anchor = &anchors[*anchor_index];
            if anchor.row > 0 && anchor.cell.col_span == 1 && anchor.kind != CellType::Empty {
                kinds.push(anchor.kind);
            }
        }
        if kinds.len() < 2 {
            continue;
        }
        let mut frequencies: HashMap<CellType, usize> = HashMap::new();
        for kind in kinds.iter().copied() {
            *frequencies.entry(kind).or_default() += 1;
        }
        consistent_columns
            .push(*frequencies.values().max().unwrap_or(&0) as f64 / kinds.len() as f64);
    }
    if !consistent_columns.is_empty() {
        column_consistency =
            consistent_columns.iter().sum::<f64>() / consistent_columns.len() as f64;
    }
    let sparsity = ((0.6 - active_ratio) / 0.6).max(0.0);
    let empty_share = 1.0 - active_ratio;
    let mut semantic = (0.35 * repeated_row_schema
        + 0.25 * grid_regularity
        + 0.2 * (active_ratio / 0.7).min(1.0)
        + 0.2 * column_consistency)
        * (1.0 - 0.6 * (empty_share - 0.2).max(0.0) / 0.8)
        * (1.0 - spacer_bands)
        * (1.0 - 0.5 * span_irregularity);
    let mut non_tabular = 0.6 * span_irregularity + 0.4 * spacer_bands + 0.5 * sparsity;
    if nested_cells > 0 && (nested_cells as f64) >= anchors.len() as f64 * 0.5 {
        non_tabular += 0.25;
        reasons.push(TableClassificationReason::NestedStructureWrapper);
    }
    if repeated_row_schema >= 0.6 {
        reasons.push(TableClassificationReason::RepeatedRowSchema);
    }
    if grid_regularity >= 0.9 {
        reasons.push(TableClassificationReason::GridRegularity);
    }
    if active_ratio >= 0.7 {
        reasons.push(TableClassificationReason::HighActiveDensity);
    }
    if column_consistency >= 0.75 {
        reasons.push(TableClassificationReason::ColumnTypeConsistency);
    }
    if span_irregularity >= 0.35 {
        reasons.push(TableClassificationReason::SpanIrregularity);
    }
    if spacer_bands >= 0.25 {
        reasons.push(TableClassificationReason::SpacerBands);
    }
    if sparsity >= 0.5 {
        reasons.push(TableClassificationReason::ExtremeSparsity);
    }

    let nearby = context
        .and_then(|context| context.nearby_text.as_ref())
        .into_iter()
        .flatten()
        .map(String::as_str);
    let keyword_present = table
        .caption
        .as_deref()
        .into_iter()
        .chain(nearby)
        .any(contains_diagram_keyword);
    if keyword_present && non_tabular >= KEYWORD_GATE {
        non_tabular += 0.15;
        reasons.push(TableClassificationReason::DiagramContextKeyword);
    }
    semantic = semantic.min(1.0);
    non_tabular = non_tabular.min(1.0);
    if semantic >= 0.55 && semantic - non_tabular >= 0.2 {
        return Ok(summary(
            TableClassificationKind::SemanticTable,
            semantic,
            non_tabular,
            reasons,
        ));
    }
    if non_tabular >= 0.45 && non_tabular - semantic >= 0.15 {
        return Ok(summary(
            TableClassificationKind::NonTabularLayout,
            semantic,
            non_tabular,
            reasons,
        ));
    }
    reasons.push(if semantic.max(non_tabular) < 0.35 {
        TableClassificationReason::LowEvidence
    } else {
        TableClassificationReason::AmbiguousScores
    });
    Ok(summary(
        TableClassificationKind::Uncertain,
        semantic,
        non_tabular,
        reasons,
    ))
}

fn cell_type_name(kind: CellType) -> &'static str {
    match kind {
        CellType::Empty => "empty",
        CellType::Number => "number",
        CellType::Date => "date",
        CellType::Short => "short",
        CellType::Long => "long",
    }
}

fn contains_diagram_keyword(text: &str) -> bool {
    let patterns = ["조직도", "연락망", "체계도", "기구표", "배치도", "흐름도"];
    if patterns.iter().any(|pattern| text.contains(pattern)) {
        return true;
    }
    contains_spaced_keyword(text, "비상", "연락망")
        || contains_spaced_keyword(text, "업무", "체계도")
        || contains_spaced_keyword(text, "추진", "체계")
}

fn contains_spaced_keyword(text: &str, left: &str, right: &str) -> bool {
    text.match_indices(left)
        .any(|(index, _)| text[index + left.len()..].trim_start().starts_with(right))
}

fn check_depth(depth: usize) -> Result<(), KordocError> {
    if depth > MAX_TABLE_DEPTH {
        Err(output_too_large("table traversal exceeds depth 64"))
    } else {
        Ok(())
    }
}

fn check_table_tree(
    blocks: &[IrBlock],
    depth: usize,
    table_count: &mut usize,
) -> Result<(), KordocError> {
    check_depth(depth)?;
    for block in blocks {
        if let Some(table) = block
            .table
            .as_ref()
            .filter(|_| block.kind == IrBlockType::Table)
        {
            *table_count += 1;
            if *table_count > MAX_TABLE_COUNT {
                return Err(output_too_large("table traversal exceeds 100,000 tables"));
            }
            checked_slots(table)?;
            for row in &table.cells {
                for cell in row {
                    if let Some(children) = cell.blocks.as_deref() {
                        check_table_tree(children, depth + 1, table_count)?;
                    }
                }
            }
            if let Some(caption_blocks) = table.caption_blocks.as_deref() {
                check_table_tree(caption_blocks, depth + 1, table_count)?;
            }
        }
        if let Some(children) = block.children.as_deref() {
            check_table_tree(children, depth + 1, table_count)?;
        }
    }
    Ok(())
}

fn collect_inner<'a>(blocks: &'a [IrBlock], out: &mut Vec<&'a IrBlock>) {
    for block in blocks {
        if block.kind == IrBlockType::Table
            && let Some(table) = block.table.as_ref()
        {
            out.push(block);
            for row in &table.cells {
                for cell in row {
                    if let Some(children) = cell.blocks.as_deref() {
                        collect_inner(children, out);
                    }
                }
            }
            if let Some(caption_blocks) = table.caption_blocks.as_deref() {
                collect_inner(caption_blocks, out);
            }
        }
        if let Some(children) = block.children.as_deref() {
            collect_inner(children, out);
        }
    }
}

pub fn collect_table_blocks(blocks: &[IrBlock]) -> Result<Vec<&IrBlock>, KordocError> {
    let mut table_count = 0;
    check_table_tree(blocks, 0, &mut table_count)?;
    let mut output = Vec::with_capacity(table_count);
    collect_inner(blocks, &mut output);
    Ok(output)
}

fn classify_in_list(
    blocks: &mut [IrBlock],
    depth: usize,
    table_count: &mut usize,
) -> Result<(), KordocError> {
    check_depth(depth)?;
    for index in 0..blocks.len() {
        let context = ClassifyContext {
            nearby_text: [index.checked_sub(2), index.checked_sub(1), Some(index + 1)]
                .into_iter()
                .flatten()
                .filter_map(|sibling_index| blocks.get(sibling_index))
                .filter(|neighbor| neighbor.kind != IrBlockType::Table)
                .filter_map(|neighbor| neighbor.text.clone())
                .collect::<Vec<_>>()
                .into(),
        };
        let block = &mut blocks[index];
        if block.kind == IrBlockType::Table
            && let Some(table) = block.table.as_mut()
        {
            *table_count += 1;
            if *table_count > MAX_TABLE_COUNT {
                return Err(output_too_large("table traversal exceeds 100,000 tables"));
            }
            checked_slots(table)?;
            table.classification = Some(classify_table(table, Some(&context))?);
            for row in &mut table.cells {
                for cell in row {
                    if let Some(children) = cell.blocks.as_deref_mut() {
                        classify_in_list(children, depth + 1, table_count)?;
                    }
                }
            }
            if let Some(caption_blocks) = table.caption_blocks.as_deref_mut() {
                classify_in_list(caption_blocks, depth + 1, table_count)?;
            }
        }
        if let Some(children) = block.children.as_deref_mut() {
            classify_in_list(children, depth + 1, table_count)?;
        }
    }
    Ok(())
}

/// Adds only classification summaries, preserving block order and source text.
pub fn classify_table_tree(blocks: &mut [IrBlock]) -> Result<(), KordocError> {
    let mut validation_tables = 0;
    check_table_tree(blocks, 0, &mut validation_tables)?;
    let mut table_count = 0;
    classify_in_list(blocks, 0, &mut table_count)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn table(rows: Vec<Vec<serde_json::Value>>) -> IrTable {
        let cells = rows.len();
        let cols = rows.first().map_or(0, Vec::len);
        let json_cells = rows
            .into_iter()
            .map(|row| {
                row.into_iter()
                    .map(|value| {
                        if value.is_string() {
                            json!({"text": value, "colSpan": 1, "rowSpan": 1})
                        } else {
                            value
                        }
                    })
                    .collect::<Vec<_>>()
            })
            .collect::<Vec<_>>();
        serde_json::from_value(
            json!({"rows": cells, "cols": cols, "hasHeader": cells > 1, "cells": json_cells}),
        )
        .unwrap()
    }

    fn texts(rows: &[&[&str]]) -> IrTable {
        table(
            rows.iter()
                .map(|row| row.iter().map(|text| json!(text)).collect())
                .collect(),
        )
    }

    fn kind(summary: &TableClassificationSummary) -> TableClassificationKind {
        summary.kind
    }

    #[test]
    fn classifier_matches_dense_sparse_wrapper_and_ambiguous_matrix() {
        let dense = texts(&[
            &["이름", "부서", "인원"],
            &["홍길동", "개발", "10"],
            &["김철수", "기획", "7"],
            &["이영희", "영업", "12"],
            &["박민수", "총무", "3"],
        ]);
        let dense_summary = classify_table(&dense, None).unwrap();
        assert_eq!(kind(&dense_summary), TableClassificationKind::SemanticTable);
        assert_eq!(
            (
                dense_summary.semantic_score,
                dense_summary.non_tabular_score
            ),
            (1.0, 0.0)
        );

        let sparse = texts(&[
            &["", "", "기관장", "", ""],
            &["", "", "", "", ""],
            &["", "기획본부", "", "사업본부", ""],
            &["", "", "", "", ""],
            &["기획팀", "", "총무팀", "", "사업팀"],
            &["", "", "", "", ""],
            &["", "", "", "", ""],
        ]);
        let sparse_summary = classify_table(&sparse, None).unwrap();
        assert_eq!(
            kind(&sparse_summary),
            TableClassificationKind::NonTabularLayout
        );
        assert_eq!(
            (
                sparse_summary.semantic_score,
                sparse_summary.non_tabular_score,
                sparse_summary.confidence
            ),
            (0.17, 0.49, 0.32),
        );
        assert_eq!(
            sparse_summary.reasons,
            [
                TableClassificationReason::GridRegularity,
                TableClassificationReason::SpacerBands,
                TableClassificationReason::ExtremeSparsity,
            ]
        );

        let sparse: IrTable = serde_json::from_value(json!({"rows":1,"cols":1,"hasHeader":false,"cells":[[{"text":"","colSpan":1,"rowSpan":1,"blocks":[{"type":"table","table":{"rows":0,"cols":0,"hasHeader":false,"cells":[]}}]}]]})).unwrap();
        let wrapper_summary = classify_table(&sparse, None).unwrap();
        assert_eq!(
            kind(&wrapper_summary),
            TableClassificationKind::NonTabularLayout
        );
        assert_eq!(
            (
                wrapper_summary.semantic_score,
                wrapper_summary.non_tabular_score
            ),
            (0.05, 0.85)
        );
        assert_eq!(
            wrapper_summary.reasons,
            [TableClassificationReason::NestedStructureWrapper]
        );

        let small = texts(&[&["a", "b"], &["c", "d"]]);
        let uncertain = classify_table(&small, None).unwrap();
        assert_eq!(kind(&uncertain), TableClassificationKind::Uncertain);
        assert_eq!(
            (uncertain.semantic_score, uncertain.non_tabular_score),
            (0.2, 0.2)
        );
        assert_eq!(uncertain.reasons, [TableClassificationReason::LowEvidence]);
    }

    #[test]
    fn classifier_preserves_reason_order_and_score_rounding() {
        let dense = texts(&[
            &["이름", "부서", "인원"],
            &["홍길동", "개발", "10"],
            &["김철수", "기획", "7"],
            &["이영희", "영업", "12"],
            &["박민수", "총무", "3"],
        ]);
        let result = classify_table(&dense, None).unwrap();
        assert_eq!(
            result.reasons,
            [
                TableClassificationReason::RepeatedRowSchema,
                TableClassificationReason::GridRegularity,
                TableClassificationReason::HighActiveDensity,
                TableClassificationReason::ColumnTypeConsistency,
            ]
        );
        assert_eq!(result.confidence, 1.0);
        let table = texts(&[
            &["", "당직실 02-120", "", "", ""],
            &["", "", "", "", ""],
            &["", "총무과", "", "시설과", "보안과"],
            &["", "", "", "", ""],
            &["", "", "", "", ""],
        ]);
        let with_keyword = classify_table(
            &table,
            Some(&ClassifyContext {
                nearby_text: Some(vec!["비상연락망".into()]),
            }),
        )
        .unwrap();
        assert_eq!(
            with_keyword.reasons.last(),
            Some(&TableClassificationReason::DiagramContextKeyword)
        );
        assert_eq!(with_keyword.non_tabular_score, 0.72);
    }

    #[test]
    fn classifier_keyword_needs_structure() {
        let table = texts(&[
            &["이름", "부서", "인원"],
            &["홍길동", "개발", "10"],
            &["김철수", "기획", "7"],
            &["이영희", "영업", "12"],
        ]);
        let result = classify_table(
            &table,
            Some(&ClassifyContext {
                nearby_text: Some(vec!["조직도".into()]),
            }),
        )
        .unwrap();
        assert_eq!(result.kind, TableClassificationKind::SemanticTable);
        assert!(
            !result
                .reasons
                .contains(&TableClassificationReason::DiagramContextKeyword)
        );
    }

    #[test]
    fn collect_tables_depth_first_including_caption_and_children() {
        let nested = texts(&[&["a", "b"], &["1", "2"], &["3", "4"]]);
        let outer: IrBlock = serde_json::from_value(json!({"type":"table","table":{"rows":1,"cols":1,"hasHeader":false,"cells":[[{"text":"","colSpan":1,"rowSpan":1,"blocks":[{"type":"table","table":nested}]}]],"captionBlocks":[{"type":"table","table":nested}]},"children":[{"type":"table","table":nested}]})).unwrap();
        let blocks = [outer];
        let tables = collect_table_blocks(&blocks).unwrap();
        assert_eq!(tables.len(), 4);
        assert_eq!(tables[0].table.as_ref().unwrap().rows, 1);
        assert!(
            tables[1..]
                .iter()
                .all(|block| block.table.as_ref().unwrap().rows == 3)
        );
    }

    #[test]
    fn classify_tree_is_opt_in_and_preserves_original_text() {
        let mut blocks = vec![
            IrBlock::paragraph("keep me"),
            IrBlock {
                kind: IrBlockType::Table,
                table: Some(texts(&[&["a", "b"], &["c", "d"]])),
                ..IrBlock::default()
            },
        ];
        assert!(blocks[1].table.as_ref().unwrap().classification.is_none());
        classify_table_tree(&mut blocks).unwrap();
        assert_eq!(blocks[0].text.as_deref(), Some("keep me"));
        assert!(blocks[1].table.as_ref().unwrap().classification.is_some());
    }

    #[test]
    fn classifier_enforces_depth_and_cell_budgets() {
        let mut block = IrBlock {
            kind: IrBlockType::Table,
            table: Some(texts(&[&["a"]])),
            ..IrBlock::default()
        };
        for _ in 0..=MAX_TABLE_DEPTH {
            block = IrBlock {
                children: Some(vec![block]),
                ..IrBlock::default()
            };
        }
        assert_eq!(
            classify_table_tree(&mut [block]).unwrap_err().code,
            ErrorCode::OutputTooLarge
        );
        let too_wide: IrTable =
            serde_json::from_value(json!({"rows":2001,"cols":1000,"hasHeader":false,"cells":[]}))
                .unwrap();
        assert_eq!(
            classify_table(&too_wide, None).unwrap_err().code,
            ErrorCode::OutputTooLarge
        );
        let large: IrTable =
            serde_json::from_value(json!({"rows":1001,"cols":1000,"hasHeader":false,"cells":[]}))
                .unwrap();
        let mut aggregate = vec![
            IrBlock {
                kind: IrBlockType::Table,
                table: Some(large.clone()),
                ..IrBlock::default()
            },
            IrBlock {
                kind: IrBlockType::Table,
                table: Some(large),
                ..IrBlock::default()
            },
        ];
        assert!(classify_table_tree(&mut aggregate).is_ok());
    }

    #[test]
    fn classifier_anchor_inspections_scale_linearly_with_large_shape() {
        let rows = 240u32;
        let cols = 240u32;
        let cell = IrCell {
            text: "x".into(),
            col_span: 1,
            row_span: 1,
            ..IrCell::default()
        };
        let table = IrTable {
            rows,
            cols,
            cells: vec![vec![cell; cols as usize]; rows as usize],
            has_header: true,
            ..IrTable::default()
        };
        let (summary, inspections) = classify_table_with_inspection_count(&table, None).unwrap();
        assert_eq!(summary.kind, TableClassificationKind::SemanticTable);
        let slots = u64::from(rows) * u64::from(cols);
        assert!(
            inspections <= 5 * slots + 4 * u64::from(rows + cols),
            "inspected {inspections} anchors for {slots} cells"
        );
    }
}
