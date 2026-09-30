#![cfg(feature = "pdfjs-v8")]

#[allow(
    dead_code,
    reason = "the text rewrite integration only reuses the shared Math.round helper"
)]
#[path = "../src/geometry.rs"]
mod geometry;
#[path = "../src/text_rewrites.rs"]
mod text_rewrites;
#[allow(
    dead_code,
    reason = "this integration only needs ECMAScript trim and PDF.js scalar helpers"
)]
#[path = "../src/v8_runtime/mod.rs"]
mod v8_runtime;

use kordoc_ir::ErrorCode;
use serde_json::Value;

const INPUTS: &str = include_str!("fixtures/pdf_text_rewrites/rewrite-inputs.json");
const REWRITES: &str = include_str!("fixtures/pdf_text_rewrites/oracle-rewrite-vectors.json");
const RADICALS: &str = include_str!("fixtures/pdf_text_rewrites/radical-inputs.json");
const RADICAL_MAPPINGS: &str =
    include_str!("fixtures/pdf_text_rewrites/oracle-radical-mappings.json");

#[test]
fn non_split_rewrites_match_pinned_oracle_text_fields() {
    let inputs: Value = serde_json::from_str(INPUTS).unwrap();
    let captures: Value = serde_json::from_str(REWRITES).unwrap();
    let cases = captures["cases"].as_array().unwrap();
    assert_eq!(cases.len(), 20);
    assert_eq!(
        captures["split_order_evidence"].as_array().unwrap().len(),
        4
    );
    for case in cases {
        let name = case["name"].as_str().unwrap();
        let input = inputs["cases"]
            .as_array()
            .unwrap()
            .iter()
            .find(|value| value["name"] == name)
            .unwrap();
        let expected = case["output_text"].as_array().unwrap();
        let items = input["items"].as_array().unwrap();
        assert_eq!(items.len(), expected.len(), "{name}");
        let actual: Vec<_> = items
            .iter()
            .map(|item| {
                let text = item["str"].as_str().unwrap();
                let size = item["transform"][0].as_f64().unwrap();
                let rounded_size = geometry::js_math_round(size);
                text_rewrites::rewrite_text(text, rounded_size).unwrap()
            })
            .collect();
        let expected: Vec<_> = expected.iter().map(|text| text.as_str().unwrap()).collect();
        assert_eq!(actual, expected, "{name}");
    }
}

#[test]
fn all_kangxi_radical_nfkc_mappings_match_node_v8_capture() {
    let inputs: Value = serde_json::from_str(RADICALS).unwrap();
    let capture: Value = serde_json::from_str(RADICAL_MAPPINGS).unwrap();
    let inputs = inputs["items"].as_array().unwrap();
    let mappings = capture["mappings"].as_array().unwrap();
    assert_eq!(inputs.len(), 214);
    assert_eq!(mappings.len(), inputs.len());
    for (index, (input, mapping)) in inputs.iter().zip(mappings).enumerate() {
        let expected_code_point = format!("U+{:04X}", 0x2f00 + index);
        let expected_scalar = char::from_u32(0x2f00 + index as u32).unwrap().to_string();
        assert_eq!(input["code_point"], expected_code_point);
        assert_eq!(input["text"], expected_scalar);
        assert_eq!(mapping["code_point"], expected_code_point);
        assert_eq!(input["code_point"], mapping["code_point"]);
        assert_eq!(input["text"], mapping["input"]);
        let actual = text_rewrites::rewrite_text(input["text"].as_str().unwrap(), 12.0).unwrap();
        assert_eq!(actual, mapping["output"], "{}", mapping["code_point"]);
    }
}

#[test]
fn single_item_input_limit_is_inclusive_and_rejects_plus_one() {
    let at_limit = "a".repeat(64 * 1024);
    assert_eq!(
        text_rewrites::rewrite_text(&at_limit, 12.0).unwrap().len(),
        64 * 1024
    );
    let over_limit = "a".repeat(64 * 1024 + 1);
    assert_eq!(
        text_rewrites::rewrite_text(&over_limit, 12.0)
            .unwrap_err()
            .code,
        ErrorCode::OutputTooLarge
    );
}

#[test]
fn rounded_font_size_must_be_finite_nonnegative_and_integral() {
    for invalid in [f64::NAN, f64::INFINITY, -1.0, 13.5] {
        assert_eq!(
            text_rewrites::rewrite_text("H O W", invalid)
                .unwrap_err()
                .code,
            ErrorCode::ParseError,
            "font size {invalid}"
        );
    }
    assert_eq!(text_rewrites::rewrite_text("H O W", 13.0).unwrap(), "H O W");
    assert_eq!(text_rewrites::rewrite_text("H O W", 14.0).unwrap(), "HOW");
}
