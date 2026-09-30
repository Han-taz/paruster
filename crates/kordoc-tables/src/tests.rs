use super::{
    AllocationBudget, BuildTableOptions, CellContext, RichCellContext, TableCellBudget,
    build_table, build_table_with_blocks, build_table_with_budget,
};
use kordoc_ir::{ErrorCode, IrBlock, IrBlockType, IrTable, KordocError};

#[derive(Default)]
struct Meter {
    limit: usize,
    used: usize,
    calls: usize,
}

impl Meter {
    fn unlimited() -> Self {
        Self {
            limit: usize::MAX,
            ..Self::default()
        }
    }
}

impl AllocationBudget for Meter {
    fn charge_bytes(&mut self, bytes: usize) -> Result<(), KordocError> {
        self.calls += 1;
        let next = self.used.checked_add(bytes).ok_or_else(too_large)?;
        if next > self.limit {
            return Err(too_large());
        }
        self.used = next;
        Ok(())
    }
}

fn too_large() -> KordocError {
    KordocError::new(ErrorCode::OutputTooLarge, "allocation test budget exceeded")
}

fn cell(text: &str, col: Option<u32>, row: Option<u32>, colspan: u32, rowspan: u32) -> CellContext {
    CellContext {
        text: text.to_owned(),
        col_span: colspan,
        row_span: rowspan,
        col_addr: col,
        row_addr: row,
    }
}

fn nested_table_block(text: &str) -> IrBlock {
    IrBlock {
        kind: IrBlockType::Table,
        table: Some(IrTable {
            rows: 1,
            cols: 1,
            cells: vec![vec![kordoc_ir::IrCell {
                text: text.to_owned(),
                col_span: 1,
                row_span: 1,
                ..kordoc_ir::IrCell::default()
            }]],
            ..IrTable::default()
        }),
        ..IrBlock::default()
    }
}

#[test]
fn metered_builder_charges_grid_before_reserving_and_is_inclusive() {
    let rows = vec![vec![cell("one", Some(0), Some(0), 1, 1)]];
    let options = BuildTableOptions::default();
    let mut cell_budget = TableCellBudget::default();
    let mut measurement = Meter::unlimited();
    build_table_with_budget(&rows, options, &mut cell_budget, &mut measurement).unwrap();
    let exact = measurement.used;
    assert!(measurement.calls > 0);

    let mut exact_budget = TableCellBudget::default();
    let mut exact_meter = Meter {
        limit: exact,
        ..Meter::default()
    };
    build_table_with_budget(&rows, options, &mut exact_budget, &mut exact_meter).unwrap();
    assert_eq!(exact_meter.used, exact);

    let mut short_budget = TableCellBudget::default();
    let mut short_meter = Meter {
        limit: exact - 1,
        ..Meter::default()
    };
    assert_eq!(
        build_table_with_budget(&rows, options, &mut short_budget, &mut short_meter)
            .unwrap_err()
            .code,
        ErrorCode::OutputTooLarge
    );
    assert!(short_meter.used < exact);

    let mut rejected_cells = TableCellBudget::with_limit(0);
    let mut reject_everything = Meter::unlimited();
    assert_eq!(
        build_table_with_budget(&rows, options, &mut rejected_cells, &mut reject_everything)
            .unwrap_err()
            .code,
        ErrorCode::OutputTooLarge
    );
}

#[test]
fn rich_builder_moves_nested_blocks_and_does_not_guess_collision_attachments() {
    let row = vec![RichCellContext {
        cell: cell("outer", Some(0), Some(0), 1, 1),
        blocks: Some(vec![
            IrBlock::paragraph("outer paragraph"),
            nested_table_block("inner"),
        ]),
    }];
    let table = build_table_with_blocks(
        vec![row],
        BuildTableOptions::default(),
        &mut TableCellBudget::default(),
        &mut Meter::unlimited(),
    )
    .unwrap();
    let blocks = table.cells[0][0].blocks.as_ref().unwrap();
    assert_eq!(blocks.len(), 2);
    assert_eq!(blocks[0].text.as_deref(), Some("outer paragraph"));
    assert_eq!(blocks[1].table.as_ref().unwrap().cells[0][0].text, "inner");

    let collision = build_table_with_blocks(
        vec![vec![
            RichCellContext {
                cell: cell("first", Some(0), Some(0), 1, 1),
                blocks: Some(vec![nested_table_block("unmatched-first")]),
            },
            RichCellContext {
                cell: cell("second", Some(0), Some(0), 1, 1),
                blocks: Some(vec![nested_table_block("unmatched-second")]),
            },
        ]],
        BuildTableOptions::default(),
        &mut TableCellBudget::default(),
        &mut Meter::unlimited(),
    )
    .unwrap();
    assert_eq!(collision.cells[0][0].text, "first\nsecond");
    assert!(collision.cells[0][0].blocks.is_none());

    let blank_collision = build_table_with_blocks(
        vec![vec![
            RichCellContext {
                cell: cell("first", Some(0), Some(0), 1, 1),
                blocks: Some(vec![IrBlock::paragraph("matching source content")]),
            },
            RichCellContext {
                cell: cell(" \t", Some(0), Some(0), 1, 1),
                blocks: None,
            },
        ]],
        BuildTableOptions::default(),
        &mut TableCellBudget::default(),
        &mut Meter::unlimited(),
    )
    .unwrap();
    assert_eq!(blank_collision.cells[0][0].text, "first");
    assert_eq!(
        blank_collision.cells[0][0].blocks.as_ref().unwrap()[0]
            .text
            .as_deref(),
        Some("matching source content")
    );

    let blank_owner_replaced = build_table_with_blocks(
        vec![vec![
            RichCellContext {
                cell: cell(" ", Some(0), Some(0), 1, 1),
                blocks: Some(vec![IrBlock::paragraph("no longer matching")]),
            },
            RichCellContext {
                cell: cell("replacement", Some(0), Some(0), 1, 1),
                blocks: None,
            },
        ]],
        BuildTableOptions::default(),
        &mut TableCellBudget::default(),
        &mut Meter::unlimited(),
    )
    .unwrap();
    assert_eq!(blank_owner_replaced.cells[0][0].text, "replacement");
    assert!(blank_owner_replaced.cells[0][0].blocks.is_none());
}

