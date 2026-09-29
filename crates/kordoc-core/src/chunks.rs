//! Structural chunks projected from source-neutral IR.

use kordoc_ir::{
    ChunkGranularity, ChunkOptions, DocChunk, DocChunkTable, DocChunkType, ErrorCode, IrBlock,
    IrBlockType, IrTable, KordocError,
};

const MAX_CHUNKS: usize = 100_000;
const MAX_CHUNK_OUTPUT_BYTES: usize = 256 * 1024 * 1024;

#[derive(Clone)]
struct HeadingCrumb {
    level: u32,
    text: String,
}

#[derive(Clone)]
struct ListCrumb {
    depth: u32,
    text: String,
}

struct PendingRun {
    breadcrumb: Vec<String>,
    blocks: Vec<IrBlock>,
    start: u32,
    end: u32,
    page: Option<u32>,
}

struct ChunkDraft {
    kind: DocChunkType,
    breadcrumb: Vec<String>,
    text: String,
    range: [u32; 2],
    page: Option<u32>,
    table: Option<DocChunkTable>,
}

fn output_too_large() -> KordocError {
    KordocError::new(
        ErrorCode::OutputTooLarge,
        "Projected chunks exceed the output limit",
    )
}

fn marker_has_space_after(text: &str, marker_end: usize) -> bool {
    text.get(marker_end..)
        .and_then(|tail| tail.chars().next())
        .is_some_and(char::is_whitespace)
}

fn is_hangul_marker(ch: char) -> bool {
    "가나다라마바사아자차카타파하".contains(ch)
}

fn is_list_marker(text: &str) -> bool {
    let text = text.trim();
    let Some(first) = text.chars().next() else {
        return false;
    };
    let first_end = first.len_utf8();
    if "□■◇◆○◎●◦ㅇ•▪▸▶※-".contains(first) && marker_has_space_after(text, first_end)
    {
        return true;
    }
    if ('①'..='⑳').contains(&first)
        || ('㉮'..='㉻').contains(&first)
        || ('㈎'..='㈛').contains(&first)
    {
        return marker_has_space_after(text, first_end);
    }
    if is_hangul_marker(first) {
        let Some(mark) = text[first_end..].chars().next() else {
            return false;
        };
        return matches!(mark, '.' | ')')
            && marker_has_space_after(text, first_end + mark.len_utf8());
    }
    if first == '('
        && let Some(close_index) = text.find(')')
    {
        let body = &text[first_end..close_index];
        let valid = !body.is_empty()
            && body.chars().count() <= 3
            && body
                .chars()
                .all(|ch| ch.is_ascii_digit() || is_hangul_marker(ch));
        return valid && marker_has_space_after(text, close_index + 1);
    }
    let digit_count = text.chars().take_while(char::is_ascii_digit).count();
    if (1..=3).contains(&digit_count) {
        let number_end = text
            .chars()
            .take(digit_count)
            .map(char::len_utf8)
            .sum::<usize>();
        if let Some(mark) = text[number_end..].chars().next() {
            return matches!(mark, '.' | ')')
                && marker_has_space_after(text, number_end + mark.len_utf8());
        }
    }
    false
}

fn list_depth(block: &IrBlock) -> Option<u32> {
    if let Some(depth) = block.list_depth {
        return Some(depth);
    }
    if block.kind == IrBlockType::List
        || (block.kind == IrBlockType::Paragraph
            && block.text.as_deref().is_some_and(is_list_marker))
    {
        Some(0)
    } else {
        None
    }
}

fn current_breadcrumb(headings: &[HeadingCrumb], lists: &[ListCrumb]) -> Vec<String> {
    headings
        .iter()
        .map(|heading| heading.text.clone())
        .chain(lists.iter().map(|list| list.text.clone()))
        .collect()
}

fn current_breadcrumb_bytes(headings: &[HeadingCrumb], lists: &[ListCrumb]) -> Option<usize> {
    headings
        .iter()
        .map(|heading| heading.text.len())
        .chain(lists.iter().map(|list| list.text.len()))
        .try_fold(0usize, usize::checked_add)
}

