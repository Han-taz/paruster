#![cfg(feature = "pdfjs-v8")]

#[allow(
    dead_code,
    reason = "this focused integration target reuses the private V8 runtime module"
)]
#[path = "../src/v8_runtime/mod.rs"]
mod v8_runtime;

use kordoc_ir::{ErrorCode, PageMode};
use sha2::{Digest, Sha256};
use v8_runtime::metadata::{PdfMetadataMode, normalize_pdf_metadata};
use v8_runtime::text_document::PdfJsMetadata;

const MIXED_PDF: &[u8] = include_bytes!("fixtures/pdfjs_metadata/mixed.pdf");
const DELIMITERS_PDF: &[u8] = include_bytes!("fixtures/pdfjs_metadata/delimiters.pdf");
const DATES_PDF: &[u8] = include_bytes!("fixtures/pdfjs_metadata/dates.pdf");

#[derive(serde::Deserialize)]
struct CapturedProjection {
    input: String,
    input_sha256: String,
    mode: String,
    metadata: serde_json::Value,
}

fn expected_input(name: &str) -> &'static [u8] {
    match name {
        "mixed.pdf" => MIXED_PDF,
        "delimiters.pdf" => DELIMITERS_PDF,
        "dates.pdf" => DATES_PDF,
        other => panic!("unexpected captured input {other}"),
    }
}

#[test]
fn authored_pdfjs_metadata_matches_frozen_full_and_metadata_only_captures() {
    let capture_text = include_str!("fixtures/pdfjs_metadata/oracle-captures.jsonl");
    let captures = capture_text
        .lines()
        .map(serde_json::from_str::<CapturedProjection>)
        .collect::<Result<Vec<_>, _>>()
        .unwrap();

    for capture in captures {
        let input = expected_input(&capture.input);
        let digest = Sha256::digest(input)
            .iter()
            .map(|byte| format!("{byte:02x}"))
            .collect::<String>();
        assert_eq!(digest, capture.input_sha256);

        let document = v8_runtime::extract_text_document(input).unwrap();
        let mode = match capture.mode.as_str() {
            "full-parse" => PdfMetadataMode::FullParse,
            "metadata-only" => PdfMetadataMode::MetadataOnly,
            other => panic!("unexpected captured mode {other}"),
        };
        let actual = normalize_pdf_metadata(&document.metadata, document.page_count, mode).unwrap();
        assert_eq!(serde_json::to_value(actual).unwrap(), capture.metadata);
    }
}

#[test]
fn normalization_matches_ecmascript_trim_keywords_and_permissive_dates() {
    let raw = PdfJsMetadata {
        title: Some("\u{feff}  Title\u{00a0}".to_owned()),
        author: Some("\u{0085}Author\u{0085}".to_owned()),
        creator: Some("Creator".to_owned()),
        subject: Some(" Subject ".to_owned()),
        keywords: Some(" ; alpha ;β,alpha,, \u{feff}".to_owned()),
        creation_date: Some("prefix D:2025120X suffix".to_owned()),
        modified_date: Some("xxD:20251399+05'30'".to_owned()),
    };
    let actual = normalize_pdf_metadata(&raw, 7, PdfMetadataMode::FullParse).unwrap();
    let expected = serde_json::json!({
        "title": "Title",
        "author": "\u{0085}Author\u{0085}",
        "creator": "Creator",
        "description": "Subject",
        "keywords": ["alpha", "β", "alpha"],
        "createdAt": "2025-12-01T00:00:00",
        "modifiedAt": "2025-13-99T00:00:00",
        "pageCount": 7,
        "pageMode": "layout"
    });
    assert_eq!(serde_json::to_value(actual).unwrap(), expected);
}

#[test]
fn metadata_only_omits_page_mode_and_empty_keywords_remain_absent() {
    let raw = PdfJsMetadata {
        title: None,
        author: Some("\u{feff} \u{a0}".to_owned()),
        creator: None,
        subject: None,
        keywords: Some(" \t\n".to_owned()),
        creation_date: Some("no date".to_owned()),
        modified_date: None,
    };
    let actual = normalize_pdf_metadata(&raw, 1, PdfMetadataMode::MetadataOnly).unwrap();
    assert_eq!(
        serde_json::to_value(actual).unwrap(),
        serde_json::json!({ "pageCount": 1 })
    );
}

#[test]
fn metadata_input_caps_are_revalidated_and_fatal() {
    let mut raw = PdfJsMetadata {
        title: Some("x".repeat(4097)),
        author: None,
        creator: None,
        subject: None,
        keywords: None,
        creation_date: None,
        modified_date: None,
    };
    let error = normalize_pdf_metadata(&raw, 1, PdfMetadataMode::FullParse).unwrap_err();
    assert_eq!(error.code, ErrorCode::OutputTooLarge);

    raw.title = Some("한".repeat(1366));
    assert_eq!(
        normalize_pdf_metadata(&raw, 1, PdfMetadataMode::FullParse)
            .unwrap_err()
            .code,
        ErrorCode::OutputTooLarge
    );

    raw.title = None;
    raw.author = Some("a".repeat(4096));
    raw.creator = Some("b".repeat(4096));
    raw.subject = Some("c".repeat(4096));
    raw.keywords = Some("d".repeat(4096));
    assert!(normalize_pdf_metadata(&raw, 1, PdfMetadataMode::FullParse).is_ok());

    raw.creation_date = Some("e".to_owned());
    assert_eq!(
        normalize_pdf_metadata(&raw, 1, PdfMetadataMode::FullParse)
            .unwrap_err()
            .code,
        ErrorCode::OutputTooLarge
    );
}

#[test]
fn normalizer_returns_existing_metadata_shape() {
    let raw = PdfJsMetadata {
        title: Some("x".to_owned()),
        author: None,
        creator: None,
        subject: None,
        keywords: Some(";,".to_owned()),
        creation_date: None,
        modified_date: None,
    };
    let full = normalize_pdf_metadata(&raw, 2, PdfMetadataMode::FullParse).unwrap();
    assert_eq!(full.page_mode, Some(PageMode::Layout));
    assert_eq!(full.page_count, Some(2));
    assert_eq!(full.keywords, Some(Vec::new()));
    let only = normalize_pdf_metadata(&raw, 2, PdfMetadataMode::MetadataOnly).unwrap();
    assert_eq!(only.page_mode, None);
    assert_eq!(
        serde_json::to_value(full).unwrap(),
        serde_json::json!({
            "title": "x", "keywords": [], "pageCount": 2, "pageMode": "layout"
        })
    );
}
