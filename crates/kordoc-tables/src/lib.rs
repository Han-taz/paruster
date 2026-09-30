//! Source-neutral bounded table construction shared by format adapters.
//!
//! The crate owns cell placement and rectangular IR construction only. Source XML interpretation,
//! warnings, captions, and parser policies belong to the caller.

use std::collections::HashMap;

use kordoc_ir::{ErrorCode, IrBlock, IrCell, IrTable, KordocError};

pub const MAX_TABLE_COLS: usize = 200;
pub const MAX_TABLE_ROWS: usize = 10_000;
pub const MAX_TABLE_CELLS: usize = 2_000_000;
const MAX_BLOCK_DEPTH: usize = 64;

/// One source cell before it is placed in the output grid.
///
/// The fields intentionally match the former core-private type so core callers and tests keep
/// their existing struct literals.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CellContext {
    pub text: String,
    pub col_span: u32,
    pub row_span: u32,
    pub col_addr: Option<u32>,
    pub row_addr: Option<u32>,
}

/// Generic grid policies formerly owned by `kordoc-core`.
///
/// Keep this type's fields stable for existing struct literals. Format-specific column and
/// coordinate policies are applied by the format adapter before it calls the generic builder.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct BuildTableOptions {
    pub max_rows: Option<usize>,
    pub keep_anchored_empty_cols: bool,
    pub keep_empty_paragraphs: bool,
}

/// Caller-provided accounting for memory allocated by a table build.
pub trait AllocationBudget {
    fn charge_bytes(&mut self, bytes: usize) -> Result<(), KordocError>;
}

/// Aggregate logical-grid budget shared by a caller across all tables it constructs.
#[derive(Debug, Clone, Copy)]
pub struct TableCellBudget {
    used: usize,
    limit: usize,
}

impl Default for TableCellBudget {
    fn default() -> Self {
        Self {
            used: 0,
            limit: MAX_TABLE_CELLS,
        }
    }
}

impl TableCellBudget {
    fn checkpoint(&self) -> usize {
        self.used
    }

    fn rollback(&mut self, checkpoint: usize) {
        self.used = checkpoint.min(self.used);
    }

    fn charge_grid(&mut self, rows: usize, columns: usize) -> Result<usize, KordocError> {
        let cells = rows.checked_mul(columns).ok_or_else(cell_limit)?;
        let next = self.used.checked_add(cells).ok_or_else(cell_limit)?;
        if next > self.limit {
            return Err(cell_limit());
        }
        self.used = next;
        Ok(cells)
    }

    #[cfg(test)]
    pub(crate) fn with_limit(limit: usize) -> Self {
        Self { used: 0, limit }
    }
}

/// Rich input for parsers that need to retain structured cell content.
///
/// This is separate from `CellContext`, preserving all existing cell-context literals. The caller
/// owns and accounts for these input payloads; the consuming builder transfers them where source
/// coordinates and built text/spans still identify the same unclaimed output anchor.
#[derive(Debug, Clone, PartialEq)]
pub struct RichCellContext {
    pub cell: CellContext,
    pub blocks: Option<Vec<IrBlock>>,
}

#[derive(Debug, Clone, Copy)]
struct CellPlacement {
    row: usize,
    col: usize,
    col_span: usize,
    row_span: usize,
    owner: Option<(usize, usize)>,
}

struct NoopBudget;

impl AllocationBudget for NoopBudget {
    fn charge_bytes(&mut self, _bytes: usize) -> Result<(), KordocError> {
        Ok(())
    }
}

/// Compatibility builder with the original two-argument API and limits.
///
/// It uses a no-op allocation meter, as before this crate existed, while retaining checked
/// geometry, fallible reservations, and the same 200-column / 10,000-row / 2,000,000-cell caps.
pub fn build_table(
    rows: &[Vec<CellContext>],
    options: BuildTableOptions,
) -> Result<IrTable, KordocError> {
    build_table_with_budget(
        rows,
        options,
        &mut TableCellBudget::default(),
        &mut NoopBudget,
    )
}

/// Metered borrowed-input builder. Text is copied into the returned IR after being charged.
pub fn build_table_with_budget<B: AllocationBudget>(
    rows: &[Vec<CellContext>],
    options: BuildTableOptions,
    cells: &mut TableCellBudget,
    allocations: &mut B,
) -> Result<IrTable, KordocError> {
    let checkpoint = cells.checkpoint();
    match build_borrowed_inner(rows, options, cells, allocations) {
        Ok(table) => Ok(table),
        Err(error) => {
            cells.rollback(checkpoint);
            Err(error)
        }
    }
}

