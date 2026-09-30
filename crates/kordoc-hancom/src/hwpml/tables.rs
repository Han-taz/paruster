//! Bounded HWPML table adapter over the source-neutral `kordoc-tables` builder.

use kordoc_ir::{
    ErrorCode, IrBlock, IrBlockType, IrTable, KordocError, ParseOptions, ParsedDocument,
};
use kordoc_tables::{
    AllocationBudget, BuildTableOptions, CellContext, RichCellContext, TableCellBudget,
    build_table_with_blocks,
};

use crate::hwpx::{XmlContent, XmlNode, budget::LoweringBudget};

const MAX_SOURCE_TABLE_ROWS: usize = 5_000;
const MAX_SOURCE_TABLE_COLS: usize = 500;
const MAX_TABLE_NESTING: usize = 8;

impl AllocationBudget for LoweringBudget {
    fn charge_bytes(&mut self, bytes: usize) -> Result<(), KordocError> {
        LoweringBudget::charge_bytes(self, bytes)
    }
}

pub(super) fn append_table_block(
    node: &XmlNode,
    page_number: u32,
    parsed: &mut ParsedDocument,
    budget: &mut LoweringBudget,
    cell_budget: &mut TableCellBudget,
    options: &ParseOptions,
) -> Result<(), KordocError> {
    let table = lower_table(node, 1, budget, cell_budget, options)?;
    budget.push(
        &mut parsed.blocks,
        IrBlock {
            kind: IrBlockType::Table,
            table: Some(table),
            page_number: Some(page_number),
            ..IrBlock::default()
        },
    )
}

pub(super) fn append_tables_in_paragraph(
    paragraph: &XmlNode,
    page_number: u32,
    parsed: &mut ParsedDocument,
    budget: &mut LoweringBudget,
    cell_budget: &mut TableCellBudget,
    options: &ParseOptions,
) -> Result<(), KordocError> {
    fn visit(
        node: &XmlNode,
        page_number: u32,
        parsed: &mut ParsedDocument,
        budget: &mut LoweringBudget,
        cell_budget: &mut TableCellBudget,
        options: &ParseOptions,
    ) -> Result<(), KordocError> {
        for child in &node.children {
            match child.name.as_str() {
                "TABLE" => {
                    append_table_block(child, page_number, parsed, budget, cell_budget, options)?
                }
                "FOOTNOTE" | "ENDNOTE" | "HEADER" | "FOOTER" => {
                    if contains_named(child, "TABLE") {
                        return Err(unsupported(
                            "HWPML tables inside ignored paragraph wrappers are unsupported",
                        ));
                    }
                }
                "PICTURE" | "SHAPEOBJECT" | "AUTONUM" => {}
                _ => visit(child, page_number, parsed, budget, cell_budget, options)?,
            }
        }
        Ok(())
    }
    visit(paragraph, page_number, parsed, budget, cell_budget, options)
}

