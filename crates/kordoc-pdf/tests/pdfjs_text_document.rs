#![cfg(feature = "pdfjs-v8")]

#[allow(
    dead_code,
    reason = "this integration test reuses the private V8 runtime module"
)]
#[path = "../src/v8_runtime/mod.rs"]
mod v8_runtime;

use kordoc_ir::ErrorCode;
use v8_runtime::text_document::{PdfJsTextDocument, TextDocumentLimits, deserialize_text_document};
use v8_runtime::{extract_text_document, test_extract_text_document_with_limits};

const DOCUMENT_PDF: &[u8] = include_bytes!("fixtures/pdfjs_text_document/document.pdf");

#[test]
fn extracts_ordered_default_text_items_geometry_and_raw_info_metadata() {
    let document: PdfJsTextDocument = extract_text_document(DOCUMENT_PDF).unwrap();
    let actual = serde_json::to_value(&document).unwrap();
    let expected: serde_json::Value =
        serde_json::from_str(include_str!("fixtures/pdfjs_text_document/expected.json")).unwrap();
    assert_eq!(actual, expected);

    assert_eq!(document.page_count, 2);
    assert_eq!(document.pages.len(), 2);
    assert_eq!(document.pages[0].page_number, 1);
    assert_eq!(document.pages[0].view_box, [0.0, 0.0, 612.0, 792.0]);
    assert_eq!(document.pages[0].rotation, 0);
    assert_eq!(
        document.pages[0]
            .items
            .iter()
            .map(|item| item.text.as_str())
            .collect::<Vec<_>>(),
        ["second-stream", "first-stream", "continued line"]
    );
    assert_eq!(document.pages[0].items[0].transform[4..], [200.0, 700.0]);
    assert_eq!(document.pages[0].items[1].transform[4..], [50.0, 700.0]);
    assert_eq!(document.pages[1].page_number, 2);
    assert_eq!(document.pages[1].view_box, [100.0, 200.0, 500.0, 700.0]);
    assert_eq!(document.pages[1].rotation, 90);
    assert_eq!(document.pages[1].items[0].text, "한글🧪ffi");
    assert_eq!(
        document.pages[1].items[0].text.as_bytes(),
        "한글🧪ffi".as_bytes()
    );
    assert!(
        document
            .pages
            .iter()
            .flat_map(|page| &page.items)
            .all(|item| { !item.font_name.is_empty() && item.font_name.len() <= 128 })
    );

    assert_eq!(document.metadata.title.as_deref(), Some("  Raw title  "));
    assert_eq!(document.metadata.author.as_deref(), Some(" Author "));
    assert_eq!(document.metadata.creator.as_deref(), Some(""));
    assert_eq!(document.metadata.subject.as_deref(), Some(""));
    assert_eq!(
        document.metadata.keywords.as_deref(),
        Some("alpha, beta; gamma ")
    );
    assert_eq!(
        document.metadata.creation_date.as_deref(),
        Some("D:20250930123456Z")
    );
    assert_eq!(document.metadata.modified_date, None);
}

#[test]
fn metadata_limit_denial_is_fatal_not_a_best_effort_null_projection() {
    let limits = TextDocumentLimits {
        max_metadata_value_bytes: 0,
        max_metadata_bytes: 0,
        ..TextDocumentLimits::default()
    };
    let error = test_extract_text_document_with_limits(DOCUMENT_PDF, limits).unwrap_err();
    assert_eq!(error.code, ErrorCode::OutputTooLarge);
}

#[test]
fn malformed_pdf_content_with_quota_looking_text_stays_a_parse_error() {
    let corrupt = b"%PDF-1.7\n% PDFJS_LIMIT: page count exceeded\n%%EOF\n";
    let error = extract_text_document(corrupt).unwrap_err();
    assert_eq!(error.code, ErrorCode::ParseError);
}

#[test]
fn page_item_text_and_response_caps_fail_without_truncation() {
    for limits in [
        TextDocumentLimits {
            max_pages: 1,
            ..TextDocumentLimits::default()
        },
        TextDocumentLimits {
            max_items: 2,
            ..TextDocumentLimits::default()
        },
        TextDocumentLimits {
            max_item_text_bytes: 13,
            ..TextDocumentLimits::default()
        },
        TextDocumentLimits {
            max_font_name_bytes: 6,
            ..TextDocumentLimits::default()
        },
        TextDocumentLimits {
            max_response_bytes: 1,
            ..TextDocumentLimits::default()
        },
    ] {
        let error = test_extract_text_document_with_limits(DOCUMENT_PDF, limits).unwrap_err();
        assert_eq!(error.code, ErrorCode::OutputTooLarge);
    }
}

