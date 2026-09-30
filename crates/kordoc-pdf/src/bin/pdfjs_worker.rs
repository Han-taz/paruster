//! One-document private PDF.js worker. Standard streams carry only the framed protocol.

#[path = "../v8_runtime/mod.rs"]
#[allow(
    dead_code,
    reason = "the private executable reuses the probe module but does not call test-only hooks"
)]
mod v8_runtime;
#[path = "../worker_protocol.rs"]
#[allow(
    dead_code,
    reason = "the one-way worker reuses framing but does not call parent-side protocol functions"
)]
mod worker_protocol;

use std::io::{self, Write};

use kordoc_ir::{ErrorCode, KordocError};

fn protocol_error(error: worker_protocol::ProtocolError) -> KordocError {
    let (code, message) = match error {
        worker_protocol::ProtocolError::TooLarge => (
            ErrorCode::OutputTooLarge,
            "PDF worker request exceeds its limit",
        ),
        _ => (
            ErrorCode::ParseError,
            "PDF worker request framing is invalid",
        ),
    };
    KordocError::new(code, message)
}

fn run() -> Result<(), worker_protocol::ProtocolError> {
    let stdin = io::stdin();
    let mut request = stdin.lock();
    let result = match worker_protocol::read_request(&mut request) {
        Ok(bytes) => v8_runtime::probe_pdf_text(&bytes),
        Err(error) => Err(protocol_error(error)),
    };

    let stdout = io::stdout();
    let mut response = stdout.lock();
    match worker_protocol::write_response(&mut response, result.as_ref()) {
        Ok(()) => response
            .flush()
            .map_err(|_| worker_protocol::ProtocolError::Io),
        Err(worker_protocol::ProtocolError::TooLarge) => {
            let bounded = KordocError::new(
                ErrorCode::OutputTooLarge,
                "PDF worker response exceeds its limit",
            );
            worker_protocol::write_response(&mut response, Err(&bounded))?;
            response
                .flush()
                .map_err(|_| worker_protocol::ProtocolError::Io)
        }
        Err(error) => Err(error),
    }
}

fn main() {
    if run().is_err() {
        std::process::exit(1);
    }
}