fn lower_table(
    node: &XmlNode,
    depth: usize,
    budget: &mut LoweringBudget,
    cell_budget: &mut TableCellBudget,
    options: &ParseOptions,
) -> Result<IrTable, KordocError> {
    if depth > MAX_TABLE_NESTING {
        return Err(unsupported(
            "HWPML table nesting exceeds the supported depth",
        ));
    }
    if contains_named(node, "CAPTION") {
        return Err(unsupported("HWPML table captions are not supported"));
    }
    reject_table_in_skipped_wrappers(node)?;
    let declared_rows = dimension(node, "RowCount", MAX_SOURCE_TABLE_ROWS)?;
    let declared_cols = dimension(node, "ColCount", MAX_SOURCE_TABLE_COLS)?;

    let row_count = node
        .children
        .iter()
        .filter(|child| child.name == "ROW")
        .count();
    if row_count > MAX_SOURCE_TABLE_ROWS {
        return Err(table_too_large(
            "HWPML table row count exceeds supported bounds",
        ));
    }
    let source_cell_count = node
        .children
        .iter()
        .filter(|child| child.name == "ROW")
        .try_fold(0usize, |count, row| {
            count.checked_add(
                row.children
                    .iter()
                    .filter(|child| child.name == "CELL")
                    .count(),
            )
        })
        .ok_or_else(allocation_error)?;
    if source_cell_count == 0 {
        return Err(unsupported("HWPML table without cells is unsupported"));
    }

    // Validate all source geometry before parsing cell text or nested tables. This keeps an
    // obviously unsupported grid from causing retained text/block allocations first.
    let mut max_row_end = 0usize;
    let mut max_col_end = 0usize;
    for row in node.children.iter().filter(|child| child.name == "ROW") {
        for cell in row.children.iter().filter(|child| child.name == "CELL") {
            let row_addr = coordinate(cell, "RowAddr")? as usize;
            let col_addr = coordinate(cell, "ColAddr")? as usize;
            let row_span = span(cell, "RowSpan")? as usize;
            let col_span = span(cell, "ColSpan")? as usize;
            let row_end = row_addr.checked_add(row_span).ok_or_else(invalid_table)?;
            let col_end = col_addr.checked_add(col_span).ok_or_else(invalid_table)?;
            if row_addr >= declared_rows
                || col_addr >= declared_cols
                || row_end > declared_rows
                || col_end > declared_cols
            {
                return Err(unsupported("HWPML table cell exceeds declared dimensions"));
            }
            if col_end > kordoc_tables::MAX_TABLE_COLS {
                return Err(table_too_large(
                    "HWPML table exceeds the supported column limit",
                ));
            }
            if row_end > kordoc_tables::MAX_TABLE_ROWS {
                return Err(table_too_large(
                    "HWPML table exceeds the supported row limit",
                ));
            }
            max_row_end = max_row_end.max(row_end);
            max_col_end = max_col_end.max(col_end);
        }
    }
    if max_row_end
        .checked_mul(max_col_end)
        .is_none_or(|cells| cells > kordoc_tables::MAX_TABLE_CELLS)
    {
        return Err(table_too_large(
            "HWPML table exceeds the logical cell limit",
        ));
    }

    let mut source_rows = Vec::new();
    reserve_items(&mut source_rows, row_count, budget)?;
    let mut nested_sources = 0usize;
    for row in node.children.iter().filter(|child| child.name == "ROW") {
        let cell_count = row
            .children
            .iter()
            .filter(|child| child.name == "CELL")
            .count();
        let mut source_cells = Vec::new();
        reserve_items(&mut source_cells, cell_count, budget)?;
        for cell in row.children.iter().filter(|child| child.name == "CELL") {
            let row_addr = coordinate(cell, "RowAddr")?;
            let col_addr = coordinate(cell, "ColAddr")?;
            let row_span = span(cell, "RowSpan")?;
            let col_span = span(cell, "ColSpan")?;
            let (text, blocks) = lower_cell_content(cell, depth, budget, cell_budget, options)?;
            nested_sources = nested_sources
                .checked_add(usize::from(blocks.is_some()))
                .ok_or_else(allocation_error)?;
            source_cells.push(RichCellContext {
                cell: CellContext {
                    text,
                    row_addr: Some(row_addr),
                    col_addr: Some(col_addr),
                    row_span,
                    col_span,
                },
                blocks,
            });
        }
        source_rows.push(source_cells);
    }

    let table = build_table_with_blocks(
        source_rows,
        BuildTableOptions {
            max_rows: Some(MAX_SOURCE_TABLE_ROWS),
            keep_anchored_empty_cols: options.keep_trailing_empty_cols == Some(true),
            keep_empty_paragraphs: false,
        },
        cell_budget,
        budget,
    )?;
    let attached = table
        .cells
        .iter()
        .flatten()
        .filter(|cell| cell.blocks.is_some())
        .count();
    if attached != nested_sources {
        return Err(unsupported("HWPML nested table anchor is ambiguous"));
    }
    Ok(table)
}

fn lower_cell_content(
    cell: &XmlNode,
    parent_depth: usize,
    budget: &mut LoweringBudget,
    cell_budget: &mut TableCellBudget,
    options: &ParseOptions,
) -> Result<(String, Option<Vec<IrBlock>>), KordocError> {
    struct CellBuilder<'a> {
        parent_depth: usize,
        text: String,
        blocks: Vec<IrBlock>,
        has_nested_table: bool,
        collect_blocks: bool,
        budget: &'a mut LoweringBudget,
        cell_budget: &'a mut TableCellBudget,
        options: &'a ParseOptions,
    }

    impl CellBuilder<'_> {
        fn visit(&mut self, node: &XmlNode) -> Result<(), KordocError> {
            for content in &node.content {
                let XmlContent::Child(index) = content else {
                    continue;
                };
                let child = &node.children[*index];
                match child.name.as_str() {
                    "P" => {
                        let mut paragraph = String::new();
                        super::append_paragraph_chars(child, &mut paragraph, self.budget)?;
                        let paragraph = paragraph.trim();
                        if !paragraph.is_empty() {
                            append_line(&mut self.text, paragraph, self.budget)?;
                            if self.collect_blocks {
                                let owned = self.budget.copy_str(paragraph)?;
                                self.budget.push(
                                    &mut self.blocks,
                                    IrBlock {
                                        kind: IrBlockType::Paragraph,
                                        text: Some(owned),
                                        ..IrBlock::default()
                                    },
                                )?;
                            }
                        }
                        self.visit(child)?;
                    }
                    "TABLE" => {
                        self.has_nested_table = true;
                        let nested = lower_table(
                            child,
                            self.parent_depth + 1,
                            self.budget,
                            self.cell_budget,
                            self.options,
                        )?;
                        let nested_text = table_flat_text(&nested, self.budget)?;
                        if !nested_text.is_empty() {
                            append_line(&mut self.text, &nested_text, self.budget)?;
                        }
                        self.budget.push(
                            &mut self.blocks,
                            IrBlock {
                                kind: IrBlockType::Table,
                                table: Some(nested),
                                ..IrBlock::default()
                            },
                        )?;
                    }
                    "FOOTNOTE" | "ENDNOTE" | "HEADER" | "FOOTER" => {
                        if contains_named(child, "TABLE") {
                            return Err(unsupported(
                                "HWPML nested tables inside ignored cell wrappers are unsupported",
                            ));
                        }
                    }
                    "PICTURE" | "SHAPEOBJECT" | "AUTONUM" => {}
                    _ => self.visit(child)?,
                }
            }
            Ok(())
        }
    }

    let mut builder = CellBuilder {
        parent_depth,
        text: String::new(),
        blocks: Vec::new(),
        has_nested_table: false,
        collect_blocks: has_relevant_table(cell),
        budget,
        cell_budget,
        options,
    };
    builder.visit(cell)?;
    Ok((
        builder.text,
        builder.has_nested_table.then_some(builder.blocks),
    ))
}

