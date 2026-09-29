//! Core document-processing operations for kordoc.

mod detect;
mod limits;
mod options;
mod parse;

pub use detect::{
    ParseDispatchError, detect_format, detect_ole2_format, detect_zip_format, is_hwpx_file,
    is_old_hwp_file, is_pdf_file, is_zip_file, try_parse, try_parse_with_options,
};
pub use kordoc_ir::{FileType, OcrOption, PageNumber, PageSelection, ParseOptions};
pub use limits::{MAX_ARCHIVE_ENTRIES, MAX_INPUT_BYTES, MAX_UNCOMPRESSED_BYTES};

#[cfg(feature = "fuzzing")]
pub mod fuzzing {
    /// Runs the bounded ZIP central-directory preflight without opening an archive.
    pub fn preflight_zip(bytes: &[u8]) -> bool {
        super::detect::preflight_zip(bytes).is_ok()
    }
}