fn same_breadcrumb(left: &[String], right: &[String]) -> bool {
    left == right
}

fn breadcrumb_bytes(breadcrumb: &[String]) -> Option<usize> {
    breadcrumb
        .iter()
        .try_fold(0usize, |total, value| total.checked_add(value.len()))
}

fn table_matrix_bytes(table: &IrTable, include_cells: bool) -> Option<usize> {
    if !include_cells {
        return Some(0);
    }
    table.cells.iter().try_fold(
        table
            .cells
            .len()
            .checked_mul(std::mem::size_of::<Vec<String>>())?,
        |total, row| {
            let with_cell_structs =
                total.checked_add(row.len().checked_mul(std::mem::size_of::<String>())?)?;
            row.iter().try_fold(with_cell_structs, |row_total, cell| {
                row_total.checked_add(cell.text.len())
            })
        },
    )
}

fn next_id(chunks: &[DocChunk]) -> String {
    format!("c{:04}", chunks.len() + 1)
}

fn push_chunk(
    chunks: &mut Vec<DocChunk>,
    output_bytes: &mut usize,
    chunk_limit: usize,
    output_limit: usize,
    draft: ChunkDraft,
) -> Result<(), KordocError> {
    if chunks.len() >= chunk_limit {
        return Err(output_too_large());
    }
    let id = next_id(chunks);
    let ChunkDraft {
        kind,
        breadcrumb,
        text,
        range,
        page,
        table,
    } = draft;
    let mut size = id
        .len()
        .checked_add(text.len())
        .and_then(|size| size.checked_add(breadcrumb_bytes(&breadcrumb)?))
        .ok_or_else(output_too_large)?;
    if let Some(table) = table.as_ref()
        && let Some(cells) = table.cells.as_ref()
    {
        size = cells
            .iter()
            .try_fold(size, |total, row| {
                row.iter()
                    .try_fold(total, |row_total, cell| row_total.checked_add(cell.len()))
            })
            .ok_or_else(output_too_large)?;
    }
    if output_bytes
        .checked_add(size)
        .is_none_or(|total| total > output_limit)
    {
        return Err(output_too_large());
    }
    *output_bytes += size;
    chunks.push(DocChunk {
        id,
        kind,
        breadcrumb,
        text,
        page,
        block_range: range,
        table,
    });
    Ok(())
}

fn flush_run(
    run: &mut Option<PendingRun>,
    chunks: &mut Vec<DocChunk>,
    output_bytes: &mut usize,
    chunk_limit: usize,
    output_limit: usize,
) -> Result<(), KordocError> {
    let Some(run) = run.take() else { return Ok(()) };
    let text = crate::markdown::blocks_to_markdown(&run.blocks)?;
    push_chunk(
        chunks,
        output_bytes,
        chunk_limit,
        output_limit,
        ChunkDraft {
            kind: DocChunkType::Text,
            breadcrumb: run.breadcrumb,
            text,
            range: [run.start, run.end],
            page: run.page,
            table: None,
        },
    )
}

/// Converts blocks into deterministic structural chunks. This function is opt-in and
/// does not split text by tokens or add overlap.
pub fn blocks_to_chunks(
    blocks: &[IrBlock],
    options: ChunkOptions,
) -> Result<Vec<DocChunk>, KordocError> {
    blocks_to_chunks_with_limits(blocks, options, MAX_CHUNKS, MAX_CHUNK_OUTPUT_BYTES)
}

