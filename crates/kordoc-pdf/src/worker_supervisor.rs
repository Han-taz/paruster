//! Private one-shot subprocess supervision for the embedded PDF.js worker.

use crate::v8_runtime::PdfJsProbe;
use crate::v8_runtime::text_document::PdfJsTextDocument;
use crate::worker_protocol::{self, ProtocolError};
use kordoc_ir::{ErrorCode, KordocError};
use std::path::Path;
use std::process::{Child, ChildStdin, ChildStdout, Command, ExitStatus, Stdio};
use std::sync::mpsc::{self, Receiver, TryRecvError};
use std::sync::{Condvar, Mutex};
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};

const MAX_CONCURRENT_WORKERS: usize = 2;
const POLL_INTERVAL: Duration = Duration::from_millis(2);

static ACTIVE_WORKERS: Mutex<usize> = Mutex::new(0);
static WORKER_SLOT_AVAILABLE: Condvar = Condvar::new();

/// Runs exactly one framed PDF request in a trusted, explicit worker executable.
///
/// The path must be absolute so process creation never searches `PATH`. The
/// deadline begins before slot acquisition and bounds process I/O and exit.
pub(crate) fn supervise_worker(
    executable: &Path,
    pdf: &[u8],
    timeout: Duration,
) -> Result<PdfJsProbe, KordocError> {
    supervise_worker_with_args(
        executable,
        &[],
        pdf,
        timeout,
        worker_protocol::write_request::<ChildStdin>,
        worker_protocol::read_response::<ChildStdout>,
        protocol_error,
    )
}

pub(crate) fn supervise_text_document_worker(
    executable: &Path,
    pdf: &[u8],
    timeout: Duration,
) -> Result<PdfJsTextDocument, KordocError> {
    supervise_worker_with_args(
        executable,
        &[],
        pdf,
        timeout,
        worker_protocol::write_text_request::<ChildStdin>,
        worker_protocol::read_text_response::<ChildStdout>,
        text_protocol_error,
    )
}

#[cfg(feature = "pdfjs-worker-tests")]
pub(crate) fn supervise_test_worker(
    executable: &Path,
    arguments: &[&str],
    pdf: &[u8],
    timeout: Duration,
) -> Result<PdfJsProbe, KordocError> {
    supervise_worker_with_args(
        executable,
        arguments,
        pdf,
        timeout,
        worker_protocol::write_request::<ChildStdin>,
        worker_protocol::read_response::<ChildStdout>,
        protocol_error,
    )
}

