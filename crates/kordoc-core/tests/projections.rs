//! Public, source-neutral projection checks. These synthetic IR cases do not
//! count as successful source-document oracle parity cases.

use kordoc_core::{
    blocks_to_chunks, blocks_to_markdown, blocks_to_pages,
    table::{classifier::classify_table_tree, visual::choose_table_representation},
};
use kordoc_ir::{
    ChunkOptions, ErrorCode, IrBlock, IrBlockType, IrCell, IrTable, PageMarkdown,
    TableRepresentation,
};

fn paragraph(text: &str, page_number: Option<u32>) -> IrBlock {
    IrBlock {
        page_number,
        ..IrBlock::paragraph(text)
    }
}

fn cell(text: &str) -> IrCell {
    IrCell {
        text: text.into(),
        col_span: 1,
        row_span: 1,
        ..IrCell::default()
    }
}

fn table_block(table: IrTable, page_number: Option<u32>) -> IrBlock {
    IrBlock {
        kind: IrBlockType::Table,
        table: Some(table),
        page_number,
        ..IrBlock::default()
    }
}

#[test]
fn projected_success_contains_exact_markdown_pages() {
    let blocks = [
        paragraph("lead", None),
        IrBlock {
            kind: IrBlockType::Heading,
            level: Some(2),
            text: Some("Scope".into()),
            page_number: Some(1),
            ..IrBlock::default()
        },
        paragraph("body", None),
        paragraph("last", Some(3)),
    ];

    assert_eq!(
        blocks_to_markdown(&blocks).unwrap(),
        "lead\n\n## Scope\n\nbody\n\nlast"
    );
    assert_eq!(
        blocks_to_pages(&blocks, None, blocks_to_markdown).unwrap(),
        Some(vec![
            PageMarkdown {
                page_number: 1,
                markdown: "lead\n\n## Scope\n\nbody".into(),
            },
            PageMarkdown {
                page_number: 2,
                markdown: String::new(),
            },
            PageMarkdown {
                page_number: 3,
                markdown: "last".into(),
            },
        ])
    );
    assert_eq!(
        blocks_to_pages(&[paragraph("unpaged", None)], None, blocks_to_markdown).unwrap(),
        None
    );
}

#[test]
fn opt_in_classification_does_not_change_default_markdown() {
    let table = IrTable {
        rows: 2,
        cols: 2,
        has_header: true,
        cells: vec![vec![cell("A"), cell("B")], vec![cell("C"), cell("D")]],
        ..IrTable::default()
    };
    let original = vec![table_block(table, Some(1))];
    let default_markdown = blocks_to_markdown(&original).unwrap();
    assert_eq!(default_markdown, "| A | B |\n| --- | --- |\n| C | D |");
    assert!(original[0].table.as_ref().unwrap().classification.is_none());
    assert_eq!(
        choose_table_representation(original[0].table.as_ref().unwrap(), false),
        TableRepresentation::Gfm
    );

    let mut classified = original.clone();
    classify_table_tree(&mut classified).unwrap();
    assert!(
        classified[0]
            .table
            .as_ref()
            .unwrap()
            .classification
            .is_some()
    );
    assert_eq!(
        classified[0].table.as_ref().unwrap().cells,
        original[0].table.as_ref().unwrap().cells
    );
    assert_eq!(blocks_to_markdown(&classified).unwrap(), default_markdown);
    assert!(original[0].table.as_ref().unwrap().classification.is_none());
}

#[test]
fn recursive_projection_is_deterministic() {
    let nested = table_block(
        IrTable {
            rows: 1,
            cols: 1,
            cells: vec![vec![cell("inner")]],
            ..IrTable::default()
        },
        None,
    );
    let outer = table_block(
        IrTable {
            rows: 1,
            cols: 1,
            cells: vec![vec![IrCell {
                text: "outer".into(),
                blocks: Some(vec![nested]),
                ..cell("")
            }]],
            ..IrTable::default()
        },
        Some(2),
    );
    let blocks = [outer];
    let first_markdown = blocks_to_markdown(&blocks).unwrap();
    assert_eq!(first_markdown.matches("<table>").count(), 2);
    assert_eq!(blocks_to_markdown(&blocks).unwrap(), first_markdown);

    let first_chunks = blocks_to_chunks(&blocks, ChunkOptions::default()).unwrap();
    assert_eq!(first_chunks.len(), 1);
    assert_eq!(first_chunks[0].text, first_markdown);
    assert_eq!(first_chunks[0].id, "c0001");
    assert_eq!(first_chunks[0].block_range, [0, 0]);
    assert_eq!(
        blocks_to_chunks(&blocks, ChunkOptions::default()).unwrap(),
        first_chunks
    );
}

#[test]
fn projection_limits_are_typed_failures() {
    let distant_pages = [paragraph("first", Some(1)), paragraph("far", Some(100_001))];
    assert_eq!(
        blocks_to_pages(&distant_pages, None, blocks_to_markdown)
            .unwrap_err()
            .code,
        ErrorCode::OutputTooLarge
    );

    let too_wide = table_block(
        IrTable {
            rows: 1,
            cols: 201,
            cells: vec![vec![cell("x"); 201]],
            ..IrTable::default()
        },
        None,
    );
    assert_eq!(
        blocks_to_markdown(&[too_wide]).unwrap_err().code,
        ErrorCode::OutputTooLarge
    );

    let mut nested = IrBlock::paragraph("deep");
    for _ in 0..65 {
        nested = table_block(
            IrTable {
                rows: 1,
                cols: 1,
                cells: vec![vec![IrCell {
                    blocks: Some(vec![nested]),
                    ..cell("")
                }]],
                ..IrTable::default()
            },
            None,
        );
    }
    assert_eq!(
        blocks_to_chunks(&[nested], ChunkOptions::default())
            .unwrap_err()
            .code,
        ErrorCode::OutputTooLarge
    );
}
