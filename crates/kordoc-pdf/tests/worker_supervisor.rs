#![cfg(feature = "pdfjs-worker-tests")]

#[path = "../src/v8_runtime/mod.rs"]
#[allow(
    dead_code,
    reason = "supervisor integration uses the runtime DTO and probe types but does not invoke V8 directly"
)]
mod v8_runtime;

#[path = "../src/worker_protocol.rs"]
mod worker_protocol;
#[path = "../src/worker_supervisor.rs"]
#[allow(
    dead_code,
    reason = "helper integration exercises the legacy request but does not invoke the rich production entry point"
)]
mod worker_supervisor;

use kordoc_ir::{ErrorCode, KordocError};
use std::path::{Path, PathBuf};
use std::sync::{Barrier, Mutex};
use std::thread;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};
use v8_runtime::PdfJsProbe;

static SUPERVISOR_TEST_LOCK: Mutex<()> = Mutex::new(());

fn helper() -> &'static Path {
    Path::new(env!("CARGO_BIN_EXE_pdfjs-test-worker"))
}

fn unique_path(label: &str) -> PathBuf {
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    std::env::temp_dir().join(format!(
        "kordoc_pdf_worker_{}_{}_{}.{}",
        std::process::id(),
        nonce,
        label,
        "marker"
    ))
}

fn request_with_marker(path: &Path) -> Vec<u8> {
    format!("MARKER={}", path.display()).into_bytes()
}

fn wait_for_file(path: &Path, timeout: Duration) -> bool {
    let deadline = Instant::now() + timeout;
    while Instant::now() < deadline {
        if path.exists() {
            return true;
        }
        thread::sleep(Duration::from_millis(2));
    }
    path.exists()
}

fn success_probe() -> PdfJsProbe {
    PdfJsProbe {
        page_count: 1,
        page_text: vec!["supervised worker result".to_owned()],
    }
}

fn run_helper(
    mode_and_args: &[&str],
    request: &[u8],
    timeout: Duration,
) -> Result<PdfJsProbe, KordocError> {
    worker_supervisor::supervise_test_worker(helper(), mode_and_args, request, timeout)
}

#[test]
fn explicit_helper_returns_one_framed_result_and_discards_stderr() {
    let _guard = SUPERVISOR_TEST_LOCK
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    let output = worker_supervisor::supervise_worker(
        helper(),
        b"authored test request",
        Duration::from_secs(2),
    )
    .unwrap();
    assert_eq!(output, success_probe());
}

#[test]
fn valid_worker_error_is_returned_without_exposing_helper_details() {
    let _guard = SUPERVISOR_TEST_LOCK
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    let error = run_helper(&["error"], b"test request", Duration::from_secs(2)).unwrap_err();
    assert_eq!(error.code, ErrorCode::ParseError);
    assert_eq!(error.message, "synthetic worker failure");
}

#[test]
fn sleeping_child_times_out_and_is_killed_before_io_threads_join() {
    let _guard = SUPERVISOR_TEST_LOCK
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    let marker = unique_path("sleep");
    let started = Instant::now();
    let error = run_helper(
        &["sleep", "5000"],
        &request_with_marker(&marker),
        Duration::from_millis(100),
    )
    .unwrap_err();
    assert_eq!(error.code, ErrorCode::ParseError);
    assert!(started.elapsed() < Duration::from_secs(2));
    assert!(wait_for_file(
        &marker.with_file_name(format!(
            "{}.started",
            marker.file_name().unwrap().to_string_lossy()
        )),
        Duration::from_millis(100)
    ));
    assert!(
        !marker
            .with_file_name(format!(
                "{}.done",
                marker.file_name().unwrap().to_string_lossy()
            ))
            .exists()
    );
}

#[test]
fn child_that_does_not_read_stdin_is_bounded_by_the_deadline() {
    let _guard = SUPERVISOR_TEST_LOCK
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    let started = Instant::now();
    let error = run_helper(
        &["blocked-read", "5000"],
        &vec![0x54; 1024 * 1024],
        Duration::from_millis(100),
    )
    .unwrap_err();
    assert_eq!(error.code, ErrorCode::ParseError);
    assert!(started.elapsed() < Duration::from_secs(2));
}

#[test]
fn nonzero_child_exit_is_rejected() {
    let _guard = SUPERVISOR_TEST_LOCK
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    let error = run_helper(&["exit-nonzero"], b"request", Duration::from_secs(2)).unwrap_err();
    assert_eq!(error.code, ErrorCode::ParseError);
}