fn blocks_to_chunks_with_limits(
    blocks: &[IrBlock],
    options: ChunkOptions,
    chunk_limit: usize,
    output_limit: usize,
) -> Result<Vec<DocChunk>, KordocError> {
    if let Some(last) = blocks.len().checked_sub(1) {
        u32::try_from(last).map_err(|_| output_too_large())?;
    }
    let granularity = options.granularity.unwrap_or(ChunkGranularity::Section);
    let include_cells = options.include_table_cells.unwrap_or(false);
    let mut chunks = Vec::new();
    let mut output_bytes = 0usize;
    let mut heading_stack: Vec<HeadingCrumb> = Vec::new();
    let mut list_stack: Vec<ListCrumb> = Vec::new();
    let mut run: Option<PendingRun> = None;

    for (index, block) in blocks.iter().enumerate() {
        let index = u32::try_from(index).map_err(|_| output_too_large())?;
        let normalized_heading;
        let render_block = if block.kind == IrBlockType::Heading && block.level == Some(0) {
            normalized_heading = IrBlock {
                level: Some(2),
                ..block.clone()
            };
            &normalized_heading
        } else {
            block
        };
        let markdown = crate::markdown::blocks_to_markdown(std::slice::from_ref(render_block))?;
        if markdown.is_empty() {
            continue;
        }

        if block.kind == IrBlockType::Heading {
            flush_run(
                &mut run,
                &mut chunks,
                &mut output_bytes,
                chunk_limit,
                output_limit,
            )?;
            let level = block
                .level
                .filter(|level| *level > 0)
                .unwrap_or(2)
                .clamp(1, 6);
            while heading_stack
                .last()
                .is_some_and(|heading| heading.level >= level)
            {
                heading_stack.pop();
            }
            list_stack.clear();
            let breadcrumb_size = current_breadcrumb_bytes(&heading_stack, &list_stack)
                .ok_or_else(output_too_large)?;
            let id_size = next_id(&chunks).len();
            if output_bytes
                .checked_add(breadcrumb_size)
                .and_then(|size| size.checked_add(markdown.len()))
                .and_then(|size| size.checked_add(id_size))
                .is_none_or(|size| size > output_limit)
            {
                return Err(output_too_large());
            }
            let breadcrumb = current_breadcrumb(&heading_stack, &list_stack);
            push_chunk(
                &mut chunks,
                &mut output_bytes,
                chunk_limit,
                output_limit,
                ChunkDraft {
                    kind: DocChunkType::Heading,
                    breadcrumb,
                    text: markdown,
                    range: [index, index],
                    page: block.page_number,
                    table: None,
                },
            )?;
            heading_stack.push(HeadingCrumb {
                level,
                text: block.text.as_deref().unwrap_or("").trim().to_owned(),
            });
            continue;
        }

        if block.kind == IrBlockType::Table
            && let Some(table_ir) = block.table.as_ref()
        {
            flush_run(
                &mut run,
                &mut chunks,
                &mut output_bytes,
                chunk_limit,
                output_limit,
            )?;
            let breadcrumb_size = current_breadcrumb_bytes(&heading_stack, &list_stack)
                .ok_or_else(output_too_large)?;
            let raw_cells_size =
                table_matrix_bytes(table_ir, include_cells).ok_or_else(output_too_large)?;
            let id_size = next_id(&chunks).len();
            let projected_size = output_bytes
                .checked_add(breadcrumb_size)
                .and_then(|size| size.checked_add(markdown.len()))
                .and_then(|size| size.checked_add(raw_cells_size))
                .and_then(|size| size.checked_add(id_size))
                .ok_or_else(output_too_large)?;
            if projected_size > output_limit {
                return Err(output_too_large());
            }
            let breadcrumb = current_breadcrumb(&heading_stack, &list_stack);
            let table = DocChunkTable {
                rows: table_ir.rows,
                cols: table_ir.cols,
                cells: include_cells.then(|| {
                    table_ir
                        .cells
                        .iter()
                        .map(|row| row.iter().map(|cell| cell.text.clone()).collect())
                        .collect()
                }),
            };
            push_chunk(
                &mut chunks,
                &mut output_bytes,
                chunk_limit,
                output_limit,
                ChunkDraft {
                    kind: DocChunkType::Table,
                    breadcrumb,
                    text: markdown,
                    range: [index, index],
                    page: block.page_number,
                    table: Some(table),
                },
            )?;
            continue;
        }

        let depth = list_depth(block);
        if let Some(depth) = depth {
            while list_stack.last().is_some_and(|list| list.depth >= depth) {
                list_stack.pop();
            }
        }
        let breadcrumb_size =
            current_breadcrumb_bytes(&heading_stack, &list_stack).ok_or_else(output_too_large)?;
        if output_bytes
            .checked_add(breadcrumb_size)
            .and_then(|size| size.checked_add(markdown.len()))
            .is_none_or(|size| size > output_limit)
        {
            return Err(output_too_large());
        }
        let breadcrumb = current_breadcrumb(&heading_stack, &list_stack);
        if let Some(depth) = depth {
            list_stack.push(ListCrumb {
                depth,
                text: block.text.as_deref().unwrap_or("").trim().to_owned(),
            });
        }
        if granularity == ChunkGranularity::Block {
            let projected_size = output_bytes
                .checked_add(breadcrumb_bytes(&breadcrumb).ok_or_else(output_too_large)?)
                .and_then(|size| size.checked_add(markdown.len()))
                .and_then(|size| size.checked_add(next_id(&chunks).len()))
                .ok_or_else(output_too_large)?;
            if projected_size > output_limit {
                return Err(output_too_large());
            }
            push_chunk(
                &mut chunks,
                &mut output_bytes,
                chunk_limit,
                output_limit,
                ChunkDraft {
                    kind: DocChunkType::Text,
                    breadcrumb,
                    text: markdown,
                    range: [index, index],
                    page: block.page_number,
                    table: None,
                },
            )?;
            continue;
        }
        let can_append = run
            .as_ref()
            .is_some_and(|pending| same_breadcrumb(&pending.breadcrumb, &breadcrumb));
        if can_append {
            let pending = run
                .as_mut()
                .expect("run exists after same-breadcrumb check");
            let separator_len = if pending.blocks.is_empty() { 0 } else { 2 };
            let pending_size = output_bytes
                .checked_add(
                    pending
                        .blocks
                        .iter()
                        .filter_map(|item| item.text.as_deref())
                        .try_fold(0usize, |sum, text| sum.checked_add(text.len()))
                        .ok_or_else(output_too_large)?,
                )
                .and_then(|size| size.checked_add(separator_len))
                .and_then(|size| size.checked_add(block.text.as_deref().map_or(0, str::len)))
                .and_then(|size| size.checked_add(breadcrumb_bytes(&pending.breadcrumb)?))
                .and_then(|size| size.checked_add(next_id(&chunks).len()))
                .ok_or_else(output_too_large)?;
            if pending_size > output_limit {
                return Err(output_too_large());
            }
            pending.blocks.push(block.clone());
            pending.end = index;
            if pending.page.is_none() {
                pending.page = block.page_number;
            }
        } else {
            flush_run(
                &mut run,
                &mut chunks,
                &mut output_bytes,
                chunk_limit,
                output_limit,
            )?;
            let projected_size = output_bytes
                .checked_add(breadcrumb_bytes(&breadcrumb).ok_or_else(output_too_large)?)
                .and_then(|size| size.checked_add(markdown.len()))
                .and_then(|size| size.checked_add(next_id(&chunks).len()))
                .ok_or_else(output_too_large)?;
            if projected_size > output_limit {
                return Err(output_too_large());
            }
            run = Some(PendingRun {
                breadcrumb,
                blocks: vec![block.clone()],
                start: index,
                end: index,
                page: block.page_number,
            });
        }
    }
    flush_run(
        &mut run,
        &mut chunks,
        &mut output_bytes,
        chunk_limit,
        output_limit,
    )?;
    Ok(chunks)
}

