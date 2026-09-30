//! HWPX private integration contract, frozen for H1a/H2a/H1b/H2b.
//!
//! H0 declares the interfaces here without executable placeholders. Crate-root exports belong to
//! the coordinator integration checkpoint, after the real implementations and tests exist:
//!
//! ```text
//! pub fn parse_hwpx(bytes: &[u8], options: &ParseOptions) -> Result<ParsedDocument, KordocError>;
//! pub fn parse_hwpx_metadata(bytes: &[u8], options: &ParseOptions) -> Result<DocumentMetadata, KordocError>;
//! pub fn validate_hwpx(bytes: &[u8], password: Option<&str>) -> Result<ValidateResult, KordocError>;
//!
//! pub(crate) struct Package<'a> { /* owned index, borrowed input, shared actual-byte budget */ }
//! impl<'a> Package<'a> {
//!     pub(crate) fn open(bytes: &'a [u8]) -> Result<Self, KordocError>;
//!     pub(crate) fn read(&mut self, path: &str) -> Result<Option<Vec<u8>>, KordocError>;
//!     pub(crate) fn section_paths(&mut self) -> Result<Vec<String>, KordocError>;
//! }
//! ```
//!
//! `Package::open` validates central-directory integrity before any recovery or member read.
//! It counts every central record, including directories and duplicate names, against the
//! inclusive 500-record HWPX ceiling. Unsafe and duplicate paths are hard failures. Consumers
//! use only `Package::read`; none opens `ZipArchive` independently. `read` returns `Ok(None)` only
//! for an absent path. It checks and charges actual logical plaintext bytes to one shared
//! inclusive 268,435,456-byte budget before appending/allocating. Re-reading the same logical
//! member does not charge its plaintext twice. Encrypted member plaintext is charged once after
//! successful all-or-none decryption; ciphertext and intermediates have separate bounds.
//!
//! `section_paths` follows the content.hpf spine order when usable; without a usable spine it
//! sorts `Contents/sectionN.xml` by numeric N, never ZIP record order. A malformed required
//! manifest is a hard failure rather than a fallback. The validator's private `ValidateResult`
//! retains exactly `ok`, ordered `issues`, and `entryCount`; each issue has `message` and optional
//! `path`. `entryCount` excludes directories, even though the safety limit counts them.
//!
//! Stable class map: untrusted/over-limit ZIP records or paths are `ZIP_BOMB`; actual extraction,
//! ciphertext, or expansion budget violations are `DECOMPRESSION_BOMB`; malformed critical XML
//! or required package metadata is `CORRUPTED`; no usable sections is `NO_SECTIONS`; missing or
//! wrong passwords are `ENCRYPTED`. A safely isolated bad section emits `PARTIAL_PARSE`; a safely
//! isolated member fault after central validation may emit `BROKEN_ZIP_RECOVERY`. Neither warning
//! is permitted to absorb a package, resource, manifest, or encryption hard error.

#![allow(
    dead_code,
    reason = "H3 entry points await the coordinator-owned crate facade"
)]

pub(crate) mod budget;
mod crypto;
mod images;
mod metadata;
mod package;
mod sections;
mod styles;
mod tables;
mod validate;
mod xml;

#[allow(
    unused_imports,
    reason = "H3 validator types await the coordinator-owned crate facade"
)]
pub(crate) use validate::{ValidateIssue, ValidateResult};

use kordoc_ir::{
    DocumentMetadata, ErrorCode, KordocError, ParseOptions, ParseWarning, ParsedDocument,
    WarningCode,
};

use self::{package::Package, sections::SectionInput};

