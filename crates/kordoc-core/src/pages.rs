use kordoc_ir::{
    ErrorCode, IrBlock, IrCell, IrSpan, IrTable, KordocError, PageEvidence, PageMarkdown,
};

const MAX_PROJECTED_PAGES: usize = 100_000;
const MAX_MARKDOWN_BYTES: usize = 256 * 1024 * 1024;
const MAX_BLOCK_DEPTH: usize = 64;

/// Project numbered top-level blocks into page Markdown, using the caller's renderer.
pub fn blocks_to_pages<F>(
    blocks: &[IrBlock],
    page_evidence: Option<&[PageEvidence]>,
    render: F,
) -> Result<Option<Vec<PageMarkdown>>, KordocError>
where
    F: Fn(&[IrBlock]) -> Result<String, KordocError>,
{
    blocks_to_pages_with_limits(
        blocks,
        page_evidence,
        render,
        MAX_PROJECTED_PAGES,
        MAX_MARKDOWN_BYTES,
    )
}

fn blocks_to_pages_with_limits<F>(
    blocks: &[IrBlock],
    page_evidence: Option<&[PageEvidence]>,
    render: F,
    max_pages: usize,
    max_output_bytes: usize,
) -> Result<Option<Vec<PageMarkdown>>, KordocError>
where
    F: Fn(&[IrBlock]) -> Result<String, KordocError>,
{
    validate_page_evidence(page_evidence, max_pages)?;
    let Some(first_page) = blocks.iter().find_map(|block| block.page_number) else {
        return Ok(None);
    };

    let mut current_page = first_page;
    let mut minimum_page = u32::MAX;
    let mut maximum_page = 0u32;
    for block in blocks {
        if let Some(page) = block.page_number {
            if page == 0 {
                return Err(invalid_page("Block page numbers must be positive"));
            }
            current_page = page;
        }
        minimum_page = minimum_page.min(current_page);
        maximum_page = maximum_page.max(current_page);
    }
    if let Some(evidence) = page_evidence {
        for item in evidence {
            minimum_page = minimum_page.min(item.page_number);
            maximum_page = maximum_page.max(item.page_number);
        }
    }

    let span = usize::try_from(u64::from(maximum_page) - u64::from(minimum_page) + 1)
        .map_err(|_| output_too_large("Page range does not fit this platform"))?;
    if span > max_pages {
        return Err(output_too_large(
            "Projected page range exceeds the configured page budget",
        ));
    }

    let mut assigned_pages = Vec::new();
    assigned_pages
        .try_reserve_exact(blocks.len())
        .map_err(|_| output_too_large("Page projection block index allocation failed"))?;
    current_page = first_page;
    for (index, block) in blocks.iter().enumerate() {
        if let Some(page) = block.page_number {
            current_page = page;
        }
        assigned_pages.push((current_page, index));
    }

    // Sorting by (page, original index) keeps revisited-page blocks in source order.
    assigned_pages.sort_unstable_by_key(|(page, index)| (*page, *index));
    let mut pages = Vec::new();
    pages
        .try_reserve_exact(span)
        .map_err(|_| output_too_large("Page projection allocation failed"))?;
    let mut assignment_index = 0usize;
    let mut output_bytes = 0usize;

    for offset in 0..span {
        let page_number = u32::try_from(u64::from(minimum_page) + offset as u64)
            .map_err(|_| output_too_large("Projected page number exceeds u32"))?;
        let start = assignment_index;
        while assignment_index < assigned_pages.len()
            && assigned_pages[assignment_index].0 == page_number
        {
            assignment_index += 1;
        }

        let markdown = if start == assignment_index {
            String::new()
        } else {
            let block_count = assignment_index - start;
            let mut page_blocks = Vec::new();
            page_blocks
                .try_reserve_exact(block_count)
                .map_err(|_| output_too_large("Page block allocation failed"))?;
            let mut clone_bytes = 0usize;
            for (_, block_index) in &assigned_pages[start..assignment_index] {
                estimate_block_clone_bytes(&blocks[*block_index], 1, &mut clone_bytes)?;
                if clone_bytes > MAX_MARKDOWN_BYTES {
                    return Err(output_too_large(
                        "Page blocks exceed the clone safety budget",
                    ));
                }
                page_blocks.push(blocks[*block_index].clone());
            }
            render(&page_blocks)?
        };

        output_bytes = output_bytes
            .checked_add(markdown.len())
            .filter(|bytes| *bytes <= max_output_bytes)
            .ok_or_else(|| output_too_large("Page Markdown exceeds the output byte budget"))?;
        pages.push(PageMarkdown {
            page_number,
            markdown,
        });
    }
    Ok(Some(pages))
}