#[cfg(test)]
mod tests {
    use super::{blocks_to_chunks, blocks_to_chunks_with_limits};
    use kordoc_ir::{
        ChunkGranularity, ChunkOptions, DocChunkType, ErrorCode, IrBlock, IrBlockType, IrCell,
        IrTable,
    };

    fn paragraph(text: &str) -> IrBlock {
        IrBlock::paragraph(text)
    }

    fn heading(level: u32, text: &str) -> IrBlock {
        IrBlock {
            kind: IrBlockType::Heading,
            level: Some(level),
            text: Some(text.into()),
            ..IrBlock::default()
        }
    }

    fn list(text: &str, depth: u32) -> IrBlock {
        IrBlock {
            kind: IrBlockType::Paragraph,
            text: Some(text.into()),
            list_depth: Some(depth),
            ..IrBlock::default()
        }
    }

    fn table_block() -> IrBlock {
        let cell = |text: &str| IrCell {
            text: text.into(),
            col_span: 1,
            row_span: 1,
            ..IrCell::default()
        };
        IrBlock {
            kind: IrBlockType::Table,
            table: Some(IrTable {
                rows: 2,
                cols: 2,
                has_header: true,
                cells: vec![
                    vec![cell("항목"), cell("값")],
                    vec![cell("예산"), cell("100")],
                ],
                ..IrTable::default()
            }),
            ..IrBlock::default()
        }
    }

