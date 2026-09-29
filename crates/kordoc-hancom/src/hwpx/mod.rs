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

mod crypto;
mod images;
mod metadata;
mod package;
mod sections;
mod styles;
mod tables;
mod validate;
mod xml;
