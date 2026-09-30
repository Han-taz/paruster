#![cfg(feature = "pdfjs-v8")]

#[path = "../src/v8_runtime/mod.rs"]
mod v8_runtime;

use kordoc_ir::{ErrorCode, KordocError};
use v8_runtime::{PdfJsProbe, probe_pdf_text, probe_with_limits};

const PROBE_PDF: &[u8] = include_bytes!("fixtures/pdfjs_probe/one_page_helvetica.pdf");

#[test]
fn extracts_text_from_the_checked_in_one_page_probe() {
    let result: PdfJsProbe = probe_pdf_text(PROBE_PDF).unwrap();
    assert_eq!(result.page_count, 1);
    assert_eq!(result.page_text, ["V8 PDF.js probe"]);
}

#[test]
fn malformed_input_is_a_typed_parse_failure() {
    let error: KordocError = probe_pdf_text(b"not a PDF").unwrap_err();
    assert!(matches!(
        error.code,
        ErrorCode::ParseError | ErrorCode::Corrupted
    ));
    let error: KordocError = probe_pdf_text(b"%PDF-1.7\nnot a valid PDF body").unwrap_err();
    assert!(matches!(
        error.code,
        ErrorCode::ParseError | ErrorCode::Corrupted
    ));
}

#[test]
fn runtime_does_not_publish_host_io_globals() {
    assert!(v8_runtime::test_host_has_no_io_globals());
}

#[test]
fn output_limit_is_inclusive_and_overflow_is_typed() {
    let result = probe_pdf_text(PROBE_PDF).unwrap();
    let output_bytes = serde_json::to_vec(&result).unwrap().len();
    assert_eq!(
        probe_with_limits(PROBE_PDF, 200, output_bytes).unwrap(),
        result
    );
    let error = probe_with_limits(PROBE_PDF, 200, output_bytes - 1).unwrap_err();
    assert_eq!(error.code, ErrorCode::OutputTooLarge);
}

#[test]
fn execution_deadline_terminates_nonterminating_script() {
    let error = v8_runtime::test_deadline_termination().unwrap_err();
    assert_eq!(error.code, ErrorCode::ParseError);
}

#[test]
fn external_array_buffer_limit_is_inclusive_and_released() {
    let error = v8_runtime::test_external_buffer_limit().unwrap_err();
    assert_eq!(error.code, ErrorCode::OutputTooLarge);
}

#[test]
fn embedded_v8_engine_reports_its_version() {
    let version = v8_runtime::test_v8_engine_version();
    assert_eq!(version, "15.2.124.1-rusty");
    eprintln!("embedded V8 engine version: {version}");
}