    #[test]
    fn chunks_breadcrumb_heading_and_list_pop() {
        let blocks = [
            heading(1, "추진 계획"),
            heading(2, "1. 개요"),
            paragraph("도입 문단"),
            list("□ 추진 배경", 0),
            list("○ 세부 배경 하나", 1),
            list("- 세부 근거", 2),
            list("○ 세부 배경 둘", 1),
            heading(2, "2. 일정"),
            paragraph("일정 본문"),
        ];
        let chunks = blocks_to_chunks(&blocks, ChunkOptions::default()).unwrap();
        assert_eq!(chunks[0].kind, DocChunkType::Heading);
        assert_eq!(chunks[0].text, "# 추진 계획");
        assert!(chunks[0].breadcrumb.is_empty());
        assert_eq!(chunks[1].text, "## 1. 개요");
        assert_eq!(chunks[1].breadcrumb, ["추진 계획"]);
        let deep = chunks
            .iter()
            .find(|chunk| chunk.text.contains("세부 근거"))
            .unwrap();
        assert_eq!(
            deep.breadcrumb,
            ["추진 계획", "1. 개요", "□ 추진 배경", "○ 세부 배경 하나"]
        );
        let sibling = chunks
            .iter()
            .find(|chunk| chunk.text.contains("세부 배경 둘"))
            .unwrap();
        assert_eq!(sibling.breadcrumb, ["추진 계획", "1. 개요", "□ 추진 배경"]);
        let final_text = chunks
            .iter()
            .find(|chunk| chunk.text == "일정 본문")
            .unwrap();
        assert_eq!(final_text.breadcrumb, ["추진 계획", "2. 일정"]);
    }

    #[test]
    fn chunks_section_merges_only_same_breadcrumb() {
        let blocks = [
            heading(1, "A"),
            paragraph("first"),
            paragraph("second"),
            list("□ item", 0),
            paragraph("under item"),
            heading(2, "B"),
            paragraph("last"),
        ];
        let chunks = blocks_to_chunks(&blocks, ChunkOptions::default()).unwrap();
        let first = chunks
            .iter()
            .find(|chunk| chunk.text.contains("first"))
            .unwrap();
        assert_eq!(first.text, "first\n\nsecond\n\n□ item");
        assert_eq!(first.block_range, [1, 3]);
        assert_eq!(
            chunks
                .iter()
                .filter(|chunk| chunk.kind == DocChunkType::Heading)
                .count(),
            2
        );
        assert_ne!(
            chunks
                .iter()
                .find(|chunk| chunk.text.contains("under item"))
                .unwrap()
                .breadcrumb,
            first.breadcrumb
        );
    }

    #[test]
    fn chunks_section_renders_consecutive_list_blocks_together() {
        let blocks = [
            IrBlock {
                kind: IrBlockType::List,
                text: Some("A".into()),
                ..IrBlock::default()
            },
            IrBlock {
                kind: IrBlockType::List,
                text: Some("B".into()),
                ..IrBlock::default()
            },
        ];
        let chunks = blocks_to_chunks(&blocks, ChunkOptions::default()).unwrap();
        assert_eq!(chunks.len(), 1);
        assert_eq!(chunks[0].text, "- A\n- B");
    }

    #[test]
    fn chunks_section_uses_renderer_for_cross_block_related_paragraphs() {
        let blocks = [paragraph("관련 내용"), paragraph("[별표 3] 적용 기준")];
        let chunks = blocks_to_chunks(&blocks, ChunkOptions::default()).unwrap();
        assert_eq!(chunks.len(), 1);
        assert_eq!(
            chunks[0].text,
            crate::markdown::blocks_to_markdown(&blocks).unwrap()
        );
    }