/// Metered consuming builder for cells with nested IR content.
///
/// Grid text and nested block payloads are moved rather than cloned. Collision text follows the
/// compatibility builder's newline rule; blocks are attached only when the source cell remains an
/// exact, unclaimed anchor. Blocks from collision/merged cells are intentionally left unattached
/// rather than guessed or concatenated.
pub fn build_table_with_blocks<B: AllocationBudget>(
    rows: Vec<Vec<RichCellContext>>,
    options: BuildTableOptions,
    cells: &mut TableCellBudget,
    allocations: &mut B,
) -> Result<IrTable, KordocError> {
    let checkpoint = cells.checkpoint();
    match build_rich_inner(rows, options, cells, allocations) {
        Ok(table) => Ok(table),
        Err(error) => {
            cells.rollback(checkpoint);
            Err(error)
        }
    }
}

fn build_borrowed_inner<B: AllocationBudget>(
    rows: &[Vec<CellContext>],
    options: BuildTableOptions,
    cells: &mut TableCellBudget,
    allocations: &mut B,
) -> Result<IrTable, KordocError> {
    let (row_count, col_count, placements) = simulate_placements(rows, options, allocations)?;
    if row_count == 0 || col_count == 0 {
        return Ok(IrTable::default());
    }
    let logical_cells = cells.charge_grid(row_count, col_count)?;
    let mut grid = allocate_grid(row_count, col_count, logical_cells, allocations)?;
    let mut anchor_cols = allocate_anchor_columns(col_count, allocations)?;

    for (row, sources) in rows.iter().enumerate() {
        for (source, placement) in sources.iter().zip(&placements[row]) {
            let value = if options.keep_empty_paragraphs {
                source.text.as_str()
            } else {
                source.text.trim()
            };
            if let Some((owner_row, owner_col)) = placement.owner {
                append_collision_text(&mut grid[owner_row][owner_col].text, value, allocations)?;
                continue;
            }
            let text = copy_text(value, allocations)?;
            grid[placement.row][placement.col] = IrCell {
                text,
                col_span: placement.col_span as u32,
                row_span: placement.row_span as u32,
                ..IrCell::default()
            };
            anchor_cols[placement.col] = true;
        }
    }
    finish_table(grid, row_count, col_count, anchor_cols, options)
}

fn build_rich_inner<B: AllocationBudget>(
    mut rows: Vec<Vec<RichCellContext>>,
    options: BuildTableOptions,
    cells: &mut TableCellBudget,
    allocations: &mut B,
) -> Result<IrTable, KordocError> {
    for row in &rows {
        for source in row {
            if let Some(blocks) = source.blocks.as_deref() {
                // The built table occupies level one; cell content begins at level two.
                validate_block_depth(blocks, 2)?;
            }
        }
    }
    let (row_count, col_count, placements) = simulate_rich_placements(&rows, options, allocations)?;
    if row_count == 0 || col_count == 0 {
        return Ok(IrTable::default());
    }
    let logical_cells = cells.charge_grid(row_count, col_count)?;
    let source_text_matches = source_text_match_flags(&rows, options, allocations)?;
    let mut grid = allocate_grid(row_count, col_count, logical_cells, allocations)?;
    let mut anchor_cols = allocate_anchor_columns(col_count, allocations)?;
    charge_items::<bool>(allocations, logical_cells)?;
    let mut collided = Vec::new();
    collided
        .try_reserve_exact(logical_cells)
        .map_err(|_| allocation_error())?;
    collided.resize(logical_cells, false);
    for (row, sources) in rows.iter_mut().enumerate() {
        for (source, placement) in sources.iter_mut().zip(&placements[row]) {
            let value = if options.keep_empty_paragraphs {
                std::mem::take(&mut source.cell.text)
            } else {
                trim_owned(std::mem::take(&mut source.cell.text))
            };
            if let Some((owner_row, owner_col)) = placement.owner {
                let changed = append_owned_collision_text(
                    &mut grid[owner_row][owner_col].text,
                    value,
                    allocations,
                )?;
                if changed {
                    let owner_index = owner_row
                        .checked_mul(col_count)
                        .and_then(|base| base.checked_add(owner_col))
                        .ok_or_else(cell_limit)?;
                    collided[owner_index] = true;
                }
                continue;
            }
            let cell = IrCell {
                text: value,
                col_span: placement.col_span as u32,
                row_span: placement.row_span as u32,
                ..IrCell::default()
            };
            grid[placement.row][placement.col] = cell;
            anchor_cols[placement.col] = true;
        }
    }
    for (row, sources) in rows.into_iter().enumerate() {
        for (column, (source, placement)) in sources.into_iter().zip(&placements[row]).enumerate() {
            if placement.owner.is_some() || !source_text_matches[row][column] {
                continue;
            }
            let index = placement
                .row
                .checked_mul(col_count)
                .and_then(|base| base.checked_add(placement.col))
                .ok_or_else(cell_limit)?;
            let target = &mut grid[placement.row][placement.col];
            if collided[index]
                || target.col_span != source.cell.col_span
                || target.row_span != source.cell.row_span
            {
                continue;
            }
            if let Some(blocks) = source.blocks {
                target.blocks = Some(blocks);
            }
        }
    }
    finish_table(grid, row_count, col_count, anchor_cols, options)
}