pub(crate) fn parse_hwpx(
    bytes: &[u8],
    options: &ParseOptions,
) -> Result<ParsedDocument, KordocError> {
    let mut package = Package::open(bytes)?;
    crypto::decrypt_package(&mut package, options.password.as_deref())?;
    read_required_xml(&mut package, "META-INF/container.xml")?;
    read_required_xml(&mut package, "Contents/content.hpf")?;
    let header = read_required_xml(&mut package, "Contents/header.xml")?;
    let styles = styles::StyleCatalog::parse(&header)?;
    let mut metadata = metadata::extract_metadata(&mut package)?;
    let paths = package.section_paths()?;
    if paths.is_empty() {
        return Err(KordocError::new(
            ErrorCode::NoSections,
            "HWPX has no sections",
        ));
    }
    let mut inputs = Vec::with_capacity(paths.len());
    let mut recovery = Vec::new();
    for (index, path) in paths.into_iter().enumerate() {
        match package.read(&path) {
            Ok(Some(contents)) => inputs.push(SectionInput::new(path, contents)),
            Ok(None)
            | Err(KordocError {
                code: ErrorCode::Corrupted,
                ..
            }) => {
                recovery.push(ParseWarning {
                    page: u32::try_from(index + 1).ok(),
                    message: format!("HWPX section member could not be read: {path}"),
                    code: WarningCode::BrokenZipRecovery,
                });
                inputs.push(SectionInput::unavailable(path));
            }
            Err(error) => return Err(error),
        }
    }
    let mut lowered = sections::lower_sections(&inputs, &styles, None, options)?;
    if lowered.usable_sections == 0 {
        return Err(KordocError::new(
            ErrorCode::NoSections,
            "HWPX has no usable sections",
        ));
    }
    let mut image_cache = sections::resolve_selected_images(&mut lowered, &mut package)?;
    if options.pages.is_none() {
        images::sweep_unreferenced(
            &mut package,
            &mut image_cache,
            &mut lowered.blocks,
            &mut lowered.images,
            &mut lowered.warnings,
        )?;
    }
    metadata.page_mode = lowered.page_mode;
    metadata.page_count = Some(lowered.source_page_count);
    let warnings = recovery
        .into_iter()
        .chain(package.take_warnings())
        .chain(lowered.warnings)
        .collect::<Vec<_>>();
    Ok(ParsedDocument {
        blocks: lowered.blocks,
        page_count: metadata.page_count,
        metadata: Some(metadata),
        outline: (!lowered.outline.is_empty()).then_some(lowered.outline),
        warnings: (!warnings.is_empty()).then_some(warnings),
        images: (!lowered.images.is_empty()).then_some(lowered.images),
        page_evidence: Some(lowered.page_evidence),
        ..ParsedDocument::default()
    })
}

pub(crate) fn parse_hwpx_metadata(
    bytes: &[u8],
    _options: &ParseOptions,
) -> Result<DocumentMetadata, KordocError> {
    let mut package = Package::open(bytes)?;
    mark_metadata_encrypted_members(&mut package)?;
    metadata::extract_metadata(&mut package)
}

fn mark_metadata_encrypted_members(package: &mut Package<'_>) -> Result<(), KordocError> {
    let Some(manifest) = package.read("META-INF/manifest.xml")? else {
        return Ok(());
    };
    let root = xml::parse_critical(&manifest)?;
    let mut paths = Vec::new();
    for entry in root.descendants("file-entry") {
        if !entry
            .children
            .iter()
            .any(|child| child.name == "encryption-data")
        {
            continue;
        }
        if paths.len() >= 500 {
            return Err(KordocError::new(
                ErrorCode::DecompressionBomb,
                "HWPX encrypted metadata-path count exceeds its bound",
            ));
        }
        let path = entry.attr("full-path").ok_or_else(|| {
            KordocError::new(ErrorCode::Corrupted, "HWPX encrypted entry has no path")
        })?;
        if !package.contains(path) {
            return Err(KordocError::new(
                ErrorCode::Corrupted,
                "HWPX encrypted entry refers to a missing member",
            ));
        }
        paths.push(path.to_owned());
    }
    package.mark_encrypted_members(&paths)
}

pub(crate) fn validate_hwpx(
    bytes: &[u8],
    password: Option<&str>,
) -> Result<ValidateResult, KordocError> {
    let mut package = Package::open(bytes)?;
    validate::validate(&mut package, password)
}

fn read_required_xml(package: &mut Package<'_>, path: &str) -> Result<Vec<u8>, KordocError> {
    let bytes = package.read(path)?.ok_or_else(|| {
        KordocError::new(
            ErrorCode::Corrupted,
            format!("Required HWPX member missing: {path}"),
        )
    })?;
    xml::parse_critical(&bytes)?;
    Ok(bytes)
}
