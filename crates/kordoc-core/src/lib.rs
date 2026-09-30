//! Core document-processing operations for kordoc.

mod chunks;
mod detect;
mod hwpx;
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
pub use hwpx::{parse_hwpx_metadata, parse_hwpx_with_options, validate_hwpx};
pub use kordoc_hancom::{ValidateIssue, ValidateResult};
pub use kordoc_ir::{FileType, OcrOption, PageNumber, PageSelection, ParseOptions};
pub use limits::{MAX_ARCHIVE_ENTRIES, MAX_INPUT_BYTES, MAX_UNCOMPRESSED_BYTES};
pub use markdown::blocks_to_markdown;
pub use pages::blocks_to_pages;

#[cfg(feature = "fuzzing")]
pub mod fuzzing {
    use std::io::{Cursor, Write};

    use kordoc_ir::{ErrorCode, KordocError, ParsedDocument};
    use zip::{CompressionMethod, ZipWriter, write::SimpleFileOptions};

    /// Runs the bounded ZIP central-directory preflight without opening an archive.
    pub fn preflight_zip(bytes: &[u8]) -> bool {
        super::detect::preflight_zip(bytes).is_ok()
    }

    /// Exercise the independent Hancom package guards, including encrypted manifests.
    /// The fixed password is a public synthetic fixture value, not a credential.
    pub fn hwpx_package(bytes: &[u8]) -> Result<ParsedDocument, KordocError> {
        kordoc_hancom::parse_hwpx(
            bytes,
            &crate::ParseOptions {
                password: Some("fixture-password".into()),
                ..crate::ParseOptions::default()
            },
        )
    }

    /// Mutate one section inside a synthetic valid package with two sound neighbors.
    /// This feature-only harness caps inputs at 128 KiB, not the product XML limit.
    pub fn hwpx_xml(section: &[u8]) -> Result<ParsedDocument, KordocError> {
        if section.len() > 128 * 1024 {
            return Err(KordocError::new(
                ErrorCode::OutputTooLarge,
                "HWPX XML fuzz input exceeds the harness limit",
            ));
        }
        let entries: [(&str, &[u8]); 7] = [
            ("mimetype", b"application/hwp+zip"),
            ("META-INF/container.xml", b"<container/>"),
            (
                "Contents/content.hpf",
                b"<package><metadata><title>Fuzz fixture</title></metadata></package>",
            ),
            ("Contents/header.xml", b"<head secCnt=\"3\"/>"),
            (
                "Contents/section0.xml",
                b"<sec><p><run><t>before</t></run></p></sec>",
            ),
            ("Contents/section1.xml", section),
            (
                "Contents/section2.xml",
                b"<sec><p><run><t>after</t></run></p></sec>",
            ),
        ];
        let failure = || KordocError::new(ErrorCode::Corrupted, "Fuzz ZIP construction failed");
        let mut writer = ZipWriter::new(Cursor::new(Vec::new()));
        for (path, contents) in entries {
            writer
                .start_file(
                    path,
                    SimpleFileOptions::default().compression_method(CompressionMethod::Stored),
                )
                .map_err(|_| failure())?;
            writer.write_all(contents).map_err(|_| failure())?;
        }
        let bytes = writer.finish().map_err(|_| failure())?.into_inner();
        kordoc_hancom::parse_hwpx(&bytes, &crate::ParseOptions::default())
    }
}
