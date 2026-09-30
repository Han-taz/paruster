//! PDF parser scaffold.
//!
//! Public parser entry points are intentionally absent until P2a establishes bounded behavior.

mod cmap;
mod document;
mod geometry;
mod glyph;
mod headings;
mod image;
mod layout;
#[allow(
    dead_code,
    reason = "P2a Task 0 substrate is intentionally private until Task 1 wires parser semantics"
)]
mod limits;
mod lines;
mod links;
mod notes;
#[allow(
    dead_code,
    reason = "P2a Task 0 substrate is intentionally private until Task 1 wires parser semantics"
)]
mod objects;
mod parser;
mod quality;
mod regions;
#[allow(
    dead_code,
    reason = "P2a Task 0 substrate is intentionally private until Task 1 wires parser semantics"
)]
mod stream;
mod table;
mod text;
#[cfg(feature = "pdfjs-v8")]
#[allow(
    dead_code,
    reason = "private Task B runtime probe is not registered as a parser"
)]
mod v8_runtime;
