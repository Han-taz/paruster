#![allow(dead_code)] // Core table projection policy remains private until facade assembly.

use kordoc_ir::{ErrorCode, IrBlock, IrBlockType, IrTable, KordocError};

pub(crate) use kordoc_tables::{BuildTableOptions, CellContext};

const MAX_BLOCK_DEPTH: usize = 64;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum LegacyLayoutFlattening {
    Never,
    Hwp3OrHwp5,
}

fn too_large(message: &'static str) -> KordocError {
    KordocError::new(ErrorCode::OutputTooLarge, message)
}

pub(crate) fn build_table(
    rows: &[Vec<CellContext>],
    options: BuildTableOptions,
) -> Result<IrTable, KordocError> {
    kordoc_tables::build_table(rows, options)
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
    use kordoc_ir::{ErrorCode, IrBlock, IrBlockType, IrCell, IrTable};

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

    fn nested_table_block(depth: usize) -> IrBlock {
        let cell = if depth == 0 {
            IrCell {
                text: "leaf".to_owned(),
                col_span: 1,
                row_span: 1,
                ..IrCell::default()
            }
        } else {
            IrCell {
                col_span: 1,
                row_span: 1,
                blocks: Some(vec![nested_table_block(depth - 1)]),
                ..IrCell::default()
            }
        };
        IrBlock {
            kind: IrBlockType::Table,
            table: Some(IrTable {
                rows: 1,
                cols: 1,
                cells: vec![vec![cell]],
                ..IrTable::default()
            }),
            ..IrBlock::default()
        }
    }

    #[test]
    fn rich_builder_depth_counts_outer_table_against_markdown_projection_limit() {
        #[derive(Default)]
        struct NoopAllocationBudget;
        impl kordoc_tables::AllocationBudget for NoopAllocationBudget {
            fn charge_bytes(&mut self, _: usize) -> Result<(), kordoc_ir::KordocError> {
                Ok(())
            }
        }
        let mut meter = NoopAllocationBudget;
        let mut build = |depth: usize| {
            kordoc_tables::build_table_with_blocks(
                vec![vec![kordoc_tables::RichCellContext {
                    cell: cell("outer", Some(0), Some(0), 1, 1),
                    blocks: Some(vec![nested_table_block(depth)]),
                }]],
                BuildTableOptions::default(),
                &mut kordoc_tables::TableCellBudget::default(),
                &mut meter,
            )
        };
        let valid = build(62).unwrap();
        let valid_block = IrBlock {
            kind: IrBlockType::Table,
            table: Some(valid),
            ..IrBlock::default()
        };
        assert!(crate::markdown::blocks_to_markdown(&[valid_block]).is_ok());

        assert_eq!(build(63).err().unwrap().code, ErrorCode::OutputTooLarge);
        let invalid_block = IrBlock {
            kind: IrBlockType::Table,
            table: Some(IrTable {
                rows: 1,
                cols: 1,
                cells: vec![vec![IrCell {
                    col_span: 1,
                    row_span: 1,
                    blocks: Some(vec![nested_table_block(63)]),
                    ..IrCell::default()
                }]],
                ..IrTable::default()
            }),
            ..IrBlock::default()
        };
        assert_eq!(
            crate::markdown::blocks_to_markdown(&[invalid_block])
                .unwrap_err()
                .code,
            ErrorCode::OutputTooLarge
        );
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