fn supervise_worker_with_args<T: Send + 'static>(
    executable: &Path,
    arguments: &[&str],
    pdf: &[u8],
    timeout: Duration,
    write_request: fn(&mut ChildStdin, &[u8]) -> Result<(), ProtocolError>,
    read_response: fn(&mut ChildStdout) -> Result<Result<T, KordocError>, ProtocolError>,
    map_protocol_error: fn(ProtocolError) -> KordocError,
) -> Result<T, KordocError> {
    if pdf.len() > worker_protocol::MAX_REQUEST_BYTES {
        return Err(worker_error(
            ErrorCode::OutputTooLarge,
            "PDF input exceeds worker limit",
        ));
    }
    if !executable.is_absolute() {
        return Err(worker_error(
            ErrorCode::MissingDependency,
            "PDF.js worker path is unavailable",
        ));
    }
    let deadline = Instant::now()
        .checked_add(timeout)
        .ok_or_else(|| worker_error(ErrorCode::ParseError, "PDF.js worker deadline is invalid"))?;
    let _slot = acquire_worker_slot(deadline)?;
    if Instant::now() >= deadline {
        return Err(worker_timeout());
    }
    let mut request = Vec::new();
    request
        .try_reserve_exact(pdf.len())
        .map_err(|_| worker_error(ErrorCode::OutputTooLarge, "PDF request allocation failed"))?;
    request.extend_from_slice(pdf);
    if Instant::now() >= deadline {
        return Err(worker_timeout());
    }

    let mut command = Command::new(executable);
    command.args(arguments);
    let mut child = command
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .map_err(|_| {
            worker_error(
                ErrorCode::MissingDependency,
                "PDF.js worker could not start",
            )
        })?;

    let Some(child_stdin) = child.stdin.take() else {
        terminate_child(&mut child, Vec::new());
        return Err(worker_error(
            ErrorCode::ParseError,
            "PDF.js worker input is unavailable",
        ));
    };
    let Some(child_stdout) = child.stdout.take() else {
        terminate_child(&mut child, Vec::new());
        return Err(worker_error(
            ErrorCode::ParseError,
            "PDF.js worker output is unavailable",
        ));
    };

    let (writer_tx, writer_rx) = mpsc::sync_channel(1);
    let writer = match thread::Builder::new()
        .name("pdfjs-worker-stdin".to_owned())
        .spawn(move || {
            let mut stdin = child_stdin;
            let result = write_request(&mut stdin, &request);
            let _ = writer_tx.send(result);
        }) {
        Ok(thread) => thread,
        Err(_) => {
            terminate_child(&mut child, Vec::new());
            return Err(worker_error(
                ErrorCode::ParseError,
                "PDF.js worker I/O could not start",
            ));
        }
    };

    let (reader_tx, reader_rx) = mpsc::sync_channel(1);
    let reader = match thread::Builder::new()
        .name("pdfjs-worker-stdout".to_owned())
        .spawn(move || {
            let mut stdout = child_stdout;
            let result = read_response(&mut stdout);
            let _ = reader_tx.send(result);
        }) {
        Ok(thread) => thread,
        Err(_) => {
            terminate_child(&mut child, vec![writer]);
            return Err(worker_error(
                ErrorCode::ParseError,
                "PDF.js worker I/O could not start",
            ));
        }
    };

    let outcome = wait_for_worker(
        &mut child,
        &writer_rx,
        &reader_rx,
        deadline,
        map_protocol_error,
    );
    match outcome {
        Ok((status, response)) if status.success() => {
            join_io_threads(vec![writer, reader])?;
            response
        }
        Ok((_status, _response)) => {
            terminate_child(&mut child, vec![writer, reader]);
            Err(worker_error(
                ErrorCode::ParseError,
                "PDF.js worker exited unsuccessfully",
            ))
        }
        Err(error) => {
            terminate_child(&mut child, vec![writer, reader]);
            Err(error)
        }
    }
}

type WorkerResponse<T> = Result<Result<T, KordocError>, ProtocolError>;

fn wait_for_worker<T>(
    child: &mut Child,
    writer: &Receiver<Result<(), ProtocolError>>,
    reader: &Receiver<WorkerResponse<T>>,
    deadline: Instant,
    map_protocol_error: fn(ProtocolError) -> KordocError,
) -> Result<(ExitStatus, Result<T, KordocError>), KordocError> {
    let mut writer_finished = false;
    let mut reader_result = None;
    let mut exit_status = None;
    loop {
        if Instant::now() >= deadline {
            return Err(worker_timeout());
        }
        if !writer_finished {
            match writer.try_recv() {
                Ok(Ok(())) => writer_finished = true,
                Ok(Err(error)) => return Err(map_protocol_error(error)),
                Err(TryRecvError::Disconnected) => {
                    return Err(worker_error(
                        ErrorCode::ParseError,
                        "PDF.js worker input failed",
                    ));
                }
                Err(TryRecvError::Empty) => {}
            }
        }
        if reader_result.is_none() {
            match reader.try_recv() {
                Ok(Ok(response)) => reader_result = Some(response),
                Ok(Err(error)) => return Err(map_protocol_error(error)),
                Err(TryRecvError::Disconnected) => {
                    return Err(worker_error(
                        ErrorCode::ParseError,
                        "PDF.js worker output failed",
                    ));
                }
                Err(TryRecvError::Empty) => {}
            }
        }
        if exit_status.is_none() {
            exit_status = child
                .try_wait()
                .map_err(|_| worker_error(ErrorCode::ParseError, "PDF.js worker status failed"))?;
        }
        if writer_finished && exit_status.is_some() && reader_result.is_some() {
            let status = exit_status.take().expect("checked above");
            let response = reader_result.take().expect("checked above");
            return Ok((status, response));
        }
        let remaining = deadline.saturating_duration_since(Instant::now());
        thread::sleep(POLL_INTERVAL.min(remaining));
    }
}

