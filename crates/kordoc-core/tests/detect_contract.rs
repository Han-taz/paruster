use std::io::{Cursor, Write};

use kordoc_core::{FileType, detect_format, try_parse};
use kordoc_ir::ErrorCode;
use zip::write::SimpleFileOptions;
use zip::ZipWriter;

fn zip_with_names(names: &[&str]) -> Vec<u8> {
    let mut writer = ZipWriter::new(Cursor::new(Vec::new()));
    for name in names {
        writer
            .start_file(name, SimpleFileOptions::default())
            .unwrap();
        writer.write_all(b"x").unwrap();
    }
    writer.finish().unwrap().into_inner()
}

#[test]
fn detects_magic_bytes_in_protocol_precedence_order() {
    assert_eq!(
        detect_format(b"HWP Document File V3.00\x1a\x01").unwrap(),
        FileType::Hwp3
    );
    assert_eq!(detect_format(b"%PDF-1.7\n").unwrap(), FileType::Pdf);
    assert_eq!(
        detect_format(b"\x89PNG\r\n\x1a\n").unwrap(),
        FileType::Image
    );
    assert_eq!(detect_format(b"\xff\xd8\xff\xe0").unwrap(), FileType::Image);
    assert_eq!(detect_format(b"RIFF\0\0\0\0WEBP").unwrap(), FileType::Image);
}

#[test]
fn refines_zip_names_with_exact_case_and_marker_precedence() {
    assert_eq!(
        detect_format(&zip_with_names(&["ppt/presentation.xml"])).unwrap(),
        FileType::Pptx
    );
    assert_eq!(
        detect_format(&zip_with_names(&["Contents/content.hpf"])).unwrap(),
        FileType::Hwpx
    );
    assert_eq!(
        detect_format(&zip_with_names(&["word/document.xml", "xl/workbook.xml"])).unwrap(),
        FileType::Xlsx
    );
    assert_eq!(
        detect_format(&zip_with_names(&["XL/workbook.xml"])).unwrap(),
        FileType::Unknown
    );
    assert_eq!(
        detect_format(&zip_with_names(&["unrelated.txt"])).unwrap(),
        FileType::Unknown
    );
    assert_eq!(
        detect_format(&zip_with_names(&[])).unwrap(),
        FileType::Unknown
    );
}

#[test]
fn empty_and_unsupported_inputs_return_stable_errors() {
    assert_eq!(try_parse(&[]).unwrap_err().code, ErrorCode::EmptyInput);
    assert_eq!(
        try_parse(b"not a document").unwrap_err().code,
        ErrorCode::UnsupportedFormat
    );
    assert_eq!(
        try_parse(b"%PDF-1.7\n").unwrap_err().code,
        ErrorCode::UnsupportedFormat
    );
}

#[test]
fn hwpml_marker_is_detected_after_pdf_and_before_image_signatures() {
    assert_eq!(
        detect_format(b"<?xml version=\"1.0\"?><HWPML>").unwrap(),
        FileType::Hwpml
    );
    assert_eq!(
        detect_format(b"\x89PNG\r\n\x1a\n").unwrap(),
        FileType::Image
    );
}
