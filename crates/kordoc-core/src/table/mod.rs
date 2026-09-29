//! Bounded construction and policy helpers for IR tables.

use kordoc_ir::{IrBlockType, IrTable};

pub(crate) mod builder;
pub mod classifier;
pub mod label;
pub mod visual;

pub(crate) fn has_structured_cell_content(table: &IrTable) -> bool {
    table.cells.iter().any(|row| {
        row.iter().any(|cell| {
            cell.blocks.as_ref().is_some_and(|blocks| {
                blocks.iter().any(|block| {
                    (block.kind == IrBlockType::Table && block.table.is_some())
                        || block.kind == IrBlockType::Separator
                })
            })
        })
    })
}