fn acquire_worker_slot(deadline: Instant) -> Result<WorkerSlot, KordocError> {
    let mut active = ACTIVE_WORKERS
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    loop {
        if *active < MAX_CONCURRENT_WORKERS {
            *active += 1;
            return Ok(WorkerSlot);
        }
        let remaining = deadline.saturating_duration_since(Instant::now());
        if remaining.is_zero() {
            return Err(worker_timeout());
        }
        let (next, wait) = WORKER_SLOT_AVAILABLE
            .wait_timeout(active, remaining)
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        active = next;
        if wait.timed_out() && *active >= MAX_CONCURRENT_WORKERS {
            return Err(worker_timeout());
        }
    }
}

struct WorkerSlot;

impl Drop for WorkerSlot {
    fn drop(&mut self) {
        let mut active = ACTIVE_WORKERS
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        *active = active.saturating_sub(1);
        WORKER_SLOT_AVAILABLE.notify_one();
    }
}

fn terminate_child(child: &mut Child, threads: Vec<JoinHandle<()>>) {
    let _ = child.kill();
    let _ = child.wait();
    for thread in threads {
        let _ = thread.join();
    }
}

fn join_io_threads(threads: Vec<JoinHandle<()>>) -> Result<(), KordocError> {
    let mut all_joined = true;
    for thread in threads {
        if thread.join().is_err() {
            all_joined = false;
        }
    }
    if all_joined {
        Ok(())
    } else {
        Err(worker_error(
            ErrorCode::ParseError,
            "PDF.js worker I/O failed",
        ))
    }
}

fn protocol_error(_error: ProtocolError) -> KordocError {
    worker_error(ErrorCode::ParseError, "PDF.js worker protocol failed")
}

fn text_protocol_error(error: ProtocolError) -> KordocError {
    if error == ProtocolError::TooLarge {
        worker_error(
            ErrorCode::OutputTooLarge,
            "PDF.js worker response exceeds its limit",
        )
    } else {
        protocol_error(error)
    }
}

fn worker_timeout() -> KordocError {
    worker_error(ErrorCode::ParseError, "PDF.js worker timed out")
}

fn worker_error(code: ErrorCode, message: &'static str) -> KordocError {
    KordocError::new(code, message)
}

#[cfg(test)]
mod tests {
    use super::join_io_threads;
    use std::sync::mpsc;
    use std::thread;
    use std::time::Duration;

    #[test]
    fn join_io_threads_joins_remaining_handles_after_a_panic() {
        let (release_tx, release_rx) = mpsc::sync_channel::<()>(0);
        let panicked = thread::spawn(|| panic!("intentional reader panic"));
        let waiting = thread::spawn(move || {
            let _ = release_rx.recv();
        });
        let (joined_tx, joined_rx) = mpsc::sync_channel(1);
        let joiner = thread::spawn(move || {
            let result = join_io_threads(vec![panicked, waiting]).is_err();
            let _ = joined_tx.send(result);
        });

        let result_before_release = joined_rx.recv_timeout(Duration::from_millis(25)).ok();
        let _ = release_tx.send(());
        let result_after_release = if result_before_release.is_none() {
            joined_rx.recv_timeout(Duration::from_secs(1)).ok()
        } else {
            None
        };
        joiner.join().unwrap();

        assert!(
            result_before_release.is_none(),
            "all handles must be joined"
        );
        assert_eq!(
            result_after_release,
            Some(true),
            "a panicked I/O thread must remain an error"
        );
    }
}