    #[test]
    fn explicit_zero_heading_level_uses_default_level() {
        let chunks = blocks_to_chunks(&[heading(0, "Zero")], ChunkOptions::default()).unwrap();
        assert_eq!(chunks[0].text, "## Zero");
    }

    #[test]
    fn chunks_block_mode_skips_empty() {
        let blocks = [paragraph("one"), paragraph(""), paragraph("two")];
        let options = ChunkOptions {
            granularity: Some(ChunkGranularity::Block),
            ..ChunkOptions::default()
        };
        let chunks = blocks_to_chunks(&blocks, options).unwrap();
        assert_eq!(
            chunks
                .iter()
                .map(|chunk| chunk.text.as_str())
                .collect::<Vec<_>>(),
            ["one", "two"]
        );
        assert_eq!(
            chunks
                .iter()
                .map(|chunk| chunk.block_range)
                .collect::<Vec<_>>(),
            [[0, 0], [2, 2]]
        );
    }

    #[test]
    fn chunks_table_is_independent_with_optional_cells() {
        let blocks = [paragraph("before"), table_block(), paragraph("after")];
        let chunks = blocks_to_chunks(&blocks, ChunkOptions::default()).unwrap();
        let table = chunks
            .iter()
            .find(|chunk| chunk.kind == DocChunkType::Table)
            .unwrap();
        assert_eq!(table.id, "c0002");
        assert_eq!(table.block_range, [1, 1]);
        assert_eq!(table.table.as_ref().unwrap().rows, 2);
        assert_eq!(table.table.as_ref().unwrap().cols, 2);
        assert_eq!(table.table.as_ref().unwrap().cells, None);
        assert!(table.text.contains("| 항목 | 값 |"));
        let with_cells = blocks_to_chunks(
            &blocks,
            ChunkOptions {
                include_table_cells: Some(true),
                ..ChunkOptions::default()
            },
        )
        .unwrap();
        assert_eq!(
            with_cells
                .iter()
                .find(|chunk| chunk.kind == DocChunkType::Table)
                .unwrap()
                .table
                .as_ref()
                .unwrap()
                .cells,
            Some(vec![
                vec!["항목".into(), "값".into()],
                vec!["예산".into(), "100".into()]
            ])
        );
        assert_eq!(
            chunks
                .iter()
                .filter(|chunk| chunk.kind == DocChunkType::Table)
                .count(),
            1
        );
    }

    #[test]
    fn chunks_nested_table_uses_exact_recursive_markdown() {
        let inner = table_block().table.unwrap();
        let outer = IrBlock {
            kind: IrBlockType::Table,
            table: Some(IrTable {
                rows: 1,
                cols: 1,
                has_header: false,
                cells: vec![vec![IrCell {
                    text: String::new(),
                    col_span: 1,
                    row_span: 1,
                    blocks: Some(vec![IrBlock {
                        kind: IrBlockType::Table,
                        table: Some(inner),
                        ..IrBlock::default()
                    }]),
                    ..IrCell::default()
                }]],
                ..IrTable::default()
            }),
            ..IrBlock::default()
        };
        let chunks = blocks_to_chunks(&[outer], ChunkOptions::default()).unwrap();
        assert_eq!(chunks.len(), 1);
        assert_eq!(chunks[0].kind, DocChunkType::Table);
        assert_eq!(
            chunks[0].text,
            "<table>\n<tr><th><table>\n<tr><th>항목</th><th>값</th></tr>\n<tr><td>예산</td><td>100</td></tr>\n</table></th></tr>\n</table>"
        );
    }

    #[test]
    fn chunks_ranges_are_monotonic_and_inclusive() {
        let blocks = [
            heading(1, "Heading"),
            paragraph("a"),
            paragraph("b"),
            paragraph(""),
            table_block(),
            paragraph("c"),
        ];
        let chunks = blocks_to_chunks(&blocks, ChunkOptions::default()).unwrap();
        let mut previous_end = None;
        for chunk in &chunks {
            let [start, end] = chunk.block_range;
            assert!(end >= start);
            if let Some(previous) = previous_end {
                assert!(start > previous);
            }
            previous_end = Some(end);
        }
        assert_eq!(
            chunks
                .iter()
                .map(|chunk| chunk.block_range)
                .collect::<Vec<_>>(),
            [[0, 0], [1, 2], [4, 4], [5, 5]]
        );
    }

