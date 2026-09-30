#![cfg(feature = "pdfjs-v8")]

#[allow(
    dead_code,
    reason = "the scalar projection uses only the shared JavaScript rounding helper"
)]
#[path = "../src/geometry.rs"]
mod geometry;
#[path = "../src/text_scalar.rs"]
mod text_scalar;
#[allow(
    dead_code,
    reason = "this integration reuses private PDF.js DTOs for scalar projection"
)]
#[path = "../src/v8_runtime/mod.rs"]
mod v8_runtime;

use kordoc_ir::ErrorCode;
use serde_json::Value;
use v8_runtime::text_document::PdfJsTextItem;

const SYNTHETIC: &str =
    include_str!("fixtures/pdfjs_text_normalization/oracle-synthetic-vectors.json");
const HYPOT: &str = include_str!("fixtures/pdfjs_text_normalization/math-hypot-vectors.json");
const RAW: &str = include_str!("fixtures/pdfjs_text_normalization/raw-worker-response.json");
const ACTUAL_ORACLE: &str =
    include_str!("fixtures/pdfjs_text_normalization/oracle-scalar-projection.json");

fn item(text: &str, transform: [f64; 6], width: f64, height: f64, font: &str) -> PdfJsTextItem {
    PdfJsTextItem {
        text: text.to_owned(),
        width,
        height,
        transform,
        font_name: font.to_owned(),
    }
}

#[test]
fn synthetic_trim_whitespace_gap_and_stable_order_match_capture() {
    let capture: Value = serde_json::from_str(SYNTHETIC).unwrap();
    let inputs = capture["cases"]["whitespace_gap_and_stable_tie"]["input"]
        .as_array()
        .unwrap();
    let items = inputs
        .iter()
        .map(|value| {
            item(
                value["str"].as_str().unwrap(),
                serde_json::from_value(value["transform"].clone()).unwrap(),
                value["width"].as_f64().unwrap(),
                value["height"].as_f64().unwrap(),
                value["fontName"].as_str().unwrap(),
            )
        })
        .collect();
    let actual = text_scalar::project_base_items(items).unwrap();
    let expected = &capture["cases"]["whitespace_gap_and_stable_tie"]["output"];
    assert_eq!(actual.len(), expected.as_array().unwrap().len());
    for (actual, expected) in actual.iter().zip(expected.as_array().unwrap()) {
        assert_eq!(actual.text, expected["text"]);
        assert_eq!(actual.x, expected["x"].as_f64().unwrap());
        assert_eq!(actual.y, expected["y"].as_f64().unwrap());
        assert_eq!(actual.width, expected["w"].as_f64().unwrap());
        assert_eq!(actual.height, expected["h"].as_f64().unwrap());
        assert_eq!(actual.font_size, expected["fontSize"].as_f64().unwrap());
        assert_eq!(actual.font_name, expected["fontName"]);
        assert_eq!(actual.is_hidden, expected["isHidden"]);
        assert_eq!(actual.sequence, expected["seq"].as_u64().unwrap() as usize);
        assert_eq!(actual.rotated_length, expected["rotated"].as_f64());
    }
    assert_eq!(
        actual.iter().map(|item| item.sequence).collect::<Vec<_>>(),
        [1, 3, 4]
    );
    assert_eq!(actual[0].text, "before");
    assert_eq!(actual[1].text, "after");
    assert!(actual[2].text.starts_with('\u{85}'));
}

#[test]
fn v8_hypot_fast_path_matches_pinned_bit_vectors() {
    let vectors: Value = serde_json::from_str(HYPOT).unwrap();
    let cases = vectors["vectors"].as_array().unwrap();
    for (index, case) in cases.iter().enumerate() {
        let a = f64::from_bits(u64::from_str_radix(case["a_bits"].as_str().unwrap(), 16).unwrap());
        let b = f64::from_bits(u64::from_str_radix(case["b_bits"].as_str().unwrap(), 16).unwrap());
        let actual = text_scalar::js_math_hypot(a, b).to_bits();
        let expected = u64::from_str_radix(case["js_hypot_bits"].as_str().unwrap(), 16).unwrap();
        assert_eq!(actual, expected, "hypot vector {index}: {case}");
    }
}

