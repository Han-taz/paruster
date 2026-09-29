//! Core document-processing operations for kordoc.

mod detect;
mod limits;

pub use detect::{ParseDispatchError, detect_format, try_parse};
pub use kordoc_ir::FileType;
pub use limits::{MAX_ARCHIVE_ENTRIES, MAX_INPUT_BYTES, MAX_UNCOMPRESSED_BYTES};

#[cfg(feature = "fuzzing")]
pub mod fuzzing {
    /// Runs the bounded ZIP central-directory preflight without opening an archive.
    pub fn preflight_zip(bytes: &[u8]) -> bool {
        super::detect::preflight_zip(bytes).is_ok()
    }
}
