//! Test-only hostile child; never packaged as the production PDF.js worker.

use kordoc_ir::{ErrorCode, KordocError};
use std::io::{Read, Write};
use std::thread;
use std::time::Duration;

const MAX_REQUEST_BYTES: usize = 32 * 1024 * 1024;
const MAX_RESPONSE_BYTES: usize = 4 * 1024 * 1024;

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
struct PdfJsProbe {
    page_count: u32,
    page_text: Vec<String>,
}

#[derive(serde::Serialize)]
#[serde(tag = "status", rename_all = "snake_case")]
enum Response<'a> {
    Success { result: &'a PdfJsProbe },
    Failure { error: WireError<'a> },
}

#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
struct WireError<'a> {
    code: ErrorCode,
    message: &'a str,
}

fn read_request() -> Result<Vec<u8>, ()> {
    let mut stdin = std::io::stdin().lock();
    let mut header = [0; 10];
    stdin.read_exact(&mut header).map_err(|_| ())?;
    if &header[..6] != b"KPDF\x01\x01" {
        return Err(());
    }
    let length = u32::from_be_bytes(header[6..10].try_into().map_err(|_| ())?) as usize;
    if length > MAX_REQUEST_BYTES {
        return Err(());
    }
    let mut bytes = Vec::new();
    bytes.try_reserve_exact(length).map_err(|_| ())?;
    bytes.resize(length, 0);
    stdin.read_exact(&mut bytes).map_err(|_| ())?;
    let mut trailing = [0];
    if stdin.read(&mut trailing).map_err(|_| ())? != 0 {
        return Err(());
    }
    Ok(bytes)
}

fn write_response(result: Result<&PdfJsProbe, &KordocError>) -> Result<(), ()> {
    let response = match result {
        Ok(result) => Response::Success { result },
        Err(error) => Response::Failure {
            error: WireError {
                code: error.code,
                message: &error.message,
            },
        },
    };
    let payload = serde_json::to_vec(&response).map_err(|_| ())?;
    if payload.len() > MAX_RESPONSE_BYTES {
        return Err(());
    }
    let length = u32::try_from(payload.len()).map_err(|_| ())?;
    let mut header = *b"KPDF\x01\x02\0\0\0\0";
    header[6..].copy_from_slice(&length.to_be_bytes());
    let mut stdout = std::io::stdout().lock();
    stdout.write_all(&header).map_err(|_| ())?;
    stdout.write_all(&payload).map_err(|_| ())?;
    stdout.flush().map_err(|_| ())
}

fn marker_path(request: &[u8]) -> Option<std::path::PathBuf> {
    let path = std::str::from_utf8(request.strip_prefix(b"MARKER=")?).ok()?;
    Some(path.into())
}

fn mark(path: &Option<std::path::PathBuf>, suffix: &str) {
    if let Some(path) = path {
        let mut path = path.as_os_str().to_owned();
        path.push(suffix);
        let _ = std::fs::write(path, std::process::id().to_string());
    }
}

fn success() -> PdfJsProbe {
    PdfJsProbe {
        page_count: 1,
        page_text: vec!["supervised worker result".to_owned()],
    }
}

fn sleep_ms(arguments: &[String]) -> u64 {
    arguments
        .get(1)
        .and_then(|value| value.parse().ok())
        .unwrap_or(5_000)
        .min(30_000)
}

fn main() {
    let arguments = std::env::args().skip(1).collect::<Vec<_>>();
    let mode = arguments.first().map(String::as_str).unwrap_or("success");
    if mode == "blocked-read" {
        thread::sleep(Duration::from_millis(sleep_ms(&arguments)));
        return;
    }
    let request = match read_request() {
        Ok(request) => request,
        Err(_) => std::process::exit(21),
    };
    let marker = marker_path(&request);
    mark(&marker, ".started");

    match mode {
        "success" => {
            eprintln!("test helper stderr is deliberately discarded");
            if write_response(Ok(&success())).is_err() {
                std::process::exit(22);
            }
        }
        "error" => {
            let error = KordocError::new(ErrorCode::ParseError, "synthetic worker failure");
            if write_response(Err(&error)).is_err() {
                std::process::exit(22);
            }
        }
        "sleep" => {
            thread::sleep(Duration::from_millis(sleep_ms(&arguments)));
            mark(&marker, ".done");
            if write_response(Ok(&success())).is_err() {
                std::process::exit(22);
            }
        }
        "exit-nonzero" => std::process::exit(17),
        "abort" => std::process::abort(),
        "flood" => {
            let mut stdout = std::io::stdout().lock();
            let header = [
                b"KPDF\x01\x02".as_slice(),
                &((MAX_RESPONSE_BYTES as u32 + 1).to_be_bytes()),
            ]
            .concat();
            let _ = stdout.write_all(&header);
            let flood = [0x46; 64 * 1024];
            loop {
                if stdout.write_all(&flood).is_err() {
                    break;
                }
            }
        }
        "truncated-frame" => {
            let header = [b"KPDF\x01\x02".as_slice(), &12u32.to_be_bytes()].concat();
            let mut stdout = std::io::stdout().lock();
            let _ = stdout.write_all(&header);
            let _ = stdout.write_all(b"{}");
            let _ = stdout.flush();
        }
        "write-and-block" => {
            if write_response(Ok(&success())).is_err() {
                std::process::exit(22);
            }
            thread::sleep(Duration::from_millis(sleep_ms(&arguments)));
        }
        #[cfg(unix)]
        "close-stdout-and-block" => {
            if write_response(Ok(&success())).is_err() {
                std::process::exit(22);
            }
            use std::os::fd::FromRawFd;
            // SAFETY: this test-only process owns its standard-output descriptor and intentionally
            // closes it after a complete frame to exercise the parent's wait-for-exit race.
            unsafe { drop(std::fs::File::from_raw_fd(1)) };
            thread::sleep(Duration::from_millis(sleep_ms(&arguments)));
        }
        _ => std::process::exit(23),
    }
}