fn validate_page_evidence(
    evidence: Option<&[PageEvidence]>,
    max_pages: usize,
) -> Result<(), KordocError> {
    let Some(evidence) = evidence else {
        return Ok(());
    };
    if evidence.len() > max_pages {
        return Err(output_too_large("Page evidence exceeds 100,000 entries"));
    }
    let mut previous = None;
    for item in evidence {
        if item.page_number == 0 || previous.is_some_and(|page| item.page_number <= page) {
            return Err(invalid_page(
                "Page evidence must contain positive, strictly ascending page numbers",
            ));
        }
        previous = Some(item.page_number);
    }
    Ok(())
}

fn estimate_block_clone_bytes(
    block: &IrBlock,
    depth: usize,
    total: &mut usize,
) -> Result<(), KordocError> {
    if depth > MAX_BLOCK_DEPTH {
        return Err(output_too_large("Block recursion exceeds 64 levels"));
    }
    add_clone_bytes(total, std::mem::size_of::<IrBlock>())?;
    if let Some(text) = &block.text {
        add_clone_bytes(total, text.len())?;
    }
    if let Some(href) = &block.href {
        add_clone_bytes(total, href.len())?;
    }
    if let Some(note) = &block.footnote_text {
        add_clone_bytes(total, note.len())?;
    }
    if let Some(spans) = &block.spans {
        add_clone_bytes(
            total,
            spans
                .len()
                .checked_mul(std::mem::size_of::<IrSpan>())
                .ok_or_else(|| output_too_large("Block span clone size overflowed"))?,
        )?;
        for span in spans {
            add_clone_bytes(total, span.text.len())?;
        }
    }
    if let Some(font_name) = block
        .style
        .as_ref()
        .and_then(|style| style.font_name.as_ref())
    {
        add_clone_bytes(total, font_name.len())?;
    }
    if let Some(image) = &block.image_data {
        add_clone_bytes(total, image.data.len())?;
        add_clone_bytes(total, image.mime_type.len())?;
        if let Some(filename) = &image.filename {
            add_clone_bytes(total, filename.len())?;
        }
    }
    if let Some(children) = &block.children {
        for child in children {
            estimate_block_clone_bytes(child, depth + 1, total)?;
        }
    }
    if let Some(table) = &block.table {
        estimate_table_clone_bytes(table, depth, total)?;
    }
    Ok(())
}

fn estimate_table_clone_bytes(
    table: &IrTable,
    depth: usize,
    total: &mut usize,
) -> Result<(), KordocError> {
    add_clone_bytes(total, std::mem::size_of::<IrTable>())?;
    if let Some(source_id) = &table.source_id {
        add_clone_bytes(total, source_id.len())?;
    }
    if let Some(caption) = &table.caption {
        add_clone_bytes(total, caption.len())?;
    }
    if let Some(regions) = &table.regions {
        add_clone_bytes(
            total,
            regions
                .len()
                .checked_mul(std::mem::size_of::<kordoc_ir::BoundingBox>())
                .ok_or_else(|| output_too_large("Table region clone size overflowed"))?,
        )?;
    }
    if let Some(classification) = &table.classification {
        add_clone_bytes(
            total,
            classification
                .reasons
                .len()
                .checked_mul(std::mem::size_of::<kordoc_ir::TableClassificationReason>())
                .ok_or_else(|| output_too_large("Table classification clone size overflowed"))?,
        )?;
    }
    add_clone_bytes(
        total,
        table
            .cells
            .len()
            .checked_mul(std::mem::size_of::<Vec<IrCell>>())
            .ok_or_else(|| output_too_large("Table row clone size overflowed"))?,
    )?;
    for row in &table.cells {
        add_clone_bytes(
            total,
            row.len()
                .checked_mul(std::mem::size_of::<IrCell>())
                .ok_or_else(|| output_too_large("Table cell clone size overflowed"))?,
        )?;
        for cell in row {
            estimate_cell_clone_bytes(cell, depth + 1, total)?;
        }
    }
    if let Some(caption_blocks) = &table.caption_blocks {
        for block in caption_blocks {
            estimate_block_clone_bytes(block, depth + 1, total)?;
        }
    }
    Ok(())
}