    #[test]
    fn chunks_first_page_of_run_and_omission() {
        let mut first = paragraph("first");
        first.page_number = Some(3);
        let mut second = paragraph("second");
        second.page_number = Some(4);
        let merged = blocks_to_chunks(&[first, second], ChunkOptions::default()).unwrap();
        assert_eq!(merged.len(), 1);
        assert_eq!(merged[0].page, Some(3));
        let unnumbered = blocks_to_chunks(&[paragraph("plain")], ChunkOptions::default()).unwrap();
        assert_eq!(unnumbered[0].page, None);
    }

    #[test]
    fn chunks_unicode_marker_property() {
        for marker in ["□", "○", "-", "1.", "가.", "(가)", "①", "㉮", "㈎"] {
            let text = format!("{marker} 항목");
            let blocks = [paragraph("상위"), paragraph(&text), paragraph("다음")];
            let chunks = blocks_to_chunks(&blocks, ChunkOptions::default()).unwrap();
            let following = chunks.iter().find(|chunk| chunk.text == "다음").unwrap();
            assert!(
                following
                    .breadcrumb
                    .iter()
                    .any(|breadcrumb| breadcrumb == &text),
                "{marker}"
            );
        }
        let blocks = [paragraph("1.2 숫자"), paragraph("plain")];
        let chunks = blocks_to_chunks(
            &blocks,
            ChunkOptions {
                granularity: Some(ChunkGranularity::Block),
                ..ChunkOptions::default()
            },
        )
        .unwrap();
        assert!(chunks[1].breadcrumb.is_empty());
    }

    #[test]
    fn chunks_limits_fail_with_output_too_large_without_truncation() {
        let chunks = [paragraph("one"), paragraph("two")];
        let by_count = blocks_to_chunks_with_limits(
            &chunks,
            ChunkOptions {
                granularity: Some(ChunkGranularity::Block),
                ..ChunkOptions::default()
            },
            1,
            usize::MAX,
        );
        assert_eq!(by_count.unwrap_err().code, ErrorCode::OutputTooLarge);
        let by_bytes =
            blocks_to_chunks_with_limits(&[paragraph("four")], ChunkOptions::default(), 10, 3);
        assert_eq!(by_bytes.unwrap_err().code, ErrorCode::OutputTooLarge);
        assert_eq!(blocks_to_chunks(&[], ChunkOptions::default()).unwrap(), []);
    }

    #[test]
    fn optional_table_cell_matrix_structure_counts_toward_budget() {
        let cell = IrCell {
            text: String::new(),
            col_span: 1,
            row_span: 1,
            ..IrCell::default()
        };
        let table = IrBlock {
            kind: IrBlockType::Table,
            table: Some(IrTable {
                rows: 10,
                cols: 10,
                cells: vec![vec![cell; 10]; 10],
                ..IrTable::default()
            }),
            ..IrBlock::default()
        };
        let err = blocks_to_chunks_with_limits(
            &[table],
            ChunkOptions {
                include_table_cells: Some(true),
                ..ChunkOptions::default()
            },
            10,
            1_000,
        )
        .unwrap_err();
        assert_eq!(err.code, ErrorCode::OutputTooLarge);
    }

    #[test]
    fn chunks_use_deterministic_ids() {
        let blocks = [paragraph("one"), paragraph("two")];
        let options = ChunkOptions {
            granularity: Some(ChunkGranularity::Block),
            ..ChunkOptions::default()
        };
        let first = blocks_to_chunks(&blocks, options.clone()).unwrap();
        let second = blocks_to_chunks(&blocks, options).unwrap();
        assert_eq!(
            first
                .iter()
                .map(|chunk| chunk.id.as_str())
                .collect::<Vec<_>>(),
            ["c0001", "c0002"]
        );
        assert_eq!(first, second);
    }
}
