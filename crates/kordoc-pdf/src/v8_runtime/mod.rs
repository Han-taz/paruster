//! Private, feature-gated PDF.js runtime.

mod allocator;
mod engine;
mod host;
#[cfg_attr(
    test,
    allow(
        dead_code,
        reason = "metadata normalization is exercised by its focused integration target"
    )
)]
pub(crate) mod metadata;
mod resources;
#[cfg_attr(
    test,
    allow(
        dead_code,
        reason = "the rich DTO decoder is exercised by its focused integration target and text-worker boundary"
    )
)]
pub(crate) mod text_document;

use kordoc_ir::{ErrorCode, KordocError};

const MAX_INPUT_BYTES: usize = 32 * 1024 * 1024;
const MAX_OUTPUT_BYTES: usize = 4 * 1024 * 1024;
const MAX_PAGES: u32 = 200;

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub(crate) struct PdfJsProbe {
    pub(crate) page_count: u32,
    pub(crate) page_text: Vec<String>,
}

pub(crate) fn probe_pdf_text(bytes: &[u8]) -> Result<PdfJsProbe, KordocError> {
    probe_with_limits(bytes, MAX_PAGES, MAX_OUTPUT_BYTES)
}

pub(crate) fn probe_with_limits(
    bytes: &[u8],
    max_pages: u32,
    max_output: usize,
) -> Result<PdfJsProbe, KordocError> {
    if bytes.is_empty() || !bytes.starts_with(b"%PDF-") {
        return Err(KordocError::new(
            ErrorCode::ParseError,
            "PDF input is malformed",
        ));
    }
    if bytes.len() > MAX_INPUT_BYTES {
        return Err(KordocError::new(
            ErrorCode::OutputTooLarge,
            "PDF input exceeds runtime limit",
        ));
    }
    engine::probe(bytes, max_pages, max_output)
}

#[cfg_attr(
    test,
    allow(
        dead_code,
        reason = "the rich DTO entry point is used by the kind-3 worker and focused integration test"
    )
)]
pub(crate) fn extract_text_document(
    bytes: &[u8],
) -> Result<text_document::PdfJsTextDocument, KordocError> {
    text_document::extract(bytes, text_document::TextDocumentLimits::default())
}

#[cfg(test)]
#[cfg_attr(
    test,
    allow(
        dead_code,
        reason = "this focused test hook is used only by the text-document integration target"
    )
)]
pub(crate) fn test_extract_text_document_with_limits(
    bytes: &[u8],
    limits: text_document::TextDocumentLimits,
) -> Result<text_document::PdfJsTextDocument, KordocError> {
    text_document::extract(bytes, limits)
}

#[cfg(test)]
#[cfg_attr(
    test,
    allow(
        dead_code,
        reason = "this focused test hook is used only by the text-document integration target"
    )
)]
pub(crate) fn test_stream_matches_default_get_text_content(
    bytes: &[u8],
) -> Result<bool, KordocError> {
    engine::test_stream_matches_default_get_text_content(bytes)
}

#[cfg(test)]
#[cfg_attr(
    test,
    allow(
        dead_code,
        reason = "this focused test hook is used only by the text-document integration target"
    )
)]
pub(crate) fn test_text_document_json_bytes(
    bytes: &[u8],
    limits: text_document::TextDocumentLimits,
) -> Result<usize, KordocError> {
    engine::test_text_document_json_bytes(bytes, limits)
}

#[cfg(test)]
pub(crate) fn test_host_has_no_io_globals() -> bool {
    engine::test_host_has_no_io_globals()
}

#[cfg(test)]
pub(crate) fn test_deadline_termination() -> Result<(), KordocError> {
    engine::test_deadline_termination()
}

#[cfg(test)]
pub(crate) fn test_external_buffer_limit() -> Result<(), KordocError> {
    engine::test_external_buffer_limit()
}

#[cfg(test)]
pub(crate) fn test_v8_engine_version() -> &'static str {
    engine::test_v8_engine_version()
}

#[cfg(test)]
#[allow(
    dead_code,
    reason = "resource integration entry point is unused by other runtime integration targets"
)]
pub(crate) fn test_probe_with_resource_stats(
    bytes: &[u8],
) -> Result<(PdfJsProbe, resources::ResourceStats), KordocError> {
    engine::test_probe_with_resource_stats(bytes)
}

#[cfg(test)]
#[allow(
    dead_code,
    reason = "resource integration entry point is unused by other runtime integration targets"
)]
pub(crate) fn test_probe_with_resource_limits(
    bytes: &[u8],
    max_item_bytes: usize,
    max_requests: usize,
    max_total_bytes: usize,
) -> Result<(PdfJsProbe, resources::ResourceStats), KordocError> {
    engine::test_probe_with_resource_limits(
        bytes,
        resources::ResourceLimits {
            max_item_bytes,
            max_requests,
            max_total_bytes,
        },
    )
}

#[cfg(test)]
#[allow(
    dead_code,
    reason = "resource integration entry point is unused by other runtime integration targets"
)]
pub(crate) fn test_resource_callback_rejects_unbounded_arguments()
-> Result<(resources::ResourceStats, Option<ErrorCode>), KordocError> {
    engine::test_resource_callback_rejects_unbounded_arguments()
}
