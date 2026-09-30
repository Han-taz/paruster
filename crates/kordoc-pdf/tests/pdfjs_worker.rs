#![cfg(feature = "pdfjs-v8")]

#[path = "../src/v8_runtime/mod.rs"]
#[allow(
    dead_code,
    reason = "this integration compares the in-process probe but does not repeat its guard tests"
)]
mod v8_runtime;
#[path = "../src/worker_protocol.rs"]
mod worker_protocol;
#[path = "../src/worker_supervisor.rs"]
#[allow(
    dead_code,
    reason = "this real-worker integration does not call the test-helper-only supervisor entry point"
)]
mod worker_supervisor;

use std::io::{Read, Write};
use std::path::Path;
use std::process::{Command, Stdio};
use std::time::Duration;

use kordoc_ir::ErrorCode;

const PROBE_PDF: &[u8] = include_bytes!("fixtures/pdfjs_probe/one_page_helvetica.pdf");
const UNICODE_PDF: &[u8] = include_bytes!("fixtures/unicode_probe/unicode_probe.pdf");
const TEXT_DOCUMENT_PDF: &[u8] = include_bytes!("fixtures/pdfjs_text_document/document.pdf");

fn run_real_worker(bytes: &[u8]) -> Result<v8_runtime::PdfJsProbe, kordoc_ir::KordocError> {
    let mut child = Command::new(env!("CARGO_BIN_EXE_pdfjs-worker"))
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .expect("spawn Cargo-built private PDF.js worker");
    {
        let mut stdin = child.stdin.take().expect("piped stdin");
        worker_protocol::write_request(&mut stdin, bytes).expect("write bounded request");
        stdin.flush().expect("flush request");
    }
    let mut stdout = child.stdout.take().expect("piped stdout");
    let response = worker_protocol::read_response(&mut stdout).expect("one exact response frame");
    let status = child.wait().expect("reap PDF.js worker");
    assert!(status.success(), "worker exited with {status}");
    let mut trailing = Vec::new();
    stdout
        .read_to_end(&mut trailing)
        .expect("read trailing stdout");
    assert!(trailing.is_empty(), "worker wrote extra stdout bytes");
    response
}

fn run_raw_worker_frame(frame: &[u8]) -> Result<v8_runtime::PdfJsProbe, kordoc_ir::KordocError> {
    let mut child = Command::new(env!("CARGO_BIN_EXE_pdfjs-worker"))
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .expect("spawn Cargo-built private PDF.js worker");
    {
        let mut stdin = child.stdin.take().expect("piped stdin");
        stdin.write_all(frame).expect("write raw request frame");
    }
    let response = worker_protocol::read_response(&mut child.stdout.take().expect("piped stdout"))
        .expect("one exact response frame");
    assert!(child.wait().expect("reap PDF.js worker").success());
    response
}

fn run_raw_text_worker_frame(
    frame: &[u8],
) -> Result<v8_runtime::text_document::PdfJsTextDocument, kordoc_ir::KordocError> {
    let mut child = Command::new(env!("CARGO_BIN_EXE_pdfjs-worker"))
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .expect("spawn Cargo-built PDF.js worker");
    child
        .stdin
        .take()
        .expect("piped stdin")
        .write_all(frame)
        .expect("write raw request frame");
    let response =
        worker_protocol::read_text_response(&mut child.stdout.take().expect("piped stdout"))
            .expect("one exact text response frame");
    assert!(child.wait().expect("reap PDF.js worker").success());
    response
}

#[test]
fn cargo_built_worker_routes_kind_three_to_rich_kind_four() {
    let expected = v8_runtime::extract_text_document(TEXT_DOCUMENT_PDF).expect("in-process DTO");
    let mut frame = Vec::new();
    worker_protocol::write_text_request(&mut frame, TEXT_DOCUMENT_PDF).unwrap();
    let actual = run_raw_text_worker_frame(&frame).expect("rich worker document");
    assert_eq!(actual, expected);
    assert_eq!(actual.page_count, 2);
    assert!(
        actual
            .pages
            .iter()
            .flat_map(|page| &page.items)
            .any(|item| item.text.contains("한글"))
    );
    let supervised = worker_supervisor::supervise_text_document_worker(
        Path::new(env!("CARGO_BIN_EXE_pdfjs-worker")),
        TEXT_DOCUMENT_PDF,
        Duration::from_secs(15),
    )
    .expect("supervised rich worker document");
    assert_eq!(supervised, expected);
}