#[test]
fn authored_pdf_raw_worker_items_match_pinned_oracle_projection() {
    let raw: Value = serde_json::from_str(RAW).unwrap();
    let input = raw["result"]["pages"][0]["items"].as_array().unwrap();
    let items = input
        .iter()
        .map(|value| {
            item(
                value["text"].as_str().unwrap(),
                serde_json::from_value(value["transform"].clone()).unwrap(),
                value["width"].as_f64().unwrap(),
                value["height"].as_f64().unwrap(),
                value["font_name"].as_str().unwrap(),
            )
        })
        .collect();
    let actual = text_scalar::project_base_items(items).unwrap();
    let expected: Value = serde_json::from_str(ACTUAL_ORACLE).unwrap();
    let expected_items = expected["items"].as_array().unwrap();
    assert_eq!(actual.len(), expected_items.len());
    for (actual, expected) in actual.iter().zip(expected_items) {
        assert_eq!(actual.text, expected["text"]);
        assert_eq!(actual.x, expected["x"].as_f64().unwrap());
        assert_eq!(actual.y, expected["y"].as_f64().unwrap());
        assert_eq!(actual.width, expected["w"].as_f64().unwrap());
        assert_eq!(actual.height, expected["h"].as_f64().unwrap());
        assert_eq!(actual.font_size, expected["fontSize"].as_f64().unwrap());
        assert_eq!(actual.font_name, expected["fontName"]);
        assert_eq!(actual.is_hidden, expected["isHidden"]);
        assert_eq!(actual.sequence, expected["seq"].as_u64().unwrap() as usize);
        assert_eq!(actual.rotated_length, expected["rotated"].as_f64());
    }
}

#[test]
fn constructed_dto_caps_and_nonfinite_values_fail_before_projection() {
    let oversized = "x".repeat(64 * 1024 + 1);
    let error = text_scalar::project_base_items(vec![item(
        &oversized,
        [12.0, 0.0, 0.0, 12.0, 1.0, 2.0],
        1.0,
        1.0,
        "f",
    )])
    .unwrap_err();
    assert_eq!(error.code, ErrorCode::OutputTooLarge);

    let too_long_font = item(
        "x",
        [1.0, 0.0, 0.0, 1.0, 0.0, 0.0],
        1.0,
        1.0,
        &"f".repeat(129),
    );
    assert_eq!(
        text_scalar::project_base_items(vec![too_long_font])
            .unwrap_err()
            .code,
        ErrorCode::OutputTooLarge
    );

    let error = text_scalar::project_base_items(vec![item(
        "bad",
        [f64::NAN, 0.0, 0.0, 12.0, 1.0, 2.0],
        1.0,
        1.0,
        "f",
    )])
    .unwrap_err();
    assert_eq!(error.code, ErrorCode::ParseError);

    let finite_but_unrepresentable_scale = item(
        "overflow",
        [f64::MAX, f64::MAX, 0.0, 1.0, 0.0, 0.0],
        1.0,
        1.0,
        "f",
    );
    assert_eq!(
        text_scalar::project_base_items(vec![finite_but_unrepresentable_scale])
            .unwrap_err()
            .code,
        ErrorCode::ParseError
    );
}

#[test]
fn signed_zero_coordinates_are_equal_sort_keys_and_keep_sequence_order() {
    let projected = text_scalar::project_base_items(vec![
        item(
            "negative zero",
            [1.0, 0.0, 0.0, 1.0, -0.1, -0.1],
            1.0,
            1.0,
            "a",
        ),
        item(
            "positive zero",
            [1.0, 0.0, 0.0, 1.0, 0.1, 0.1],
            1.0,
            1.0,
            "b",
        ),
    ])
    .unwrap();
    assert!(projected[0].x.is_sign_negative());
    assert!(projected[0].y.is_sign_negative());
    assert_eq!(projected[0].sequence, 1);
    assert_eq!(projected[1].sequence, 2);
}