fn source_text_match_flags<B: AllocationBudget>(
    rows: &[Vec<RichCellContext>],
    options: BuildTableOptions,
    allocations: &mut B,
) -> Result<Vec<Vec<bool>>, KordocError> {
    charge_items::<Vec<bool>>(allocations, rows.len())?;
    let mut matches = Vec::new();
    matches
        .try_reserve_exact(rows.len())
        .map_err(|_| allocation_error())?;
    for row in rows {
        charge_items::<bool>(allocations, row.len())?;
        let mut row_matches = Vec::new();
        row_matches
            .try_reserve_exact(row.len())
            .map_err(|_| allocation_error())?;
        row_matches.extend(row.iter().map(|source| {
            if options.keep_empty_paragraphs {
                source.cell.text.as_str() == source.cell.text
            } else {
                source.cell.text.trim() == source.cell.text
            }
        }));
        matches.push(row_matches);
    }
    Ok(matches)
}

fn simulate_placements(
    rows: &[Vec<CellContext>],
    options: BuildTableOptions,
    allocations: &mut impl AllocationBudget,
) -> Result<(usize, usize, Vec<Vec<CellPlacement>>), KordocError> {
    simulate_by(rows, options, allocations, |cell| cell)
}

fn simulate_rich_placements(
    rows: &[Vec<RichCellContext>],
    options: BuildTableOptions,
    allocations: &mut impl AllocationBudget,
) -> Result<(usize, usize, Vec<Vec<CellPlacement>>), KordocError> {
    simulate_by(rows, options, allocations, |cell| &cell.cell)
}