#[test]
fn rich_worker_accepts_existing_valid_helvetica_probe_pdf() {
    let expected = v8_runtime::extract_text_document(PROBE_PDF).expect("valid Helvetica DTO");
    assert_eq!(expected.page_count, 1);
    assert!(
        expected.pages[0]
            .items
            .iter()
            .any(|item| item.text == "V8 PDF.js probe")
    );
    let actual = worker_supervisor::supervise_text_document_worker(
        Path::new(env!("CARGO_BIN_EXE_pdfjs-worker")),
        PROBE_PDF,
        Duration::from_secs(15),
    )
    .expect("valid Helvetica rich worker result");
    assert_eq!(actual, expected);
}

#[test]
fn recognized_text_kind_framing_errors_stay_on_kind_four() {
    let six_byte_prefix = b"KPDF\x01\x03";
    let error = run_raw_text_worker_frame(six_byte_prefix).unwrap_err();
    assert_eq!(error.code, ErrorCode::ParseError);

    let truncated = b"KPDF\x01\x03\x00\x00\x00\x02x";
    let error = run_raw_text_worker_frame(truncated).unwrap_err();
    assert_eq!(error.code, ErrorCode::ParseError);

    let trailing = b"KPDF\x01\x03\x00\x00\x00\x00x";
    let error = run_raw_text_worker_frame(trailing).unwrap_err();
    assert_eq!(error.code, ErrorCode::ParseError);

    let oversized = [
        b"KPDF\x01\x03".as_slice(),
        &((worker_protocol::MAX_REQUEST_BYTES as u32 + 1).to_be_bytes()),
    ]
    .concat();
    let error = run_raw_text_worker_frame(&oversized).unwrap_err();
    assert_eq!(error.code, ErrorCode::OutputTooLarge);
}

#[test]
fn unknown_kind_preserves_legacy_kind_two_error_frame() {
    let unknown = b"KPDF\x01\x09\x00\x00\x00\x00";
    let error = run_raw_worker_frame(unknown).unwrap_err();
    assert_eq!(error.code, ErrorCode::ParseError);

    let short_prefix = b"KPDF\x01";
    let error = run_raw_worker_frame(short_prefix).unwrap_err();
    assert_eq!(error.code, ErrorCode::ParseError);
}

#[test]
fn cargo_built_worker_matches_the_in_process_cc0_probe() {
    let expected = v8_runtime::probe_pdf_text(PROBE_PDF).expect("in-process probe");
    let actual = worker_supervisor::supervise_worker(
        Path::new(env!("CARGO_BIN_EXE_pdfjs-worker")),
        PROBE_PDF,
        Duration::from_secs(15),
    )
    .expect("supervised worker probe");
    assert_eq!(actual, expected);
    assert_eq!(actual.page_count, 1);
    assert_eq!(actual.page_text, ["V8 PDF.js probe"]);
}

#[test]
fn supervised_worker_preserves_tounicode_hangul_and_astral_text() {
    let expected = v8_runtime::probe_pdf_text(UNICODE_PDF).expect("in-process Unicode probe");
    let actual = worker_supervisor::supervise_worker(
        Path::new(env!("CARGO_BIN_EXE_pdfjs-worker")),
        UNICODE_PDF,
        Duration::from_secs(15),
    )
    .expect("supervised Unicode probe");
    assert_eq!(actual, expected);
    assert_eq!(actual.page_count, 1);
    assert_eq!(actual.page_text, ["한글🧪"]);
}

#[test]
fn malformed_pdf_returns_a_typed_frame_and_clean_exit() {
    let error = run_real_worker(b"%PDF-1.7\nnot a valid PDF body").unwrap_err();
    assert!(matches!(
        error.code,
        ErrorCode::ParseError | ErrorCode::Corrupted
    ));
    assert!(!error.message.is_empty());
}

#[test]
fn malformed_or_oversized_request_frame_returns_a_bounded_typed_error() {
    let invalid_magic = b"NOPE\x01\x01\x00\x00\x00\x00";
    let malformed = run_raw_worker_frame(invalid_magic).unwrap_err();
    assert_eq!(malformed.code, ErrorCode::ParseError);

    let oversized_header = [
        b"KPDF\x01\x01".as_slice(),
        &((worker_protocol::MAX_REQUEST_BYTES as u32 + 1).to_be_bytes()),
    ]
    .concat();
    let oversized = run_raw_worker_frame(&oversized_header).unwrap_err();
    assert_eq!(oversized.code, ErrorCode::OutputTooLarge);
}
