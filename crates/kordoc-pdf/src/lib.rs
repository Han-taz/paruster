//! PDF parser scaffold.
//!
//! Public parser entry points are intentionally absent until P2a establishes bounded behavior.

mod cmap;
mod document;
#[cfg(feature = "pdfjs-v8")]
#[allow(
    dead_code,
    reason = "private geometry checkpoint is not yet wired to PDF parser layout"
)]
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

#[cfg(feature = "pdfjs-v8")]
#[allow(
    dead_code,
    reason = "private worker protocol is not a registered parser boundary"
)]
mod worker_protocol;

#[cfg(feature = "pdfjs-v8")]
#[allow(
    dead_code,
    reason = "private process supervisor is not a registered parser"
)]
mod worker_supervisor;