fn estimate_cell_clone_bytes(
    cell: &IrCell,
    depth: usize,
    total: &mut usize,
) -> Result<(), KordocError> {
    add_clone_bytes(total, cell.text.len())?;
    if let Some(blocks) = &cell.blocks {
        for block in blocks {
            estimate_block_clone_bytes(block, depth + 1, total)?;
        }
    }
    Ok(())
}

fn add_clone_bytes(total: &mut usize, bytes: usize) -> Result<(), KordocError> {
    *total = total
        .checked_add(bytes)
        .ok_or_else(|| output_too_large("Page block clone size overflowed"))?;
    Ok(())
}

fn invalid_page(message: &str) -> KordocError {
    KordocError::new(ErrorCode::ParseError, message)
}

fn output_too_large(message: &str) -> KordocError {
    KordocError::new(ErrorCode::OutputTooLarge, message)
}

#[cfg(test)]
mod tests {
    use super::{blocks_to_pages, blocks_to_pages_with_limits};
    use kordoc_ir::{ErrorCode, IrBlock, PageEvidence, PageMarkdown};

    fn render_text(blocks: &[IrBlock]) -> Result<String, kordoc_ir::KordocError> {
        Ok(blocks
            .iter()
            .filter_map(|block| block.text.as_deref())
            .collect::<Vec<_>>()
            .join("|"))
    }

    fn paragraph(text: &str, page_number: Option<u32>) -> IrBlock {
        IrBlock {
            page_number,
            ..IrBlock::paragraph(text)
        }
    }

    #[test]
    fn pages_omitted_without_page_numbers() {
        let blocks = vec![paragraph("a", None), paragraph("b", None)];
        let result = blocks_to_pages(&blocks, None, render_text).unwrap();
        assert_eq!(result, None);
        let evidence = [PageEvidence { page_number: 1 }];
        assert_eq!(
            blocks_to_pages(&blocks, Some(&evidence), render_text).unwrap(),
            None
        );
    }

    #[test]
    fn pages_validate_evidence_when_blocks_are_pageless() {
        let blocks = vec![paragraph("no numbered blocks", None)];
        for evidence in [
            vec![PageEvidence { page_number: 0 }],
            vec![
                PageEvidence { page_number: 2 },
                PageEvidence { page_number: 1 },
            ],
        ] {
            assert_eq!(
                blocks_to_pages(&blocks, Some(&evidence), render_text)
                    .unwrap_err()
                    .code,
                ErrorCode::ParseError
            );
        }
        let over_budget = [
            PageEvidence { page_number: 1 },
            PageEvidence { page_number: 2 },
        ];
        assert_eq!(
            blocks_to_pages_with_limits(&blocks, Some(&over_budget), render_text, 1, 100)
                .unwrap_err()
                .code,
            ErrorCode::OutputTooLarge
        );
    }

    #[test]
    fn pages_keep_leading_and_middle_unumbered_blocks() {
        let blocks = vec![
            paragraph("leading", None),
            paragraph("two-a", Some(2)),
            paragraph("middle", None),
            paragraph("three", Some(3)),
            paragraph("trailing", None),
        ];
        let pages = blocks_to_pages(&blocks, None, render_text)
            .unwrap()
            .unwrap();
        assert_eq!(
            pages,
            [
                PageMarkdown {
                    page_number: 2,
                    markdown: "leading|two-a|middle".into(),
                },
                PageMarkdown {
                    page_number: 3,
                    markdown: "three|trailing".into(),
                },
            ]
        );
    }

    #[test]
    fn pages_emit_empty_gap_entries() {
        let blocks = vec![paragraph("first", Some(1)), paragraph("third", Some(3))];
        let pages = blocks_to_pages(&blocks, None, render_text)
            .unwrap()
            .unwrap();
        assert_eq!(
            pages,
            [
                PageMarkdown {
                    page_number: 1,
                    markdown: "first".into(),
                },
                PageMarkdown {
                    page_number: 2,
                    markdown: String::new(),
                },
                PageMarkdown {
                    page_number: 3,
                    markdown: "third".into(),
                },
            ]
        );
    }