#[test]
fn serialized_response_limit_is_inclusive_at_exact_byte_length() {
    let output_bytes =
        v8_runtime::test_text_document_json_bytes(DOCUMENT_PDF, TextDocumentLimits::default())
            .unwrap();
    let inclusive = TextDocumentLimits {
        max_response_bytes: output_bytes,
        ..TextDocumentLimits::default()
    };
    assert!(test_extract_text_document_with_limits(DOCUMENT_PDF, inclusive).is_ok());

    let overflow = TextDocumentLimits {
        max_response_bytes: output_bytes - 1,
        ..TextDocumentLimits::default()
    };
    let error = test_extract_text_document_with_limits(DOCUMENT_PDF, overflow).unwrap_err();
    assert_eq!(error.code, ErrorCode::OutputTooLarge);
}

#[test]
fn page_item_and_string_caps_accept_the_inclusive_boundary() {
    let limits = TextDocumentLimits {
        max_pages: 2,
        max_items: 4,
        max_item_text_bytes: 14,
        max_text_bytes: 52,
        max_font_name_bytes: 7,
        max_font_names_bytes: 28,
        max_metadata_value_bytes: 19,
        max_metadata_bytes: 57,
        ..TextDocumentLimits::default()
    };
    assert!(test_extract_text_document_with_limits(DOCUMENT_PDF, limits).is_ok());
}

#[test]
fn hostile_parent_dto_page_sequence_is_bounded_during_deserialization() {
    let page = r#"{"page_number":1,"view_box":[0,0,1,1],"rotation":0,"items":[]}"#;
    let pages = std::iter::repeat_n(page, 201).collect::<Vec<_>>().join(",");
    let json = format!(
        r#"{{"page_count":201,"metadata":{{"title":null,"author":null,"creator":null,"subject":null,"keywords":null,"creation_date":null,"modified_date":null}},"pages":[{pages}]}}"#
    );
    let error =
        deserialize_text_document(json.as_bytes(), TextDocumentLimits::default()).unwrap_err();
    assert_eq!(error.code, ErrorCode::OutputTooLarge);
}

#[test]
fn hostile_parent_dto_item_sequence_rejects_n_plus_one_without_decoding_it() {
    let item = serde_json::json!({
        "text": "x", "width": 1, "height": 1,
        "transform": [1, 0, 0, 1, 0, 0], "font_name": "f"
    });
    let value = serde_json::json!({
        "page_count": 1,
        "metadata": {
            "title": null, "author": null, "creator": null, "subject": null,
            "keywords": null, "creation_date": null, "modified_date": null
        },
        "pages": [{
            "page_number": 1, "view_box": [0, 0, 1, 1], "rotation": 0,
            "items": [item.clone(), item.clone(), item]
        }]
    });
    let limits = TextDocumentLimits {
        max_items: 2,
        ..TextDocumentLimits::default()
    };
    let error =
        deserialize_text_document(&serde_json::to_vec(&value).unwrap(), limits).unwrap_err();
    assert_eq!(error.code, ErrorCode::OutputTooLarge);
}

#[test]
fn unknown_field_cannot_spoof_a_quota_error() {
    let json = br#"{"page_count":1,"metadata":{"title":null,"author":null,"creator":null,"subject":null,"keywords":null,"creation_date":null,"modified_date":null},"pages":[],"unknown PDFJS_LIMIT: field byte cap exceeded":true}"#;
    let error = deserialize_text_document(json, TextDocumentLimits::default()).unwrap_err();
    assert_eq!(error.code, ErrorCode::ParseError);
}

#[test]
fn malformed_page_count_and_control_character_font_are_parse_errors() {
    let mismatch = serde_json::json!({
        "page_count": 2,
        "metadata": {"title":null,"author":null,"creator":null,"subject":null,
            "keywords":null,"creation_date":null,"modified_date":null},
        "pages": []
    });
    assert_eq!(
        deserialize_text_document(
            &serde_json::to_vec(&mismatch).unwrap(),
            TextDocumentLimits::default()
        )
        .unwrap_err()
        .code,
        ErrorCode::ParseError
    );

    let invalid_font = serde_json::json!({
        "page_count": 1,
        "metadata": {"title":null,"author":null,"creator":null,"subject":null,
            "keywords":null,"creation_date":null,"modified_date":null},
        "pages": [{"page_number":1,"view_box":[0,0,1,1],"rotation":0,
            "items":[{"text":"x","width":1,"height":1,"transform":[1,0,0,1,0,0],"font_name":"bad\u{0001}name"}]}]
    });
    assert_eq!(
        deserialize_text_document(
            &serde_json::to_vec(&invalid_font).unwrap(),
            TextDocumentLimits::default()
        )
        .unwrap_err()
        .code,
        ErrorCode::ParseError
    );
}