fn has_relevant_table(node: &XmlNode) -> bool {
    node.children.iter().any(|child| match child.name.as_str() {
        "TABLE" => true,
        "PICTURE" | "SHAPEOBJECT" | "AUTONUM" => false,
        _ => has_relevant_table(child),
    })
}

fn reject_table_in_skipped_wrappers(node: &XmlNode) -> Result<(), KordocError> {
    for child in &node.children {
        if matches!(
            child.name.as_str(),
            "FOOTNOTE" | "ENDNOTE" | "HEADER" | "FOOTER"
        ) {
            if contains_named(child, "TABLE") {
                return Err(unsupported(
                    "HWPML nested tables inside ignored cell wrappers are unsupported",
                ));
            }
        } else {
            reject_table_in_skipped_wrappers(child)?;
        }
    }
    Ok(())
}

fn table_flat_text(table: &IrTable, budget: &mut LoweringBudget) -> Result<String, KordocError> {
    let mut text = String::new();
    for cell in table.cells.iter().flatten() {
        if cell.text.trim().is_empty() {
            continue;
        }
        append_line(&mut text, &cell.text, budget)?;
    }
    Ok(text)
}

fn append_line(
    target: &mut String,
    line: &str,
    budget: &mut LoweringBudget,
) -> Result<(), KordocError> {
    if !target.is_empty() {
        budget.append_str(target, "\n")?;
    }
    budget.append_str(target, line)
}

fn dimension(node: &XmlNode, attr: &str, max: usize) -> Result<usize, KordocError> {
    let parsed = decimal(node.attr(attr).ok_or_else(invalid_table)?)?;
    if parsed == 0 || parsed > max {
        return Err(unsupported(
            "HWPML table dimensions exceed supported bounds",
        ));
    }
    Ok(parsed)
}

fn coordinate(node: &XmlNode, attr: &str) -> Result<u32, KordocError> {
    let value = decimal(node.attr(attr).ok_or_else(invalid_table)?)?;
    u32::try_from(value).map_err(|_| unsupported("HWPML table coordinate exceeds supported bounds"))
}

fn span(node: &XmlNode, attr: &str) -> Result<u32, KordocError> {
    match node.attr(attr) {
        None => Ok(1),
        Some(value) => {
            let value = decimal(value)?;
            if value == 0 {
                return Err(invalid_table());
            }
            u32::try_from(value)
                .map_err(|_| unsupported("HWPML table span exceeds supported bounds"))
        }
    }
}

fn decimal(value: &str) -> Result<usize, KordocError> {
    if value.is_empty() || !value.bytes().all(|byte| byte.is_ascii_digit()) {
        return Err(invalid_table());
    }
    value.bytes().try_fold(0usize, |number, byte| {
        number
            .checked_mul(10)
            .and_then(|number| number.checked_add(usize::from(byte - b'0')))
            .ok_or_else(|| unsupported("HWPML table number exceeds supported bounds"))
    })
}

fn contains_named(node: &XmlNode, name: &str) -> bool {
    node.name == name
        || node
            .children
            .iter()
            .any(|child| contains_named(child, name))
}

fn reserve_items<T>(
    target: &mut Vec<T>,
    count: usize,
    budget: &mut LoweringBudget,
) -> Result<(), KordocError> {
    budget.charge_items::<T>(count)?;
    target
        .try_reserve_exact(count)
        .map_err(|_| allocation_error())
}

fn invalid_table() -> KordocError {
    KordocError::new(
        ErrorCode::UnsupportedFormat,
        "Malformed HWPML table structure is unsupported",
    )
}

fn unsupported(message: &'static str) -> KordocError {
    KordocError::new(ErrorCode::UnsupportedFormat, message)
}

fn allocation_error() -> KordocError {
    KordocError::new(
        ErrorCode::OutputTooLarge,
        "HWPML lowering exceeds its allocation limit",
    )
}

fn table_too_large(message: &'static str) -> KordocError {
    KordocError::new(ErrorCode::OutputTooLarge, message)
}
