use std::io::{Cursor, Write};

use kordoc_core::{FileType, ParseOptions, try_parse_with_options};
use kordoc_ir::ErrorCode;
use zip::{CompressionMethod, ZipWriter, write::SimpleFileOptions};

fn hwpx(section: &str) -> Vec<u8> {
    let entries = [
        ("mimetype", "application/hwp+zip"),
        ("META-INF/container.xml", "<container/>"),
        (
            "Contents/content.hpf",
            "<package><metadata><title>Core fixture</title></metadata><manifest><item id=\"s0\" href=\"section0.xml\"/></manifest><spine><itemref idref=\"s0\"/></spine></package>",
        ),
        ("Contents/header.xml", "<head secCnt=\"1\"/>"),
        ("Contents/section0.xml", section),
    ];
    let mut writer = ZipWriter::new(Cursor::new(Vec::new()));
    for (path, contents) in entries {
        writer
            .start_file(
                path,
                SimpleFileOptions::default().compression_method(CompressionMethod::Stored),
            )
            .unwrap();
        writer.write_all(contents.as_bytes()).unwrap();
    }
    writer.finish().unwrap().into_inner()
}

#[test]
fn hwpx_parse_assembles_markdown_in_core() {
    let bytes = hwpx("<sec><p><run><t>Core body</t></run></p></sec>");
    let result = try_parse_with_options(&bytes, &ParseOptions::default()).unwrap();
    assert!(result.success);
    assert_eq!(result.file_type, FileType::Hwpx);
    assert_eq!(result.markdown, "Core body");
    assert_eq!(result.page_count, Some(1));
    assert_eq!(
        result.metadata.unwrap().title.as_deref(),
        Some("Core fixture")
    );
    assert_eq!(result.pages.unwrap()[0].markdown, "Core body");
}

#[test]
fn hwpx_dispatch_uses_strict_detector_first() {
    let mut bytes = hwpx("<sec><p/></sec>");
    let end = bytes.len() - 22;
    bytes[end + 12..end + 16].copy_from_slice(&u32::MAX.to_le_bytes());
    let error = try_parse_with_options(&bytes, &ParseOptions::default()).unwrap_err();
    assert_eq!(error.file_type, FileType::Unknown);
    assert_eq!(error.code, ErrorCode::ZipBomb);
}

#[test]
fn hwpx_metadata_uses_specialized_path_without_reading_section_xml() {
    let bytes = hwpx("<sec><unclosed>");
    let metadata = kordoc_core::parse_hwpx_metadata(&bytes, &ParseOptions::default()).unwrap();
    assert_eq!(metadata.title.as_deref(), Some("Core fixture"));
}

#[test]
fn format_specific_entrypoints_reject_other_formats() {
    let options = ParseOptions::default();
    let bytes = b"%PDF-1.7\n";
    for error in [
        kordoc_core::parse_hwpx_with_options(bytes, &options).unwrap_err(),
        kordoc_core::parse_hwpx_metadata(bytes, &options).unwrap_err(),
        kordoc_core::validate_hwpx(bytes, None).unwrap_err(),
    ] {
        assert_eq!(error.file_type, FileType::Pdf);
        assert_eq!(error.code, ErrorCode::UnsupportedFormat);
    }
}

#[test]
fn hwpx_validator_preserves_order_and_files_only_count() {
    let result = kordoc_core::validate_hwpx(&hwpx("<sec/>"), None).unwrap();
    assert!(result.ok);
    assert!(result.issues.is_empty());
    assert_eq!(result.entry_count, 5);
}
