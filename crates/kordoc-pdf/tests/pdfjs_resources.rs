#![cfg(feature = "pdfjs-v8")]

#[allow(
    dead_code,
    reason = "this integration test reuses the private V8 runtime module"
)]
#[path = "../src/v8_runtime/mod.rs"]
mod v8_runtime;
#[path = "../src/worker_protocol.rs"]
#[allow(
    dead_code,
    reason = "the real-worker integration uses only the supervisor protocol path"
)]
mod worker_protocol;
#[path = "../src/worker_supervisor.rs"]
#[allow(
    dead_code,
    reason = "this resource integration uses only the supervised probe entry point"
)]
mod worker_supervisor;

use std::path::Path;
use std::time::Duration;

use kordoc_ir::ErrorCode;
use v8_runtime::{
    PdfJsProbe, probe_pdf_text, test_probe_with_resource_limits, test_probe_with_resource_stats,
};

const RESOURCE_PDF: &[u8] = include_bytes!("fixtures/pdfjs_resource_probe/resource_probe.pdf");
const HELVETTE_PDF: &[u8] = include_bytes!("fixtures/pdfjs_probe/one_page_helvetica.pdf");

#[test]
fn cjk_probe_loads_builtin_cmap_and_standard_font_data() {
    let (result, stats): (PdfJsProbe, _) = test_probe_with_resource_stats(RESOURCE_PDF).unwrap();
    assert_eq!(result.page_count, 1);
    assert_eq!(result.page_text, ["한글V8 resource probe"]);
    assert!(stats.cmaps > 0, "fixture must fetch an embedded CMap");
    assert!(
        stats.standard_fonts > 0,
        "fixture must fetch embedded standard font data"
    );
    assert_eq!(stats.denied, 0);
}

#[test]
fn resource_budgets_accept_the_inclusive_boundary_and_reject_one_less() {
    let (expected, stats) = test_probe_with_resource_stats(RESOURCE_PDF).unwrap();
    let (inclusive, inclusive_stats) = test_probe_with_resource_limits(
        RESOURCE_PDF,
        stats.max_item_bytes,
        stats.requests,
        stats.bytes,
    )
    .unwrap();
    assert_eq!(inclusive, expected);
    assert_eq!(inclusive_stats, stats);

    for (max_item_bytes, max_requests, max_total_bytes) in [
        (stats.max_item_bytes - 1, stats.requests, stats.bytes),
        (stats.max_item_bytes, stats.requests, stats.bytes - 1),
        (stats.max_item_bytes, stats.requests - 1, stats.bytes),
    ] {
        let error = test_probe_with_resource_limits(
            RESOURCE_PDF,
            max_item_bytes,
            max_requests,
            max_total_bytes,
        )
        .unwrap_err();
        assert_eq!(error.code, ErrorCode::OutputTooLarge);
    }
}

#[test]
fn oversized_callback_arguments_consume_the_request_budget() {
    let (stats, failure) = v8_runtime::test_resource_callback_rejects_unbounded_arguments()
        .expect("initialized V8 callback regression");
    assert_eq!(stats.requests, 2);
    assert_eq!(stats.denied, 2);
    assert_eq!(failure, Some(ErrorCode::ParseError));
}

#[test]
fn callback_denials_override_pdfjs_resource_fallbacks() {
    let denied_cmap =
        test_probe_with_resource_limits(RESOURCE_PDF, 0, 512, 8 * 1024 * 1024).unwrap_err();
    assert_eq!(denied_cmap.code, ErrorCode::OutputTooLarge);

    let denied_standard_font =
        test_probe_with_resource_limits(HELVETTE_PDF, 0, 512, 8 * 1024 * 1024).unwrap_err();
    assert_eq!(denied_standard_font.code, ErrorCode::OutputTooLarge);
}

#[test]
fn supervised_worker_matches_the_resource_callback_probe() {
    let expected = probe_pdf_text(RESOURCE_PDF).unwrap();
    let actual = worker_supervisor::supervise_worker(
        Path::new(env!("CARGO_BIN_EXE_pdfjs-worker")),
        RESOURCE_PDF,
        Duration::from_secs(15),
    )
    .unwrap();
    assert_eq!(actual, expected);
    assert_eq!(actual.page_text, ["한글V8 resource probe"]);
}

#[test]
fn resource_probe_fixture_has_the_authored_bytes() {
    use sha2::{Digest, Sha256};

    let digest = Sha256::digest(RESOURCE_PDF);
    let hex = digest
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect::<String>();
    assert_eq!(
        hex,
        "62601a0563488196397e88773eadfd7e2259c56fa33777a797ce958045a72679"
    );
}
