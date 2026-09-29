//! Core document-processing operations for kordoc.

mod chunks;
mod detect;
mod limits;
pub mod markdown;
pub mod markdown_units;
mod options;
mod pages;
mod parse;
pub mod table;

pub use chunks::blocks_to_chunks;

pub use detect::{
    ParseDispatchError, detect_format, detect_ole2_format, detect_zip_format, is_hwpx_file,
    is_old_hwp_file, is_pdf_file, is_zip_file, try_parse, try_parse_with_options,
};
pub use kordoc_ir::{FileType, OcrOption, PageNumber, PageSelection, ParseOptions};
pub use limits::{MAX_ARCHIVE_ENTRIES, MAX_INPUT_BYTES, MAX_UNCOMPRESSED_BYTES};
pub use markdown::blocks_to_markdown;
pub use pages::blocks_to_pages;

#[cfg(feature = "fuzzing")]
pub mod fuzzing {
    /// Runs the bounded ZIP central-directory preflight without opening an archive.
    pub fn preflight_zip(bytes: &[u8]) -> bool {
        super::detect::preflight_zip(bytes).is_ok()
    }
}