#[test]
fn parent_decoder_rejects_invalid_geometry_shape_rotation_and_page_order() {
    let valid = serde_json::json!({
        "page_count": 1,
        "metadata": {"title":null,"author":null,"creator":null,"subject":null,
            "keywords":null,"creation_date":null,"modified_date":null},
        "pages": [{"page_number":1,"view_box":[0,0,1,1],"rotation":0,"items":[
            {"text":"x","width":1,"height":1,"transform":[1,0,0,1,0,0],"font_name":"f"}
        ]}]
    });
    let mut invalid = valid.clone();
    invalid["pages"][0]["page_number"] = serde_json::json!(2);
    assert_eq!(
        deserialize_text_document(
            &serde_json::to_vec(&invalid).unwrap(),
            TextDocumentLimits::default()
        )
        .unwrap_err()
        .code,
        ErrorCode::ParseError
    );
    let mut invalid = valid.clone();
    invalid["pages"][0]["rotation"] = serde_json::json!(45);
    assert_eq!(
        deserialize_text_document(
            &serde_json::to_vec(&invalid).unwrap(),
            TextDocumentLimits::default()
        )
        .unwrap_err()
        .code,
        ErrorCode::ParseError
    );
    let mut invalid = valid;
    invalid["pages"][0]["items"][0]["transform"] = serde_json::json!([1, 0, 0, 1, 0]);
    assert_eq!(
        deserialize_text_document(
            &serde_json::to_vec(&invalid).unwrap(),
            TextDocumentLimits::default()
        )
        .unwrap_err()
        .code,
        ErrorCode::ParseError
    );
    let invalid_finite = br#"{"page_count":1,"metadata":{"title":null,"author":null,"creator":null,"subject":null,"keywords":null,"creation_date":null,"modified_date":null},"pages":[{"page_number":1,"view_box":[1e999,0,1,1],"rotation":0,"items":[]}]}"#;
    assert_eq!(
        deserialize_text_document(invalid_finite, TextDocumentLimits::default())
            .unwrap_err()
            .code,
        ErrorCode::ParseError
    );
}

#[test]
fn parent_dto_rejects_oversized_individual_text_before_retaining_it() {
    let long_text = "x".repeat(64 * 1024 + 1);
    let json = serde_json::json!({
        "page_count": 1,
        "metadata": {
            "title": null, "author": null, "creator": null, "subject": null,
            "keywords": null, "creation_date": null, "modified_date": null
        },
        "pages": [{
            "page_number": 1, "view_box": [0, 0, 1, 1], "rotation": 0,
            "items": [{"text": long_text, "width": 1, "height": 1,
                "transform": [1, 0, 0, 1, 0, 0], "font_name": "f"}]
        }]
    });
    let bytes = serde_json::to_vec(&json).unwrap();
    let error = deserialize_text_document(&bytes, TextDocumentLimits::default()).unwrap_err();
    assert_eq!(error.code, ErrorCode::OutputTooLarge);
}

#[test]
fn parent_dto_enforces_utf8_byte_caps_for_non_ascii_strings() {
    let value = serde_json::json!({
        "page_count": 1,
        "metadata": {
            "title": null, "author": null, "creator": null, "subject": null,
            "keywords": null, "creation_date": null, "modified_date": null
        },
        "pages": [{
            "page_number": 1, "view_box": [0, 0, 1, 1], "rotation": 0,
            "items": [{"text": "한", "width": 1, "height": 1,
                "transform": [1, 0, 0, 1, 0, 0], "font_name": "f"}]
        }]
    });
    let limits = TextDocumentLimits {
        max_item_text_bytes: 2,
        max_text_bytes: 2,
        ..TextDocumentLimits::default()
    };
    let error =
        deserialize_text_document(&serde_json::to_vec(&value).unwrap(), limits).unwrap_err();
    assert_eq!(error.code, ErrorCode::OutputTooLarge);
}

#[test]
fn escaped_oversized_field_is_rejected_at_the_field_cap() {
    let quoted = "\"".repeat(64 * 1024 + 1);
    let value = serde_json::json!({
        "page_count": 1,
        "metadata": {
            "title": null, "author": null, "creator": null, "subject": null,
            "keywords": null, "creation_date": null, "modified_date": null
        },
        "pages": [{
            "page_number": 1, "view_box": [0, 0, 1, 1], "rotation": 0,
            "items": [{"text": quoted, "width": 1, "height": 1,
                "transform": [1, 0, 0, 1, 0, 0], "font_name": "f"}]
        }]
    });
    let bytes = serde_json::to_vec(&value).unwrap();
    assert!(bytes.len() < TextDocumentLimits::default().max_response_bytes);
    let error = deserialize_text_document(&bytes, TextDocumentLimits::default()).unwrap_err();
    assert_eq!(error.code, ErrorCode::OutputTooLarge);
}

#[test]
fn streamed_items_match_default_get_text_content_for_non_xfa_pages() {
    assert!(v8_runtime::test_stream_matches_default_get_text_content(DOCUMENT_PDF).unwrap());
}
