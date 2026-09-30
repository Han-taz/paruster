#![cfg(feature = "pdfjs-v8")]

#[path = "../src/geometry.rs"]
mod geometry;
#[allow(
    dead_code,
    reason = "this integration reuses the private V8 runtime DTO for geometry checks"
)]
#[path = "../src/v8_runtime/mod.rs"]
mod v8_runtime;

use geometry::{base_text_position, page_frame};
use kordoc_ir::ErrorCode;
use serde_json::Value;
use sha2::{Digest, Sha256};
use v8_runtime::text_document::{PdfJsPage, PdfJsTextDocument, PdfJsTextItem};

const PDF: &[u8] = include_bytes!("fixtures/pdfjs_geometry/fractional_cropbox.pdf");
const WORKER: &str = include_str!("fixtures/pdfjs_geometry/worker-response.json");
const ORACLE: &str = include_str!("fixtures/pdfjs_geometry/oracle-public-output.json");
const ROUNDING: &str = include_str!("fixtures/pdfjs_geometry/rounding-vectors.json");

fn sha256(bytes: &[u8]) -> String {
    Sha256::digest(bytes)
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

fn page(view_box: [f64; 4], rotation: i32) -> PdfJsPage {
    page_with_number(view_box, rotation, 1)
}

fn page_with_number(view_box: [f64; 4], rotation: i32, page_number: u32) -> PdfJsPage {
    PdfJsPage {
        page_number,
        view_box,
        rotation,
        items: vec![],
    }
}

fn item(e: f64, f: f64) -> PdfJsTextItem {
    PdfJsTextItem {
        text: "probe".to_owned(),
        width: 10.0,
        height: 10.0,
        transform: [10.0, 0.0, 0.0, 10.0, e, f],
        font_name: "font".to_owned(),
    }
}

#[test]
fn fractional_cropbox_matches_pinned_pdfjs_and_oracle_captures() {
    assert_eq!(
        sha256(PDF),
        "d3d9d26a445c99234439c112a5822d8bc34f2b6e0b88ac9c78a7520ce14b253e"
    );
    assert_eq!(
        sha256(WORKER.as_bytes()),
        "459107dc4d729bae5504ca298455a5e9207016b137562700b8406afbc9629aa9"
    );
    assert_eq!(
        sha256(ORACLE.as_bytes()),
        "579336a388156c85af42b9762fb5cd6c39f8275404b35ef0bfc7491b6746f883"
    );

    let captured: Value = serde_json::from_str(WORKER).unwrap();
    let document: PdfJsTextDocument =
        v8_runtime::extract_text_document(PDF).expect("actual pinned PDF.js V8 extraction");
    assert_eq!(serde_json::to_value(document).unwrap(), captured["result"]);

    let oracle: Value = serde_json::from_str(ORACLE).unwrap();
    assert_eq!(oracle["markdown"], "positive tie");
    assert_eq!(oracle["blocks"][0]["bbox"]["x"], 1.75);
    assert_eq!(oracle["blocks"][0]["bbox"]["y"], 1.25);
    assert_eq!(oracle["blocks"][0]["bbox"]["width"], 47.0);
    assert_eq!(oracle["blocks"][0]["bbox"]["height"], 10.0);
}

#[test]
fn page_frame_and_text_position_use_unrotated_cropbox_and_round_before_shift() {
    let document: PdfJsTextDocument =
        v8_runtime::extract_text_document(PDF).expect("actual PDF.js DTO");
    let page = &document.pages[0];
    let frame = page_frame(page).expect("valid cropbox frame");
    assert_eq!(frame.page_number, 1);
    assert_eq!(
        (frame.width, frame.height, frame.rotation),
        (100.0, 200.0, 90)
    );
    let position = base_text_position(&frame, &page.items[0]).expect("horizontal text position");
    assert_eq!((position.x, position.y), (1.75, 1.25));
}

#[test]
fn js_math_round_matches_pinned_v8_vectors_including_negative_zero() {
    let vectors: Value = serde_json::from_str(ROUNDING).unwrap();
    for vector in vectors["vectors"].as_array().unwrap() {
        let input: f64 = vector["input"].as_str().unwrap().parse().unwrap();
        let expected_bits =
            u64::from_str_radix(vector["rounded_bits"].as_str().unwrap(), 16).unwrap();
        let frame = page([0.0, 0.0, 10.0, 10.0], 0);
        let frame = page_frame(&frame).unwrap();
        let result = base_text_position(&frame, &item(input, input)).unwrap();
        assert_eq!(
            result.x.to_bits(),
            expected_bits,
            "input {:?}",
            vector["input"]
        );
        assert_eq!(
            result.y.to_bits(),
            expected_bits,
            "input {:?}",
            vector["input"]
        );
        assert_eq!(
            result.x.is_sign_negative() && result.x == 0.0,
            vector["is_negative_zero"].as_bool().unwrap()
        );
    }
}

#[test]
fn malformed_page_and_item_geometry_returns_parse_error() {
    for view_box in [
        [0.0, 0.0, 0.0, 1.0],
        [0.0, 0.0, 1.0, 0.0],
        [1.0, 0.0, 0.0, 1.0],
        [0.0, 1.0, 1.0, 0.0],
        [f64::NAN, 0.0, 1.0, 1.0],
        [0.0, 0.0, f64::INFINITY, 1.0],
        [f64::NEG_INFINITY, 0.0, f64::MAX, 1.0],
    ] {
        assert_eq!(
            page_frame(&page(view_box, 0)).unwrap_err().code,
            ErrorCode::ParseError
        );
    }
    for rotation in [-45, 1, 45, 91] {
        assert_eq!(
            page_frame(&page([0.0, 0.0, 1.0, 1.0], rotation))
                .unwrap_err()
                .code,
            ErrorCode::ParseError
        );
    }
    assert_eq!(
        page_frame(&page_with_number([0.0, 0.0, 1.0, 1.0], 0, 0))
            .unwrap_err()
            .code,
        ErrorCode::ParseError
    );

    let frame = page_frame(&page([0.0, 0.0, 10.0, 10.0], 0)).unwrap();
    for bad in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
        for index in 0..6 {
            let mut candidate = item(1.0, 1.0);
            candidate.transform[index] = bad;
            assert_eq!(
                base_text_position(&frame, &candidate).unwrap_err().code,
                ErrorCode::ParseError,
                "transform index {index}"
            );
        }
    }
    let huge_origin = page_frame(&page([-f64::MAX, 0.0, f64::MAX, 10.0], 0));
    assert_eq!(huge_origin.unwrap_err().code, ErrorCode::ParseError);

    let finite_wide_frame = page_frame(&page([-f64::MAX, 0.0, 0.0, 10.0], 0)).unwrap();
    assert_eq!(
        base_text_position(&finite_wide_frame, &item(f64::MAX, 1.0))
            .unwrap_err()
            .code,
        ErrorCode::ParseError
    );
}
