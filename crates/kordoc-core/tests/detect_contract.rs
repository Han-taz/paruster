use std::io::{Cursor, Write};

use kordoc_core::{
    FileType, detect_format, detect_ole2_format, detect_zip_format, is_hwpx_file, is_old_hwp_file,
    is_pdf_file, is_zip_file, try_parse,
};
use kordoc_ir::ErrorCode;
use zip::ZipWriter;
use zip::write::SimpleFileOptions;

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

fn compound_with_stream(path: &str) -> Vec<u8> {
    let mut compound = cfb::CompoundFile::create(Cursor::new(Vec::new())).unwrap();
    if let Some(parent) = path
        .rsplit_once('/')
        .map(|(parent, _)| parent)
        .filter(|parent| !parent.is_empty() && *parent != "/")
    {
        compound.create_storage(parent).unwrap();
    }
    compound
        .create_stream(path)
        .unwrap()
        .write_all(b"fixture")
        .unwrap();
    compound.into_inner().into_inner()
}

#[test]
fn detects_magic_bytes_in_protocol_precedence_order() {
    assert_eq!(
        detect_format(b"HWP Document File V3.00\x1a\x01").unwrap(),
        FileType::Hwp3
    );
    assert_eq!(detect_format(b"%PDF-1.7\n").unwrap(), FileType::Pdf);
    assert_eq!(detect_format(b"%PDF").unwrap(), FileType::Pdf);
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
        detect_format(&zip_with_names(&["mimetype"])).unwrap(),
        FileType::Hwpx
    );
    assert_eq!(
        detect_format(&zip_with_names(&["Contents/section0.xml"])).unwrap(),
        FileType::Hwpx
    );
    assert_eq!(
        detect_format(&zip_with_names(&["Contents/unrelated.bin"])).unwrap(),
        FileType::Hwpx
    );
    assert_eq!(
        detect_format(&zip_with_names(&[
            "ppt/presentation.xml",
            "Contents/content.hpf"
        ]))
        .unwrap(),
        FileType::Pptx
    );
    assert_eq!(
        detect_format(&zip_with_names(&[
            "word/document.xml",
            "ppt/presentation.xml",
            "Contents/content.hpf"
        ]))
        .unwrap(),
        FileType::Docx
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
fn refines_ole_streams_and_ignores_storage_lookalikes() {
    assert_eq!(
        detect_format(&compound_with_stream("/Workbook")).unwrap(),
        FileType::Xls
    );
    assert_eq!(
        detect_format(&compound_with_stream("/ObjectPool/Book")).unwrap(),
        FileType::Xls
    );
    assert_eq!(
        detect_format(&compound_with_stream("/FileHeader")).unwrap(),
        FileType::Hwp
    );
    assert_eq!(
        detect_format(&compound_with_stream("/BodyText/Section9")).unwrap(),
        FileType::Hwp
    );

    let mut compound = cfb::CompoundFile::create(Cursor::new(Vec::new())).unwrap();
    compound.create_storage("/Workbook").unwrap();
    let bytes = compound.into_inner().into_inner();
    assert_eq!(detect_format(&bytes).unwrap(), FileType::Unknown);
}

#[test]
fn empty_and_unsupported_inputs_return_stable_errors() {
    let empty = try_parse(&[]).unwrap_err();
    assert_eq!(empty.code, ErrorCode::EmptyInput);
    assert_eq!(empty.message, "빈 버퍼이거나 유효하지 않은 입력입니다.");
    assert_eq!(
        try_parse(b"not a document").unwrap_err().code,
        ErrorCode::UnsupportedFormat
    );
    let error = try_parse(b"%PDF-1.7\n").unwrap_err();
    assert_eq!(error.code, ErrorCode::UnsupportedFormat);
    assert_eq!(error.file_type, FileType::Pdf);
}

#[test]
fn malformed_zip_security_errors_remain_distinguishable() {
    let mut zip = zip_with_names(&["xl/workbook.xml"]);
    let eocd = zip.len() - 22;
    zip[eocd + 12..eocd + 16].copy_from_slice(&u32::MAX.to_le_bytes());
    assert_eq!(detect_format(&zip).unwrap_err().code, ErrorCode::ZipBomb);
    let error = try_parse(&zip).unwrap_err();
    assert_eq!(error.code, ErrorCode::ZipBomb);
    assert_eq!(error.file_type, FileType::Unknown);
}

#[test]
fn hwpml_marker_is_detected_after_pdf_and_before_image_signatures() {
    assert_eq!(
        detect_format(b" \n<?xml version=\"1.0\"?><HWPML>").unwrap(),
        FileType::Hwpml
    );
    assert_eq!(
        detect_format(b"<?xml version=\"1.0\"?><HWPML>\x89PNG\r\n\x1a\n").unwrap(),
        FileType::Hwpml
    );
    assert_eq!(
        detect_format(b"plain text <HWPML>").unwrap(),
        FileType::Unknown
    );
    assert_eq!(
        detect_format(b"\x89PNG\r\n\x1a\n").unwrap(),
        FileType::Image
    );
}

#[test]
fn truncated_signatures_are_not_recognized() {
    for bytes in [
        &b"HWP Document File V3.0"[..],
        &b"\xd0\xcf\x11\xe0\xa1\xb1\x1a"[..],
        &b"%PD"[..],
        &b"\x89PNG\r\n\x1a"[..],
        &b"\xff\xd8"[..],
        &b"RIFF\0\0\0\0WEB"[..],
    ] {
        assert_eq!(detect_format(bytes).unwrap(), FileType::Unknown);
    }
    assert_eq!(
        detect_format(b"HWP Document File V3.00").unwrap(),
        FileType::Hwp3
    );
    assert_eq!(detect_format(b"PK\x07\x08").unwrap(), FileType::Unknown);
}

#[test]
fn legacy_magic_predicates_use_exact_four_byte_prefixes() {
    assert!(is_zip_file(b"PK\x03\x04"));
    assert!(is_hwpx_file(b"PK\x03\x04not-a-package"));
    assert!(is_old_hwp_file(b"\xd0\xcf\x11\xe0"));
    assert!(is_pdf_file(b"%PDF"));
    assert!(!is_zip_file(b"PK\x05\x06"));
    assert!(!is_old_hwp_file(b"\xd0\xcf\x11"));
    assert!(!is_pdf_file(b"%PD"));
}

#[test]
fn malformed_container_refinement_returns_unknown() {
    assert_eq!(detect_zip_format(b"PK\x03\x04truncated"), FileType::Unknown);
    assert_eq!(
        detect_ole2_format(b"\xd0\xcf\x11\xe0truncated"),
        FileType::Unknown
    );
}

#[test]
fn legacy_ole_refinement_accepts_section_names_at_any_location() {
    let root_section = compound_with_stream("/SectionX");
    assert_eq!(detect_ole2_format(&root_section), FileType::Hwp);
    assert_eq!(detect_format(&root_section).unwrap(), FileType::Unknown);

    let nested_section = compound_with_stream("/ObjectPool/SectionCustom");
    assert_eq!(detect_ole2_format(&nested_section), FileType::Hwp);
}
