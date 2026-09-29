//! Small, bounded property inputs for the source-neutral projection APIs.
//!
//! These cases exercise arbitrary Unicode and recursive IR shapes. They do not
//! use the migration oracle and do not count as source-document parity cases.

use kordoc_core::{
    blocks_to_chunks, blocks_to_markdown, blocks_to_pages, markdown_units::split_markdown_units,
    table::classifier::classify_table_tree,
};
use kordoc_ir::{ChunkOptions, IrBlock, IrBlockType, IrCell, IrSpan, IrTable};
use proptest::prelude::*;
use proptest::test_runner::TestCaseResult;

const MAX_TEST_OUTPUT_BYTES: usize = 1024 * 1024;

fn unicode_text() -> impl Strategy<Value = String> {
    prop::collection::vec(any::<char>(), 0..=48).prop_map(|chars| chars.into_iter().collect())
}

fn span_strategy() -> impl Strategy<Value = IrSpan> {
    (
        unicode_text(),
        any::<Option<bool>>(),
        any::<Option<bool>>(),
        any::<Option<bool>>(),
        any::<Option<bool>>(),
        any::<Option<bool>>(),
    )
        .prop_map(|(text, bold, italic, strike, underline, code)| IrSpan {
            text,
            bold,
            italic,
            strike,
            underline,
            code,
            placeholder: None,
        })
}

fn leaf_strategy() -> impl Strategy<Value = IrBlock> {
    (
        0u8..2,
        unicode_text(),
        prop::collection::vec(span_strategy(), 0..=4),
        prop::option::of(1u32..=8),
        0u32..=8,
    )
        .prop_map(|(kind, text, spans, page_number, level)| IrBlock {
            kind: if kind == 0 {
                IrBlockType::Paragraph
            } else {
                IrBlockType::Heading
            },
            text: Some(text),
            spans: Some(spans),
            page_number,
            level: Some(level),
            ..IrBlock::default()
        })
}

fn table_strategy(inner: BoxedStrategy<IrBlock>) -> BoxedStrategy<IrBlock> {
    (1u32..=3, 1u32..=3)
        .prop_flat_map(move |(rows, cols)| {
            let inner = inner.clone();
            let row = prop::collection::vec(
                (unicode_text(), prop::collection::vec(inner, 0..=2)),
                cols as usize,
            )
            .prop_map(move |generated| {
                generated
                    .into_iter()
                    .map(|(text, blocks)| IrCell {
                        text,
                        col_span: 1,
                        row_span: 1,
                        blocks: Some(blocks),
                        ..IrCell::default()
                    })
                    .collect::<Vec<_>>()
            });
            prop::collection::vec(row, rows as usize).prop_map(move |cells| IrBlock {
                kind: IrBlockType::Table,
                table: Some(IrTable {
                    rows,
                    cols,
                    cells,
                    has_header: rows > 1,
                    ..IrTable::default()
                }),
                page_number: Some(1),
                ..IrBlock::default()
            })
        })
        .boxed()
}

fn block_strategy() -> BoxedStrategy<IrBlock> {
    leaf_strategy()
        .prop_recursive(3, 24, 3, |inner| {
            prop_oneof![leaf_strategy(), table_strategy(inner)]
        })
        .boxed()
}

fn blocks_strategy() -> impl Strategy<Value = Vec<IrBlock>> {
    prop::collection::vec(block_strategy(), 0..=10)
}

fn successful_markdown_is_bounded(
    result: &Result<String, kordoc_ir::KordocError>,
) -> TestCaseResult {
    if let Ok(markdown) = result {
        prop_assert!(markdown.len() <= MAX_TEST_OUTPUT_BYTES);
    }
    Ok(())
}

proptest! {
    #![proptest_config(ProptestConfig {
        cases: 96,
        max_shrink_iters: 2_000,
        .. ProptestConfig::default()
    })]

    #[test]
    fn unicode_recursive_projections_are_deterministic_and_bounded(blocks in blocks_strategy()) {
        let markdown = blocks_to_markdown(&blocks);
        let markdown_again = blocks_to_markdown(&blocks);
        prop_assert_eq!(&markdown, &markdown_again);
        successful_markdown_is_bounded(&markdown)?;

        let pages = blocks_to_pages(&blocks, None, blocks_to_markdown);
        let pages_again = blocks_to_pages(&blocks, None, blocks_to_markdown);
        prop_assert_eq!(&pages, &pages_again);
        if let Ok(Some(pages)) = &pages {
            prop_assert!(pages.len() <= 8);
            let bytes = pages.iter().map(|page| page.markdown.len()).sum::<usize>();
            prop_assert!(bytes <= MAX_TEST_OUTPUT_BYTES);
        }

        let chunks = blocks_to_chunks(&blocks, ChunkOptions::default());
        let chunks_again = blocks_to_chunks(&blocks, ChunkOptions::default());
        prop_assert_eq!(&chunks, &chunks_again);
        if let Ok(chunks) = &chunks {
            prop_assert!(chunks.len() <= 10);
            let bytes = chunks.iter().map(|chunk| chunk.text.len()).sum::<usize>();
            prop_assert!(bytes <= MAX_TEST_OUTPUT_BYTES);
        }

        let mut classified = blocks.clone();
        let mut classified_again = blocks.clone();
        let classification = classify_table_tree(&mut classified);
        let classification_again = classify_table_tree(&mut classified_again);
        prop_assert_eq!(&classification, &classification_again);
        prop_assert_eq!(&classified, &classified_again);
        if classification.is_ok() {
            let encoded = serde_json::to_vec(&classified).expect("IR serialization is infallible");
            prop_assert!(encoded.len() <= MAX_TEST_OUTPUT_BYTES);
        }
    }

    #[test]
    fn markdown_units_are_deterministic_for_arbitrary_unicode(markdown in unicode_text()) {
        let units = split_markdown_units(&markdown);
        let units_again = split_markdown_units(&markdown);
        prop_assert_eq!(&units, &units_again);
        if let Ok(units) = &units {
            let bytes = units.iter().map(|unit| {
                unit.raw.len() + unit.lines.iter().map(String::len).sum::<usize>()
            }).sum::<usize>();
            prop_assert!(bytes <= MAX_TEST_OUTPUT_BYTES);
        }
    }
}
