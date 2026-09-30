//! Bounded HWPX document parsing, metadata extraction, and structural validation.

#[allow(
    dead_code,
    reason = "private HWPML text candidate awaits semantic qualification and facade"
)]
mod hwpml;
mod hwpx;

use kordoc_ir::{DocumentMetadata, KordocError, ParseOptions, ParsedDocument};

/// One structural issue, in deterministic validation order.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub struct ValidateIssue {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub path: Option<String>,
    pub message: String,
}

/// Structural validation result. Directory entries do not contribute to `entry_count`.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ValidateResult {
    pub ok: bool,
    pub issues: Vec<ValidateIssue>,
    pub entry_count: usize,
}

/// Parse HWPX into source-neutral IR; the core owns Markdown and page projections.
pub fn parse_hwpx(bytes: &[u8], options: &ParseOptions) -> Result<ParsedDocument, KordocError> {
    hwpx::parse_hwpx(bytes, options)
}

/// Read bounded package metadata without parsing section content.
pub fn parse_hwpx_metadata(
    bytes: &[u8],
    options: &ParseOptions,
) -> Result<DocumentMetadata, KordocError> {
    hwpx::parse_hwpx_metadata(bytes, options)
}

/// Validate an HWPX package, decrypting protected members only with a valid password.
pub fn validate_hwpx(bytes: &[u8], password: Option<&str>) -> Result<ValidateResult, KordocError> {
    let result = hwpx::validate_hwpx(bytes, password)?;
    Ok(ValidateResult {
        ok: result.ok,
        issues: result
            .issues
            .into_iter()
            .map(|issue| ValidateIssue {
                path: issue.path,
                message: issue.message,
            })
            .collect(),
        entry_count: result.entry_count,
    })
}
