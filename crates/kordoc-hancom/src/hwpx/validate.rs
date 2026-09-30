//! Structural HWPX validator with deterministic issue ordering.

#![allow(
    dead_code,
    reason = "H1b validator is wired by the H3 integration join"
)]

use super::{crypto, package::Package, xml};
use kordoc_ir::{ErrorCode, KordocError};

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ValidateIssue {
    pub path: Option<String>,
    pub message: String,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ValidateResult {
    pub ok: bool,
    pub issues: Vec<ValidateIssue>,
    pub entry_count: usize,
}

const REQUIRED: &[&str] = &[
    "mimetype",
    "META-INF/container.xml",
    "Contents/content.hpf",
    "Contents/header.xml",
    "Contents/section0.xml",
];

pub(crate) fn validate(
    package: &mut Package<'_>,
    password: Option<&str>,
) -> Result<ValidateResult, KordocError> {
    crypto::decrypt_package(package, password)?;
    let mut issues = Vec::new();
    let paths = package.file_paths();
    let first = package.first_path().unwrap_or_default();
    if first != "mimetype" {
        issues.push(issue(
            None,
            format!("First ZIP entry is '{first}', expected 'mimetype'"),
        ));
    }
    if package.contains("mimetype") {
        let bytes = package.read("mimetype")?.unwrap_or_default();
        let value = String::from_utf8_lossy(&bytes).trim().to_owned();
        if value != "application/hwp+zip" {
            issues.push(issue(
                Some("mimetype".into()),
                format!("Content is '{value}', expected 'application/hwp+zip'"),
            ));
        }
    }
    for path in REQUIRED {
        if !package.contains(path) {
            issues.push(issue(None, format!("Required file is missing: {path}")));
        }
    }
    for path in paths.iter().filter(|path| {
        [".xml", ".hpf", ".rdf"]
            .iter()
            .any(|suffix| path.ends_with(suffix))
    }) {
        let Some(bytes) = package.read(path)? else {
            continue;
        };
        if let Err(error) = xml::parse(&bytes) {
            if error.is_resource_limit() {
                return Err(KordocError::new(
                    ErrorCode::DecompressionBomb,
                    error.message,
                ));
            }
            issues.push(issue(
                Some(path.clone()),
                format!("XML well-formedness violation: {}", error.message),
            ));
        }
    }
    if package.contains("Contents/header.xml") {
        let header = package.read("Contents/header.xml")?.unwrap_or_default();
        let header = xml::parse_critical(&header)?;
        if let Some(head) = header.descendants("head").into_iter().next()
            && let Some(declared) = head.attr("secCnt").and_then(|s| s.parse::<usize>().ok())
        {
            let actual = paths.iter().filter(|p| section_file(p)).count();
            if declared != actual {
                issues.push(issue(
                    Some("Contents/header.xml".into()),
                    format!("secCnt={declared}, but found {actual} sectionN.xml files"),
                ));
            }
        }
    }
    if package.contains("Contents/content.hpf") {
        let hpf = package.read("Contents/content.hpf")?.unwrap_or_default();
        let root = xml::parse_critical(&hpf)?;
        for item in root.descendants("item") {
            if let Some(href) = item.attr("href")
                && !package.contains(href)
                && !package.contains(&format!("Contents/{href}"))
            {
                issues.push(issue(
                    Some("Contents/content.hpf".into()),
                    format!("Manifest references missing file: {href}"),
                ));
            }
        }
    }
    Ok(ValidateResult {
        ok: issues.is_empty(),
        issues,
        entry_count: package.file_count(),
    })
}

fn section_file(path: &str) -> bool {
    let Some(digits) = path
        .strip_prefix("Contents/section")
        .and_then(|name| name.strip_suffix(".xml"))
    else {
        return false;
    };
    !digits.is_empty() && digits.bytes().all(|byte| byte.is_ascii_digit())
}

fn issue(path: Option<String>, message: String) -> ValidateIssue {
    ValidateIssue { path, message }
}

fn _encrypted_error_class() -> ErrorCode {
    ErrorCode::Encrypted
}

#[cfg(test)]
#[path = "validate/tests.rs"]
mod tests;