    #[test]
    fn pages_preserve_revisited_page_order() {
        let blocks = vec![
            paragraph("one-a", Some(1)),
            paragraph("two", Some(2)),
            paragraph("one-b", Some(1)),
        ];
        let pages = blocks_to_pages(&blocks, None, render_text)
            .unwrap()
            .unwrap();
        assert_eq!(pages[0].markdown, "one-a|one-b");
        assert_eq!(pages[1].markdown, "two");
    }

    #[test]
    fn pages_reject_absurd_gap_before_allocation() {
        let blocks = vec![paragraph("first", Some(1)), paragraph("fourth", Some(4))];
        let rendered = std::cell::Cell::new(false);
        let too_wide = blocks_to_pages_with_limits(
            &blocks,
            None,
            |_| {
                rendered.set(true);
                Ok(String::new())
            },
            3,
            100,
        );
        assert_eq!(too_wide.unwrap_err().code, ErrorCode::OutputTooLarge);
        assert!(!rendered.get());

        let inclusive_edge = vec![paragraph("first", Some(1)), paragraph("third", Some(3))];
        assert_eq!(
            blocks_to_pages_with_limits(&inclusive_edge, None, render_text, 3, 100)
                .unwrap()
                .unwrap()
                .len(),
            3
        );
    }

    #[test]
    fn pages_keep_empty_source_evidence_without_reassigning_blocks() {
        let blocks = vec![paragraph("block-on-two", Some(2))];
        let evidence = [
            PageEvidence { page_number: 1 },
            PageEvidence { page_number: 2 },
            PageEvidence { page_number: 4 },
        ];
        let pages = blocks_to_pages(&blocks, Some(&evidence), render_text)
            .unwrap()
            .unwrap();
        assert_eq!(
            pages,
            [
                PageMarkdown {
                    page_number: 1,
                    markdown: String::new(),
                },
                PageMarkdown {
                    page_number: 2,
                    markdown: "block-on-two".into(),
                },
                PageMarkdown {
                    page_number: 3,
                    markdown: String::new(),
                },
                PageMarkdown {
                    page_number: 4,
                    markdown: String::new(),
                },
            ]
        );
    }

    #[test]
    fn pages_validate_ascending_bounded_evidence() {
        let blocks = vec![paragraph("page", Some(2))];
        for evidence in [
            vec![PageEvidence { page_number: 0 }],
            vec![
                PageEvidence { page_number: 2 },
                PageEvidence { page_number: 1 },
            ],
            vec![
                PageEvidence { page_number: 2 },
                PageEvidence { page_number: 2 },
            ],
        ] {
            assert_eq!(
                blocks_to_pages(&blocks, Some(&evidence), render_text)
                    .unwrap_err()
                    .code,
                ErrorCode::ParseError
            );
        }
        assert_eq!(
            blocks_to_pages_with_limits(&blocks, Some(&[]), render_text, 0, 100)
                .unwrap_err()
                .code,
            ErrorCode::OutputTooLarge
        );
        let too_many = [
            PageEvidence { page_number: 1 },
            PageEvidence { page_number: 3 },
        ];
        assert_eq!(
            blocks_to_pages_with_limits(&blocks, Some(&too_many), render_text, 1, 100)
                .unwrap_err()
                .code,
            ErrorCode::OutputTooLarge
        );
    }

    #[test]
    fn pages_reject_output_over_budget_without_partial_result() {
        let blocks = vec![paragraph("page", Some(1))];
        let result = blocks_to_pages_with_limits(&blocks, None, |_| Ok("too long".into()), 1, 7);
        assert_eq!(result.unwrap_err().code, ErrorCode::OutputTooLarge);
    }

    #[test]
    fn pages_reject_block_recursion_over_limit_before_rendering() {
        let mut nested = paragraph("deep", None);
        for _ in 0..65 {
            nested = IrBlock {
                children: Some(vec![nested]),
                ..IrBlock::default()
            };
        }
        nested.page_number = Some(1);
        let rendered = std::cell::Cell::new(false);
        let result = blocks_to_pages(&[nested], None, |_| {
            rendered.set(true);
            Ok(String::new())
        });
        assert_eq!(result.unwrap_err().code, ErrorCode::OutputTooLarge);
        assert!(!rendered.get());
    }
}