#[test]
fn aborted_child_is_rejected_and_parent_test_process_remains_alive() {
    let _guard = SUPERVISOR_TEST_LOCK
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    let parent_pid = std::process::id();
    let error = run_helper(&["abort"], b"request", Duration::from_secs(2)).unwrap_err();
    assert_eq!(error.code, ErrorCode::ParseError);
    assert_eq!(std::process::id(), parent_pid);
}

#[test]
fn request_above_inclusive_cap_is_rejected_before_worker_spawn() {
    let _guard = SUPERVISOR_TEST_LOCK
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    let input = vec![0x58; worker_protocol::MAX_REQUEST_BYTES + 1];
    let parent_pid = std::process::id();
    let error =
        worker_supervisor::supervise_worker(helper(), &input, Duration::from_secs(2)).unwrap_err();
    assert_eq!(error.code, ErrorCode::OutputTooLarge);
    assert_eq!(std::process::id(), parent_pid);
}

#[test]
fn stdout_flood_is_rejected_without_waiting_for_a_blocked_writer() {
    let _guard = SUPERVISOR_TEST_LOCK
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    let started = Instant::now();
    let error = run_helper(&["flood"], b"request", Duration::from_secs(2)).unwrap_err();
    assert_eq!(error.code, ErrorCode::ParseError);
    assert!(started.elapsed() < Duration::from_secs(2));
}

#[test]
fn truncated_response_is_rejected() {
    let _guard = SUPERVISOR_TEST_LOCK
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    let error = run_helper(&["truncated-frame"], b"request", Duration::from_secs(2)).unwrap_err();
    assert_eq!(error.code, ErrorCode::ParseError);
}

#[test]
fn valid_frame_followed_by_open_stdout_is_bounded_by_the_deadline() {
    let _guard = SUPERVISOR_TEST_LOCK
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    let started = Instant::now();
    let error = run_helper(
        &["write-and-block", "5000"],
        b"request",
        Duration::from_millis(100),
    )
    .unwrap_err();
    assert_eq!(error.code, ErrorCode::ParseError);
    assert!(started.elapsed() < Duration::from_secs(2));
}

#[cfg(unix)]
#[test]
fn valid_frame_with_eof_before_process_exit_is_retained_until_exit() {
    let _guard = SUPERVISOR_TEST_LOCK
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    let output = run_helper(
        &["close-stdout-and-block", "150"],
        b"request",
        Duration::from_secs(2),
    )
    .unwrap();
    assert_eq!(output, success_probe());
}

#[test]
fn active_worker_slots_are_bounded() {
    let _guard = SUPERVISOR_TEST_LOCK
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    let marker_a = unique_path("concurrent_a");
    let marker_b = unique_path("concurrent_b");
    let marker_c = unique_path("concurrent_c");
    let barrier = std::sync::Arc::new(Barrier::new(3));
    let launch = |marker: PathBuf, barrier: std::sync::Arc<Barrier>| {
        thread::spawn(move || {
            barrier.wait();
            run_helper(
                &["sleep", "2500"],
                &request_with_marker(&marker),
                Duration::from_secs(4),
            )
        })
    };
    let first = launch(marker_a.clone(), barrier.clone());
    let second = launch(marker_b.clone(), barrier.clone());
    barrier.wait();
    let marker_a_started = marker_a.with_extension("marker.started");
    let marker_b_started = marker_b.with_extension("marker.started");
    let startup_deadline = Instant::now() + Duration::from_secs(2);
    while Instant::now() < startup_deadline
        && !(marker_a_started.exists() && marker_b_started.exists())
    {
        thread::sleep(Duration::from_millis(2));
    }
    assert!(marker_a_started.exists() && marker_b_started.exists());
    let third_started = Instant::now();
    let third = run_helper(
        &["sleep", "5000"],
        &request_with_marker(&marker_c),
        Duration::from_millis(100),
    );
    assert_eq!(third.unwrap_err().code, ErrorCode::ParseError);
    assert!(third_started.elapsed() < Duration::from_secs(2));
    assert!(!marker_c.with_extension("marker.started").exists());
    let _ = first.join().unwrap();
    let _ = second.join().unwrap();
}

#[test]
fn relative_executable_paths_are_rejected_without_path_search() {
    let _guard = SUPERVISOR_TEST_LOCK
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    let error = worker_supervisor::supervise_worker(
        Path::new("pdfjs-test-worker"),
        b"request",
        Duration::from_secs(1),
    )
    .unwrap_err();
    assert_eq!(error.code, ErrorCode::MissingDependency);
}