#[test]
fn rich_block_depth_is_bounded_at_the_existing_core_projection_limit() {
    fn nested_table(depth: usize) -> IrBlock {
        if depth == 0 {
            return nested_table_block("leaf");
        }
        IrBlock {
            kind: IrBlockType::Table,
            table: Some(IrTable {
                rows: 1,
                cols: 1,
                cells: vec![vec![kordoc_ir::IrCell {
                    text: "level".to_owned(),
                    col_span: 1,
                    row_span: 1,
                    blocks: Some(vec![nested_table(depth - 1)]),
                    ..kordoc_ir::IrCell::default()
                }]],
                ..IrTable::default()
            }),
            ..IrBlock::default()
        }
    }

    let build = |depth| {
        build_table_with_blocks(
            vec![vec![RichCellContext {
                cell: cell("outer", Some(0), Some(0), 1, 1),
                blocks: Some(vec![nested_table(depth)]),
            }]],
            BuildTableOptions::default(),
            &mut TableCellBudget::default(),
            &mut Meter::unlimited(),
        )
    };
    assert!(build(62).is_ok());
    assert_eq!(build(63).err().unwrap().code, ErrorCode::OutputTooLarge);

    fn nested_children(depth: usize) -> IrBlock {
        if depth == 0 {
            return IrBlock::paragraph("leaf");
        }
        IrBlock {
            kind: IrBlockType::List,
            children: Some(vec![nested_children(depth - 1)]),
            ..IrBlock::default()
        }
    }
    let build_children = |depth| {
        build_table_with_blocks(
            vec![vec![RichCellContext {
                cell: cell("outer", Some(0), Some(0), 1, 1),
                blocks: Some(vec![nested_children(depth)]),
            }]],
            BuildTableOptions::default(),
            &mut TableCellBudget::default(),
            &mut Meter::unlimited(),
        )
    };
    assert!(build_children(62).is_ok());
    assert_eq!(
        build_children(63).err().unwrap().code,
        ErrorCode::OutputTooLarge
    );
}

#[test]
fn table_cell_budget_is_inclusive_for_actual_output_geometry() {
    let rows = vec![
        vec![
            cell("a", Some(0), Some(0), 1, 1),
            cell("b", Some(1), Some(0), 1, 1),
        ],
        vec![
            cell("c", Some(0), Some(1), 1, 1),
            cell("d", Some(1), Some(1), 1, 1),
        ],
    ];
    let mut exact = TableCellBudget::with_limit(4);
    let mut meter = Meter::unlimited();
    let table =
        build_table_with_budget(&rows, BuildTableOptions::default(), &mut exact, &mut meter)
            .unwrap();
    assert_eq!((table.rows, table.cols), (2, 2));

    let mut one_short = TableCellBudget::with_limit(3);
    assert_eq!(
        build_table_with_budget(
            &rows,
            BuildTableOptions::default(),
            &mut one_short,
            &mut Meter::unlimited(),
        )
        .unwrap_err()
        .code,
        ErrorCode::OutputTooLarge
    );
}

#[test]
fn default_logical_cell_budget_allows_exactly_two_million_cells() {
    let mut budget = TableCellBudget::default();
    assert_eq!(budget.charge_grid(10_000, 200).unwrap(), 2_000_000);
    assert_eq!(
        budget.charge_grid(1, 1).unwrap_err().code,
        ErrorCode::OutputTooLarge
    );
}

#[test]
fn compatibility_builder_keeps_original_empty_grid_and_trailing_column_results() {
    let all_blank = vec![
        vec![cell("", None, None, 1, 1), cell("", None, None, 1, 1)],
        vec![cell("", None, None, 1, 1), cell("", None, None, 1, 1)],
    ];
    let table = build_table(&all_blank, BuildTableOptions::default()).unwrap();
    assert_eq!((table.rows, table.cols), (2, 2));

    let anchored = vec![vec![
        cell("value", Some(0), Some(0), 1, 1),
        cell("", Some(1), Some(0), 1, 1),
    ]];
    assert_eq!(
        build_table(&anchored, BuildTableOptions::default())
            .unwrap()
            .cols,
        1
    );
    assert_eq!(
        build_table(
            &anchored,
            BuildTableOptions {
                keep_anchored_empty_cols: true,
                ..BuildTableOptions::default()
            },
        )
        .unwrap()
        .cols,
        2
    );
}