fn simulate_by<R, F>(
    rows: &[Vec<R>],
    options: BuildTableOptions,
    allocations: &mut impl AllocationBudget,
    cell_context: F,
) -> Result<(usize, usize, Vec<Vec<CellPlacement>>), KordocError>
where
    F: Fn(&R) -> &CellContext,
{
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
        .map(&cell_context)
        .any(|cell| cell.col_addr.is_some() || cell.row_addr.is_some());

    charge_items::<Vec<CellPlacement>>(allocations, rows.len())?;
    let mut placements = Vec::new();
    placements
        .try_reserve_exact(rows.len())
        .map_err(|_| allocation_error())?;
    let mut occupied = HashMap::<(usize, usize), (usize, usize)>::new();
    let mut row_count = rows.len();
    let mut col_count = 0usize;

    for (row_index, row) in rows.iter().enumerate() {
        charge_items::<CellPlacement>(allocations, row.len())?;
        let mut row_placements = Vec::new();
        row_placements
            .try_reserve_exact(row.len())
            .map_err(|_| allocation_error())?;
        let mut next_col = 0usize;
        for input in row {
            let cell = cell_context(input);
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
            let occupied_cells = row_span
                .checked_mul(col_span)
                .ok_or_else(|| too_large("Table span area overflow"))?;
            let occupied_len = occupied
                .len()
                .checked_add(occupied_cells)
                .ok_or_else(allocation_error)?;
            if occupied_len > occupied.capacity() {
                // HashMap may round bucket capacity up and temporarily hold both old and new
                // tables during growth. Four entry widths per requested slot conservatively
                // account for bucket rounding and control bytes before reserve can allocate.
                charge_bytes(
                    allocations,
                    occupied_len
                        .checked_mul(std::mem::size_of::<((usize, usize), (usize, usize))>())
                        .and_then(|bytes| bytes.checked_mul(4))
                        .ok_or_else(allocation_error)?,
                )?;
            }
            occupied
                .try_reserve(occupied_cells)
                .map_err(|_| allocation_error())?;
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

fn allocate_grid(
    rows: usize,
    columns: usize,
    logical_cells: usize,
    allocations: &mut impl AllocationBudget,
) -> Result<Vec<Vec<IrCell>>, KordocError> {
    charge_items::<Vec<IrCell>>(allocations, rows)?;
    charge_items::<IrCell>(allocations, logical_cells)?;
    let mut grid = Vec::new();
    grid.try_reserve_exact(rows)
        .map_err(|_| allocation_error())?;
    for _ in 0..rows {
        let mut row = Vec::new();
        row.try_reserve_exact(columns)
            .map_err(|_| allocation_error())?;
        row.resize_with(columns, empty_cell);
        grid.push(row);
    }
    Ok(grid)
}

fn empty_cell() -> IrCell {
    IrCell {
        col_span: 1,
        row_span: 1,
        ..IrCell::default()
    }
}

fn allocate_anchor_columns(
    columns: usize,
    allocations: &mut impl AllocationBudget,
) -> Result<Vec<bool>, KordocError> {
    charge_items::<bool>(allocations, columns)?;
    let mut anchor_cols = Vec::new();
    anchor_cols
        .try_reserve_exact(columns)
        .map_err(|_| allocation_error())?;
    anchor_cols.resize(columns, false);
    Ok(anchor_cols)
}

fn finish_table(
    mut grid: Vec<Vec<IrCell>>,
    row_count: usize,
    col_count: usize,
    anchor_cols: Vec<bool>,
    options: BuildTableOptions,
) -> Result<IrTable, KordocError> {
    let mut effective_cols = col_count;
    while effective_cols > 0 {
        let last_is_textless = grid
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
    for row in &mut grid {
        row.truncate(effective_cols);
        for (column, cell) in row.iter_mut().enumerate() {
            let remaining = effective_cols - column;
            if cell.col_span as usize > remaining {
                cell.col_span = remaining as u32;
            }
        }
    }
    Ok(IrTable {
        rows: row_count as u32,
        cols: effective_cols as u32,
        cells: grid,
        has_header: row_count > 1,
        ..IrTable::default()
    })
}

fn copy_text(source: &str, allocations: &mut impl AllocationBudget) -> Result<String, KordocError> {
    charge_bytes(allocations, source.len())?;
    let mut text = String::new();
    text.try_reserve_exact(source.len())
        .map_err(|_| allocation_error())?;
    text.push_str(source);
    Ok(text)
}

fn append_collision_text(
    target: &mut String,
    source: &str,
    allocations: &mut impl AllocationBudget,
) -> Result<(), KordocError> {
    if source.trim().is_empty() {
        return Ok(());
    }
    let separator = usize::from(!target.is_empty());
    let required = source
        .len()
        .checked_add(separator)
        .ok_or_else(allocation_error)?;
    charge_bytes(allocations, required)?;
    target
        .try_reserve_exact(required)
        .map_err(|_| allocation_error())?;
    if separator != 0 {
        target.push('\n');
    }
    target.push_str(source);
    Ok(())
}

fn append_owned_collision_text(
    target: &mut String,
    source: String,
    allocations: &mut impl AllocationBudget,
) -> Result<bool, KordocError> {
    if source.trim().is_empty() {
        return Ok(false);
    }
    if target.is_empty() {
        *target = source;
        return Ok(true);
    }
    let required = source.len().checked_add(1).ok_or_else(allocation_error)?;
    charge_bytes(allocations, required)?;
    target
        .try_reserve_exact(required)
        .map_err(|_| allocation_error())?;
    target.push('\n');
    target.push_str(&source);
    Ok(true)
}

fn trim_owned(mut text: String) -> String {
    let start = text.trim_start().len();
    if start != 0 {
        text.drain(..text.len() - start);
    }
    let end = text.trim_end().len();
    text.truncate(end);
    text
}

fn validate_block_depth(blocks: &[IrBlock], depth: usize) -> Result<(), KordocError> {
    if depth > MAX_BLOCK_DEPTH {
        return Err(too_large("Block nesting exceeds the projection limit"));
    }
    for block in blocks {
        if let Some(children) = block.children.as_deref() {
            validate_block_depth(children, depth + 1)?;
        }
        if let Some(table) = block.table.as_ref() {
            if let Some(caption) = table.caption_blocks.as_deref() {
                validate_block_depth(caption, depth + 1)?;
            }
            for cell in table.cells.iter().flatten() {
                if let Some(children) = cell.blocks.as_deref() {
                    validate_block_depth(children, depth + 1)?;
                }
            }
        }
    }
    Ok(())
}

fn charge_items<T>(
    allocations: &mut impl AllocationBudget,
    count: usize,
) -> Result<(), KordocError> {
    let bytes = std::mem::size_of::<T>()
        .checked_mul(count)
        .ok_or_else(allocation_error)?;
    charge_bytes(allocations, bytes)
}

fn charge_bytes(allocations: &mut impl AllocationBudget, bytes: usize) -> Result<(), KordocError> {
    allocations.charge_bytes(bytes)
}

fn too_large(message: &'static str) -> KordocError {
    KordocError::new(ErrorCode::OutputTooLarge, message)
}

fn cell_limit() -> KordocError {
    too_large("Table exceeds the cell budget")
}

fn allocation_error() -> KordocError {
    too_large("Table allocation failed or exceeds the configured limit")
}

#[cfg(test)]
mod tests;