#[test]
fn scalar_metrics_cover_vertical_signs_both_hidden_rules_and_derived_overflow() {
    let projected = text_scalar::project_base_items(vec![
        item(
            "positive",
            [0.0, 10.0, 0.0, 10.0, 50.0, 30.0],
            22.0,
            8.0,
            "p",
        ),
        item(
            "negative",
            [0.0, -10.0, 0.0, 10.0, 70.0, 20.0],
            22.0,
            8.0,
            "n",
        ),
        item("font-hidden", [0.0; 6], 1.0, 1.0, "z"),
        item(
            "width-hidden",
            [10.0, 0.0, 0.0, 10.0, 4.0, 10.0],
            0.0,
            10.0,
            "w",
        ),
    ])
    .unwrap();
    let positive = projected
        .iter()
        .find(|item| item.text == "positive")
        .unwrap();
    assert_eq!((positive.x, positive.y), (40.0, 30.0));
    assert_eq!(
        (positive.width, positive.rotated_length),
        (10.0, Some(22.0))
    );
    let negative = projected
        .iter()
        .find(|item| item.text == "negative")
        .unwrap();
    assert_eq!((negative.x, negative.y), (70.0, 20.0));
    assert_eq!(
        (negative.width, negative.rotated_length),
        (10.0, Some(22.0))
    );
    assert!(
        projected
            .iter()
            .find(|item| item.text == "font-hidden")
            .unwrap()
            .is_hidden
    );
    assert!(
        projected
            .iter()
            .find(|item| item.text == "width-hidden")
            .unwrap()
            .is_hidden
    );

    let overflow = item(
        "overflow",
        [0.0, f64::MAX, 0.0, f64::MAX, -f64::MAX, 1.0],
        1.0,
        1.0,
        "f",
    );
    assert_eq!(
        text_scalar::project_base_items(vec![overflow])
            .unwrap_err()
            .code,
        ErrorCode::ParseError
    );
}

#[test]
fn item_count_limit_rejects_the_first_excess_item() {
    let at_limit = (0..100_000)
        .map(|_| item("", [0.0; 6], 0.0, 0.0, ""))
        .collect();
    assert!(
        text_scalar::project_base_items(at_limit)
            .unwrap()
            .is_empty()
    );
    let over_limit = (0..100_001)
        .map(|_| item("", [0.0; 6], 0.0, 0.0, ""))
        .collect();
    assert_eq!(
        text_scalar::project_base_items(over_limit)
            .unwrap_err()
            .code,
        ErrorCode::OutputTooLarge
    );
}

#[test]
fn aggregate_caps_are_inclusive_and_reject_the_next_byte() {
    let text_at_limit = (0..32)
        .map(|_| {
            item(
                "x".repeat(64 * 1024).as_str(),
                [1.0, 0.0, 0.0, 1.0, 0.0, 0.0],
                1.0,
                1.0,
                "",
            )
        })
        .collect();
    assert_eq!(
        text_scalar::project_base_items(text_at_limit)
            .unwrap()
            .len(),
        32
    );

    let mut text_over_limit: Vec<_> = (0..32)
        .map(|_| {
            item(
                "x".repeat(64 * 1024).as_str(),
                [1.0, 0.0, 0.0, 1.0, 0.0, 0.0],
                1.0,
                1.0,
                "",
            )
        })
        .collect();
    text_over_limit.push(item("x", [1.0, 0.0, 0.0, 1.0, 0.0, 0.0], 1.0, 1.0, ""));
    assert_eq!(
        text_scalar::project_base_items(text_over_limit)
            .unwrap_err()
            .code,
        ErrorCode::OutputTooLarge
    );

    let font_at_limit = (0..4096)
        .map(|_| {
            item(
                "f",
                [1.0, 0.0, 0.0, 1.0, 0.0, 0.0],
                1.0,
                1.0,
                &"a".repeat(128),
            )
        })
        .collect();
    assert_eq!(
        text_scalar::project_base_items(font_at_limit)
            .unwrap()
            .len(),
        4096
    );
    let mut font_over_limit: Vec<_> = (0..4096)
        .map(|_| {
            item(
                "f",
                [1.0, 0.0, 0.0, 1.0, 0.0, 0.0],
                1.0,
                1.0,
                &"a".repeat(128),
            )
        })
        .collect();
    font_over_limit.push(item("f", [1.0, 0.0, 0.0, 1.0, 0.0, 0.0], 1.0, 1.0, "a"));
    assert_eq!(
        text_scalar::project_base_items(font_over_limit)
            .unwrap_err()
            .code,
        ErrorCode::OutputTooLarge
    );
}
