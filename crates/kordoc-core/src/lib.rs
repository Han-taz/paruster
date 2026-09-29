//! Core document-processing operations for kordoc.

mod detect;
mod limits;

pub use detect::{ParseDispatchError, detect_format, try_parse};
pub use kordoc_ir::FileType;
pub use limits::{MAX_ARCHIVE_ENTRIES, MAX_INPUT_BYTES, MAX_UNCOMPRESSED_BYTES};
