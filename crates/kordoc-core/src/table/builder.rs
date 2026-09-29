#![allow(dead_code)] // Exports and ParseSuccess assembly are coordinator integration work.

use kordoc_ir::{ErrorCode, IrBlock, IrBlockType, IrCell, IrTable, KordocError};
use std::collections::HashMap;

pub(crate) const MAX_TABLE_COLS: usize = 200;
pub(crate) const MAX_TABLE_ROWS: usize = 10_000;
pub(crate) const MAX_TABLE_CELLS: usize = 2_000_000;
const MAX_BLOCK_DEPTH: usize = 64;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct CellContext {
    pub text: String,
    pub col_span: u32,
    pub row_span: u32,
    pub col_addr: Option<u32>,
    pub row_addr: Option<u32>,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(crate) struct BuildTableOptions {
    pub max_rows: Option<usize>,
    pub keep_anchored_empty_cols: bool,
    pub keep_empty_paragraphs: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum LegacyLayoutFlattening {
    Never,
    Hwp3OrHwp5,
}

fn too_large(message: &'static str) -> KordocError {
    KordocError::new(ErrorCode::OutputTooLarge, message)
}

fn empty_cell() -> IrCell {
    IrCell {
        text: String::new(),
        col_span: 1,
        row_span: 1,
        blocks: None,
        is_header: None,
    }
}

#[derive(Debug, Clone, Copy)]
struct CellPlacement {
    row: usize,
    col: usize,
    col_span: usize,
    row_span: usize,
    owner: Option<(usize, usize)>,
}

fn simulate_placements(
    rows: &[Vec<CellContext>],
    options: BuildTableOptions,
) -> Result<(usize, usize, Vec<Vec<CellPlacement>>), KordocError> {
    if rows.is_empty() {
        return Ok((0, 0, Vec::new()));
    }
    let row_limit = options
        .max_rows
        .unwrap_or(MAX_TABLE_ROWS)
        .min(MAX_TABLE_CELLS);
    if rows.len() > row_limit {
        return Err(too_large("Table row count exceeds the configured limit"));
    }
    let input_cells = rows
        .iter()
        .try_fold(0usize, |count, row| count.checked_add(row.len()))
        .ok_or_else(|| too_large("Table source cell count overflow"))?;
    if input_cells > MAX_TABLE_CELLS {
        return Err(too_large("Table source cell count exceeds the cell budget"));
    }
    let addressed = rows
        .iter()
        .flatten()
        .any(|cell| cell.col_addr.is_some() || cell.row_addr.is_some());
    let mut row_count = rows.len();
    let mut col_count = 0usize;
    let mut occupied = HashMap::<(usize, usize), (usize, usize)>::new();
    let mut placements = Vec::with_capacity(rows.len());
    for (row_index, row) in rows.iter().enumerate() {
        let mut row_placements = Vec::with_capacity(row.len());
        let mut next_col = 0usize;
        for cell in row {
            if cell.col_span == 0 || cell.row_span == 0 {
                return Err(KordocError::new(
                    ErrorCode::ParseError,
                    "Table spans must be positive",
                ));
            }
            let row = if addressed {
                cell.row_addr.unwrap_or(row_index as u32) as usize
            } else {
                row_index
            };
            if row >= row_limit {
                return Err(too_large("Table row anchor exceeds the row limit"));
            }
            let mut col = if addressed {
                cell.col_addr.map_or(0, |value| value as usize)
            } else {
                next_col
            };
            if !addressed || cell.col_addr.is_none() {
                while col < MAX_TABLE_COLS && occupied.contains_key(&(row, col)) {
                    col += 1;
                }
            }
            if col >= MAX_TABLE_COLS {
                return Err(too_large("Table exceeds the column limit"));
            }
            if let Some(owner) = occupied.get(&(row, col)).copied() {
                row_placements.push(CellPlacement {
                    row,
                    col,
                    col_span: 1,
                    row_span: 1,
                    owner: Some(owner),
                });
                next_col = col
                    .checked_add(cell.col_span as usize)
                    .ok_or_else(|| too_large("Table column overflow"))?;
                continue;
            }
            let mut col_span = cell.col_span as usize;
            let mut row_span = cell.row_span as usize;
            let requested_row_end = row
                .checked_add(row_span)
                .ok_or_else(|| too_large("Table row span overflow"))?;
            let requested_col_end = col
                .checked_add(col_span)
                .ok_or_else(|| too_large("Table column span overflow"))?;
            if requested_row_end > row_limit || requested_col_end > MAX_TABLE_COLS {
                return Err(too_large("Table span exceeds its configured limit"));
            }
            for dc in 1..col_span {
                if occupied.contains_key(&(row, col + dc)) {
                    col_span = dc;
                    break;
                }
            }
            for dr in 1..row_span {
                if (col..col + col_span)
                    .any(|target_col| occupied.contains_key(&(row + dr, target_col)))
                {
                    row_span = dr;
                    break;
                }
            }
            row_count = row_count.max(row + row_span);
            col_count = col_count.max(col + col_span);
            if row_count
                .checked_mul(col_count)
                .is_none_or(|count| count > MAX_TABLE_CELLS)
            {
                return Err(too_large("Table exceeds the cell budget"));
            }
            for dr in 0..row_span {
                for dc in 0..col_span {
                    occupied.insert((row + dr, col + dc), (row, col));
                }
            }
            row_placements.push(CellPlacement {
                row,
                col,
                col_span,
                row_span,
                owner: None,
            });
            next_col = col
                .checked_add(col_span)
                .ok_or_else(|| too_large("Table column overflow"))?;
        }
        placements.push(row_placements);
    }
    Ok((row_count, col_count, placements))
}

/// Build a rectangular IR table while preserving every source cell's text.
pub(crate) fn build_table(
    rows: &[Vec<CellContext>],
    options: BuildTableOptions,
) -> Result<IrTable, KordocError> {
    let (row_count, col_count, placements) = simulate_placements(rows, options)?;
    if row_count == 0 || col_count == 0 {
        return Ok(IrTable::default());
    }

    let mut cells = vec![vec![empty_cell(); col_count]; row_count];
    let mut anchor_cols = vec![false; col_count];

    for (row_index, row) in rows.iter().enumerate() {
        for (source, placement) in row.iter().zip(&placements[row_index]) {
            let r = placement.row;
            let c = placement.col;
            let text = if options.keep_empty_paragraphs {
                source.text.clone()
            } else {
                source.text.trim().to_owned()
            };
            if let Some((owner_row, owner_col)) = placement.owner {
                if !text.trim().is_empty() {
                    let owner = &mut cells[owner_row][owner_col].text;
                    if !owner.is_empty() {
                        owner.push('\n');
                    }
                    owner.push_str(&text);
                }
                continue;
            }
            let col_span = placement.col_span;
            let row_span = placement.row_span;
            let cell = IrCell {
                text,
                col_span: col_span as u32,
                row_span: row_span as u32,
                blocks: None,
                is_header: None,
            };
            cells[r][c] = cell;
            anchor_cols[c] = true;
            for dr in 0..row_span {
                for dc in 0..col_span {
                    if dr != 0 || dc != 0 {
                        cells[r + dr][c + dc] = empty_cell();
                    }
                }
            }
        }
    }

    let mut effective_cols = col_count;
    while effective_cols > 0 {
        let last_is_textless = cells
            .iter()
            .all(|row| row[effective_cols - 1].text.trim().is_empty());
        if !last_is_textless
            || (options.keep_anchored_empty_cols && anchor_cols[effective_cols - 1])
        {
            break;
        }
        effective_cols -= 1;
    }
    if effective_cols == 0 {
        effective_cols = col_count;
    }
    for row in &mut cells {
        row.truncate(effective_cols);
    }
    for row in &mut cells {
        for (col, cell) in row.iter_mut().enumerate() {
            let remaining = effective_cols - col;
            if cell.col_span as usize > remaining {
                cell.col_span = remaining as u32;
            }
        }
    }

    Ok(IrTable {
        rows: row_count as u32,
        cols: effective_cols as u32,
        cells,
        render_as_table: None,
        has_header: row_count > 1,
        classification: None,
        source_id: None,
        regions: None,
        caption: None,
        caption_blocks: None,
    })
}

/// Flatten page-layout tables only when a format adapter explicitly selects the legacy policy.
pub(crate) fn flatten_layout_tables(
    blocks: &[IrBlock],
    policy: LegacyLayoutFlattening,
) -> Result<Vec<IrBlock>, KordocError> {
    let mut out = Vec::new();
    flatten_inner(blocks, policy, 0, &mut out)?;
    Ok(out)
}

fn flatten_inner(
    blocks: &[IrBlock],
    policy: LegacyLayoutFlattening,
    depth: usize,
    out: &mut Vec<IrBlock>,
) -> Result<(), KordocError> {
    if depth > MAX_BLOCK_DEPTH {
        return Err(too_large("Block nesting exceeds the projection limit"));
    }
    for block in blocks {
        let Some(table) = block
            .table
            .as_ref()
            .filter(|_| block.kind == IrBlockType::Table)
        else {
            out.push(block.clone());
            continue;
        };
        if policy == LegacyLayoutFlattening::Never || (table.rows == 1 && table.cols == 1) {
            out.push(block.clone());
            continue;
        }
        let text_len: usize = table
            .cells
            .iter()
            .flatten()
            .map(|cell| cell.text.len())
            .sum();
        let newlines: usize = table
            .cells
            .iter()
            .flatten()
            .map(|cell| cell.text.bytes().filter(|byte| *byte == b'\n').count())
            .sum();
        let nested = table.cells.iter().flatten().any(|cell| {
            cell.blocks.as_ref().is_some_and(|children| {
                children
                    .iter()
                    .any(|child| child.kind == IrBlockType::Table)
            })
        });
        let form_frame = nested && text_len <= 600;
        let is_layout =
            !form_frame && table.cols < 4 && (newlines > 5 || (table.rows <= 2 && text_len > 300));
        if !is_layout || table.rows > 3 {
            out.push(block.clone());
            continue;
        }
        for cell in table.cells.iter().flatten() {
            if let Some(children) = cell
                .blocks
                .as_deref()
                .filter(|children| !children.is_empty())
            {
                flatten_inner(children, policy, depth + 1, out)?;
                continue;
            }
            for line in cell
                .text
                .lines()
                .map(str::trim)
                .filter(|line| !line.is_empty())
            {
                let mut paragraph = IrBlock::paragraph(line);
                paragraph.page_number = block.page_number;
                out.push(paragraph);
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{
        BuildTableOptions, CellContext, LegacyLayoutFlattening, build_table, flatten_layout_tables,
    };
    use kordoc_ir::{ErrorCode, IrBlock};

    fn cell(
        text: &str,
        col_addr: Option<u32>,
        row_addr: Option<u32>,
        col_span: u32,
        row_span: u32,
    ) -> CellContext {
        CellContext {
            text: text.to_owned(),
            col_span,
            row_span,
            col_addr,
            row_addr,
        }
    }

    #[test]
    fn builder_places_addressed_spans_without_losing_collisions() {
        let table = build_table(
            &[
                vec![cell("a", Some(0), Some(0), 1, 2)],
                vec![cell("b", Some(0), Some(1), 1, 1)],
            ],
            BuildTableOptions::default(),
        )
        .unwrap();
        assert_eq!(table.rows, 2);
        assert_eq!(table.cells[0][0].text, "a\nb");
        assert_eq!(table.cells[1][0].text, "");
        assert_eq!(table.cells[0][0].row_span, 2);
    }

    #[test]
    fn builder_keeps_anchored_empty_cols_only_when_requested() {
        let rows = [vec![
            cell("value", Some(0), Some(0), 1, 1),
            cell("", Some(1), Some(0), 1, 1),
        ]];
        let trimmed = build_table(&rows, BuildTableOptions::default()).unwrap();
        let retained = build_table(
            &rows,
            BuildTableOptions {
                keep_anchored_empty_cols: true,
                ..BuildTableOptions::default()
            },
        )
        .unwrap();
        assert_eq!(trimmed.cols, 1);
        assert_eq!(retained.cols, 2);
    }

    #[test]
    fn builder_caps_grid_before_allocation() {
        let rows = [vec![cell("wide", Some(0), Some(0), u32::MAX, u32::MAX)]];
        assert_eq!(
            build_table(&rows, BuildTableOptions::default())
                .unwrap_err()
                .code,
            ErrorCode::OutputTooLarge
        );
    }

    #[test]
    fn builder_preflights_actual_mixed_address_placement() {
        let rows = [
            vec![
                cell("reserved", Some(0), Some(0), 1, 2),
                cell("implicit col", None, Some(0), 1, 1),
            ],
            vec![cell("implicit row", Some(1), None, 1, 2)],
        ];
        let table = build_table(&rows, BuildTableOptions::default()).unwrap();
        assert_eq!((table.rows, table.cols), (3, 2));
        assert_eq!(table.cells[0][1].text, "implicit col");
        assert_eq!(table.cells[1][1].text, "implicit row");
    }

    #[test]
    fn builder_retains_empty_grid_geometry_when_every_cell_is_blank() {
        let rows = [
            vec![cell("", None, None, 1, 1), cell("", None, None, 1, 1)],
            vec![cell("", None, None, 1, 1), cell("", None, None, 1, 1)],
        ];
        let table = build_table(&rows, BuildTableOptions::default()).unwrap();
        assert_eq!((table.rows, table.cols), (2, 2));
        assert!(
            table
                .cells
                .iter()
                .flatten()
                .all(|cell| cell.text.is_empty())
        );
    }

    #[test]
    fn flatten_layout_tables_only_with_explicit_parser_evidence() {
        let body = (0..7)
            .map(|n| format!("line {n}"))
            .collect::<Vec<_>>()
            .join("\n");
        let table_block: IrBlock = serde_json::from_value(serde_json::json!({
            "type":"table", "table":{"rows":2,"cols":1,"hasHeader":true,"cells":[
                [{"text":"header","colSpan":1,"rowSpan":1}],
                [{"text":body,"colSpan":1,"rowSpan":1}]
            ]}
        }))
        .unwrap();
        let generic = flatten_layout_tables(
            std::slice::from_ref(&table_block),
            LegacyLayoutFlattening::Never,
        )
        .unwrap();
        let legacy =
            flatten_layout_tables(&[table_block], LegacyLayoutFlattening::Hwp3OrHwp5).unwrap();
        assert_eq!(generic.len(), 1);
        assert!(legacy.len() > 1);
        assert!(legacy.iter().all(|block| block.table.is_none()));
    }
}
