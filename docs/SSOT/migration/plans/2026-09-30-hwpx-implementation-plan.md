# HWPX Parser Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use `superpowers:subagent-driven-development` for the bounded worker tasks, `superpowers:test-driven-development` for behavior changes, and `superpowers:verification-before-completion` before each commit or PR handoff. Track every step with the checkboxes below.

**Goal:** Deliver a bounded, source-neutral HWPX parser, metadata-only reader, and structural validator through the existing Rust core and Python API, with real-document parity evidence.

**Architecture:** `kordoc-hancom` depends only on `kordoc-ir` and returns `ParsedDocument`; it never constructs final Markdown or imports `kordoc-core`. A validated ZIP package reader meters every member, streams XML, and supplies independent section/metadata/image consumers. The coordinator wires the format adapter into core and the existing Python boundary after crate-local tests pass; core alone projects Markdown, pages, and the frozen success envelope.

**Tech Stack:** Rust 1.97, `zip`, `quick-xml`, checked decompression, AES-256-CBC/PBKDF2/SHA-256, PyO3 `abi3-py310`, Python 3.10-3.14, pytest, cargo-fuzz.

---

## Preconditions and resolved behavior

P0 and P7 are merged. Read the [approved design](../2026-09-29-rust-python-port-design.md), [master DAG](2026-09-30-product-port-implementation-plan.md), [parser seam](2026-09-30-parser-seam-implementation-plan.md), [IR contract](../../contracts/ir.md), [error contract](../../contracts/errors.md), [detection rules](../../components/detection.md), and [parity policy](../../quality/parity.md). Before worker branches start, merge a coordinator-owned `feature/parser-wave-scaffold` PR that registers minimal `kordoc-hancom` and `kordoc-pdf` crates in the root workspace, pins reviewed candidate dependencies, creates compilable module/test shells, regenerates the root lockfile, and passes the normal protected CI. Implement on `feature/parse-hwpx` from that scaffold merge; this planning worktree is not the implementation branch. The ignored `kordoc/` checkout is read-only research material, absent from builds, tests, CI, wheels, and sdists.

These decisions are acceptance criteria:

1. Core detection and its generic ZIP preflight run first. A corrupt central directory, forged offsets/counts, multidisk representation, or other preflight rejection remains `ZIP_BOMB` or the detector's existing result. Never scan local headers to bypass that gate. `BROKEN_ZIP_RECOVERY` is permitted only for an already validated HWPX archive whose individual member fails safely and independently; preserve sound sections, boundedly skip that member, and record the warning. A damaged required manifest, missing all sections, or a package whose member map cannot be trusted is a typed hard failure.
2. The HWPX-specific ceiling is **500 central-directory records, including directory records**, inclusive, after the generic 100,000-entry gate. Count duplicate names as separate records; reject ambiguous duplicates and unsafe names. The HWPX aggregate *actual* uncompressed-byte ceiling is 268,435,456 bytes inclusive across all logical member plaintext, including metadata, styles, images, sections, and decrypted output. Count an encrypted member's decrypted content once against this ceiling; independently bound its ciphertext and intermediate buffers. Keep the generic declared 1,073,741,824-byte gate. Use checked counters before allocating or appending; do not quietly truncate.
3. XML uses namespace-local element names, rejects DTD/entity declarations and external resolution, and caps element depth at **200**. Malformed or over-depth package-wide critical XML (`Contents/content.hpf`, `META-INF/manifest.xml` when present, and required header/container metadata) is a hard `CORRUPTED` failure. A malformed or over-depth *section* emits `PARTIAL_PARSE` for that section and preserves other sections. Resource/expansion violations remain hard `DECOMPRESSION_BOMB` or `ZIP_BOMB`, even when encountered while reading a section. No XML depth case silently returns clipped text.
4. Encrypted ODF entries use the manifest-declared AES-256-CBC scheme, SHA-256 password start key, bounded PBKDF2, bounded raw-deflate output, and constant-time `sha256-1k` checksum comparison. Each entry allows at most **1,000,000** PBKDF iterations, and the **sum of declared iterations across encrypted entries allows at most 4,000,000 per document**, checked before any derivation. Count every encrypted manifest entry once, irrespective of PRF fallback. Unsupported algorithms and invalid parameters fail explicitly. Missing and wrong passwords return stable `ENCRYPTED` with safe distinct password-required/invalid diagnostics, mapped to existing Python `EncryptedError`; a wrong password and corrupt ciphertext are not distinguished to callers. The decrypted plaintext is installed only after every required encrypted member succeeds.
5. `validate_hwpx(input, password=None)` reports the same typed `ENCRYPTED` password-required/invalid failures for encrypted input. With no password it does not decrypt or inspect encrypted content as plaintext. With a valid password it returns the frozen `ValidateResult` (`ok`, ordered `issues`, `entryCount`), with `entryCount` counting non-directory files as the existing schema specifies. The parser's 500-record safety ceiling still counts directories. Do not add an unapproved error code or change `ValidateIssue`/`ValidateResult` wire fields.
6. P1 exposes no Python `on_progress` callback; the existing explicit `NotImplementedError` remains. No Windows COM or DRM fallback is added. Format parsing may use existing options but must preserve omitted versus explicit false. Metadata-only extraction is bounded and does not parse sections. The format crate returns IR, metadata, assets, page evidence and warnings; core applies P7 Markdown/table projections.

## File map and ownership

| Owner | Files | Responsibility |
| --- | --- | --- |
| Luna A, package/security | `src/hwpx/{package,crypto,metadata,validate}.rs` and colocated `src/hwpx/{package,crypto,metadata,validate}/tests.rs` | Validated package access, counting/decompression, encryption, metadata-only path, validator. Own no `sections.rs` or structural tests. |
| Luna B, content/IR | `src/hwpx/{xml,sections,styles,tables,images}.rs` and colocated module `tests.rs` files under those paths | Streaming XML and structural IR lowering. Own no package/crypto/validator files. |
| SOL manager | `src/hwpx/mod.rs`, `tests/hwpx_integration.rs`, `docs/SSOT/components/hwpx.md`, generated HWPX fixture recipe and crate-local test fixtures | Freeze private interfaces, integrate workers, review safety/parity, prepare coordinator checkpoint. No simultaneous edits with either Luna. |
| Coordinator only | root `Cargo.toml`, `Cargo.lock`, scaffolded `crates/kordoc-hancom/{Cargo.toml,src/lib.rs}`, `crates/kordoc-core/src/{parse,detect}.rs`, `crates/kordoc-python/src/lib.rs`, `python/kordoc/{_api,_models,_native.pyi,__init__}.py`, `contracts/`, `tests/golden/document-manifest.json`, `tests/parity/`, `tests/python/`, `fuzz/`, shared workflows, `docs/SSOT/{README.md,architecture/workspace.md,contracts/python-api.md,migration/status.md}`, WIKI | Workspace/dispatch/Python/manifest/CI and contract status. Shared contract changes require explicit coordinator approval before implementation. |

The manager may stage crate-local integration only after each worker's owned files are stable. The scaffold makes the crate a root-workspace member before any worker commit, so every slice is compiled and tested by protected workspace CI. No nested `[workspace]` or uncommitted local lockfile is allowed. If a worker needs a dependency change, the coordinator updates the root dependency pin and lockfile on the integration branch before that slice is reviewed. No worker edits the root workspace or shared contracts.

Private entry points to agree before worker code:

```rust
pub fn parse_hwpx(bytes: &[u8], options: &ParseOptions) -> Result<ParsedDocument, KordocError>;
pub fn parse_hwpx_metadata(bytes: &[u8], options: &ParseOptions) -> Result<DocumentMetadata, KordocError>;
pub fn validate_hwpx(bytes: &[u8], password: Option<&str>) -> Result<ValidateResult, KordocError>;

// package.rs hands out a validated, metered member reader. No consumer opens ZipArchive directly.
pub(crate) struct Package<'a> { /* owned index, borrowed input, shared actual-byte budget */ }
impl<'a> Package<'a> {
    pub(crate) fn open(bytes: &'a [u8]) -> Result<Self, KordocError>;
    pub(crate) fn read(&mut self, path: &str) -> Result<Option<Vec<u8>>, KordocError>;
    pub(crate) fn section_paths(&mut self) -> Result<Vec<String>, KordocError>;
}
```

`ValidateResult` and `ValidateIssue` are crate-local representations until the coordinator maps them to the frozen contract. The package's byte budget remains shared after decrypting, and no consumer can bypass it. The manager reviews signatures before either worker starts dependent code.

## Task DAG and commit boundaries

```text
S0 protected root-workspace scaffold (coordinator + managers)
  |
H0 private interfaces + fixture recipe (manager)
  ├─ H1a package/limits (Luna A) ───────────┐
  └─ H2a XML/sections/styles (Luna B) ──────┤
                                            ├─ H1b crypto/metadata/validate (Luna A)
                                            └─ H2b tables/images (Luna B)
                                                     \        /
                                      H3 crate integration, validator + parity review (manager)
                         |
          H4 root/core/Python/contracts/golden checkpoint (coordinator)
                         |
          H5 fuzz, full gates, hosted PR and squash merge
```

H1a and H2a run concurrently after H0. Their reviewed interfaces form an explicit join: H1b consumes H2a's bounded XML reader for manifests and metadata, while H2b consumes H1a's metered `Package` reader for images and package-backed content. H1b and H2b may then run concurrently; neither starts before both prerequisite commits are green. Separate commits by behavioral slice: `test/feat: bound HWPX package`, `test/feat: stream HWPX XML`, `test/feat: decrypt HWPX entries`, `test/feat: lower HWPX sections`, `test/feat: validate HWPX structure`, then coordinator `feat: expose HWPX parsing in Python`. Each commit includes its failing-test evidence and the green command in its handoff. PRs may split only at these dependency boundaries, remain based on non-main branches until their prerequisites merge, and never change an incomplete public capability to implemented.

### Task H0: Synthetic fixture contract and private interfaces — SOL manager

**Files:** The scaffold has already created `crates/kordoc-hancom/Cargo.toml`, `src/lib.rs`, `src/hwpx/mod.rs`, and a compilable `tests/hwpx_integration.rs`. H0 creates `tests/support/hwpx_fixture.rs` and fills the private signatures/test shell; later publish `docs/SSOT/components/hwpx.md` with accepted behavior.

- [ ] Define a deterministic fixture builder that writes fixed-order, fixed-timestamp ZIP records and explicit UTF-8 XML. Provide `minimal`, `two_section_spine_reversed`, `nested_table`, `page_cache`, `missing_page_cache`, `encrypted_sha1`, `encrypted_sha256`, `malformed_section`, `over_depth_section`, `over_depth_manifest`, `directory_entry_500`, and `directory_entry_501` recipes. Record generator source digest and fixture SHA-256; generated bytes are CC0-1.0. Never copy oracle fixture bytes into the repo.
- [ ] Record the oracle source commit/digests and exact local capture command for permitted synthetic cases; capture full flat `ParseResult` only from the read-only oracle. Mark any security divergence, such as strict ZIP refusal, as explicit non-parity evidence. No change to answers, scoring, or normalization rules.
- [ ] Freeze `Package` error classes, `read` budget semantics, section ordering, and the three public crate functions above. Test the fixture builder's byte-for-byte determinism. Expected initial failure: the crate entry points do not exist.
- [ ] Commit the recipe and private skeleton only after `cargo +1.97.0 test -p kordoc-hancom --test hwpx_integration --locked` passes the deterministic fixture test in the registered root workspace.

### Task H1a: Validated package access and limits — Luna A

**Files:** `src/hwpx/package.rs`, `src/hwpx/package/tests.rs`.

- [ ] Write failing tests `rejects_corrupt_central_directory_before_recovery`, `counts_directory_and_duplicate_records_toward_500`, `accepts_500_records_rejects_501`, `rejects_traversal_absolute_backslash_and_duplicate_names`, `meters_all_members_at_256_mib`, `rejects_member_crc_or_extent_damage_without_local_header_scan`, and `retains_other_sections_after_bounded_member_failure`. Assert exact stable codes, inclusive limits, and bounded warning behavior. The 501st *directory* record must fail even with only one file.
- [ ] Run `cargo +1.97.0 test -p kordoc-hancom --lib hwpx::package::tests --locked`; expected red: unresolved `Package` or assertion failures, never a missing fixture/oracle error.
- [ ] Implement `Package::open/read/section_paths` with checked arithmetic, central-record counting, canonical relative path validation, one shared actual-byte counter, per-member bounded streaming reads, and manifest spine order followed by numeric section fallback. The direct crate entry point must independently validate central-directory spans, counts, local extents, and single-disk ZIP64 metadata before reading members; differential tests compare its acceptance boundary with core preflight. The coordinator's integration test proves strict core detection runs first. Do not recover a corrupt central directory. A safely skippable optional member may warn; a required member fails.
- [ ] Re-run the focused test and `cargo fmt --manifest-path crates/kordoc-hancom/Cargo.toml -- --check`; commit package behavior with its test.

### Task H1b: Encryption, metadata-only path, and validation — Luna A

**Files:** `src/hwpx/{crypto,metadata,validate}.rs` and their colocated module test files.

- [ ] Add red tests `password_required_without_decryption`, `wrong_password_is_encrypted_without_leak`, `decrypts_odf_sha1_and_sha256`, `rejects_iteration_1000001`, `accepts_aggregate_4000000_rejects_4000001`, `rejects_unsupported_crypto_parameters`, `decrypts_all_or_none`, and `inflated_ciphertext_counts_in_shared_budget`. Use deterministic synthetic ciphertext, exact manifest fields, both PRFs, and known plaintext hashes.
- [ ] Add red tests `metadata_only_does_not_read_sections`, `validator_checks_mimetype_first_and_required_entries`, `validator_checks_seccnt_and_manifest_hrefs`, `validator_reports_xml_path_in_order`, `validator_encrypted_missing_or_wrong_password_is_typed`, and `validator_counts_files_but_safety_counts_directories`. Use the frozen result keys and deterministic diagnostic order.
- [ ] Run `cargo +1.97.0 test -p kordoc-hancom --lib --locked`; expected red: missing functions or wrong stable codes.
- [ ] Implement manifest parsing through the bounded XML reader, checked per-entry and aggregate iteration budgets *before* PBKDF, bounded AES/decompression and checksum verification, metadata-only OPF/Dublin Core extraction, and structural validator. Missing password returns `ENCRYPTED` before decrypting any member. Wrong password returns `ENCRYPTED` after bounded verification. Validation never returns `ok: true` after skipping an encrypted member.
- [ ] Re-run focused tests and commit encryption/metadata/validation in separate commits if review size warrants; hand off test names and limits.

### Task H2a: XML stream and section structure — Luna B

**Files:** `src/hwpx/{xml,sections,styles}.rs` and their colocated module test files.

- [ ] Add red tests `uses_local_names_for_prefixed_elements`, `rejects_dtd_and_entities`, `critical_xml_depth_201_is_corrupted`, `section_depth_201_is_partial_parse`, `malformed_middle_section_preserves_neighbors`, `spine_order_wins_over_zip_order`, `falls_back_to_numeric_section_order`, `keeps_paragraph_run_spans_and_heading_outline`, `retains_footnotes_and_endnotes`, `uses_layout_cache_when_all_sections_usable`, and `falls_back_to_section_pages_recursively`. Assert that a failed section cannot leave half its blocks or shared numbering/page state behind.
- [ ] Run `cargo +1.97.0 test -p kordoc-hancom --lib --locked`; expected red: missing XML/section lowering APIs.
- [ ] Implement a non-expanding streaming XML reader: namespace-local names, entity/DTD refusal, depth counter, explicit UTF-8 errors, and bounded text buffers. Parse each section into temporary blocks/shared-state delta, commit only on success, and apply `PARTIAL_PARSE` for section-local syntax/depth faults. Keep package-wide XML faults hard. Preserve source order, run spans, headings/outline, notes, page cache, and page fallback. Apply page selection only after the layout-versus-section decision; retain omitted/false option semantics.
- [ ] Re-run focused tests and commit XML/section behavior. No final Markdown call is allowed in this crate.

### Task H2b: Tables, images, and structural IR — Luna B

**Files:** `src/hwpx/{tables,images}.rs` and their colocated module test files.

- [ ] Add red tests `keeps_merged_cell_topology`, `keeps_nested_table_and_caption_blocks_in_order`, `preserves_header_and_trailing_empty_cells`, `resolves_referenced_images_with_bytes_and_mime`, `skips_missing_optional_image_with_warning`, and `rejects_image_expansion_beyond_budget`. Check recursive IR fields, text, image bytes, warnings, and page numbers; no table flattening.
- [ ] Run `cargo +1.97.0 test -p kordoc-hancom --lib --locked`; expected red: missing table/image lowering or mismatched IR.
- [ ] Implement bounded table/caption and image lowering through the shared package reader. Enforce P7's 200-column/2,000,000-cell and 64-level logical recursion budgets without dropping content. Preserve nested table/caption blocks and only issue `SKIPPED_IMAGE` for optional missing/unsupported image data, never for a size-limit failure.
- [ ] Re-run focused tests and commit structural behavior with exact recursive-IR assertions.

### Task H3: Crate integration and review — SOL manager

**Files:** `src/hwpx/mod.rs`, `tests/hwpx_integration.rs`, `docs/SSOT/components/hwpx.md`.

- [ ] Add a red integration test exercising all three crate functions on the deterministic fixture suite. Assert `ParsedDocument` has no Markdown, metadata-only performs no section reads, `validate_hwpx` keeps the frozen result shape, parser errors retain stable codes, and two usable sections survive a malformed middle section with one `PARTIAL_PARSE` warning.
- [ ] Run `cargo test --manifest-path crates/kordoc-hancom/Cargo.toml --test hwpx_integration`; expected red: missing orchestration or result mismatch.
- [ ] Compose package, crypto, XML, sections, tables, images, metadata, and validation. Review that section-local catch handles only section-local XML/member faults; package integrity, resource limits, manifest/header faults, and encryption failures escape as hard errors. Review allocation sites and hostile UTF-8/path handling. Write the normative HWPX component page for the code PR.
- [ ] Re-run all three crate tests; record successful synthetic cases, deliberate safety divergences, residual gaps, and measured parser throughput/RSS against permitted oracle inputs. Hand the coordinator a crate-local green commit and fixture provenance.

### Task H4: Serialized core, Python, contract-status, and parity checkpoint — coordinator

**Files:** Root workspace/lockfile; `crates/kordoc-core/src/parse.rs`; PyO3/Python files listed above; `contracts/public-api.json`; `tests/golden/document-manifest.json`, `tests/golden/document/`, `tests/parity/test_document_goldens.py`, `tests/python/test_hwpx.py`; SSOT index/architecture/Python contract/status and append-only WIKI entry.

- [ ] Add red core tests `hwpx_dispatch_uses_strict_detector_first`, `hwpx_parse_assembles_markdown_in_core`, `hwpx_metadata_uses_specialized_path`, and `hwpx_parser_panic_is_sanitized`. The corrupt-central-directory case must stop before `kordoc-hancom`. Verify format crate dependencies contain `kordoc-ir` and never `kordoc-core`.
- [ ] Add red installed-wheel Python tests `parse_hwpx_bytes_path_stream_same_result`, `parse_hwpx_returns_document_and_full_wire`, `validate_hwpx_frozen_result`, `encrypted_parse_and_validate_raise_encrypted_error`, `on_progress_remains_unimplemented`, and `try_parse_hwpx_returns_typed_failure`. Check GIL release, image `bytes`, exact camelCase serialization, and typed `ENCRYPTED` diagnostics.
- [ ] Add manifest cases with input/expected SHA-256, CC0 synthetic generator and digest, pinned oracle commit/source digest, exact capture command, options, dimensions, evidence kind, and normalization pointer lists. Capture successful oracle outputs where behavior is comparable; security-only cases stay source-contract smoke or are labeled intentional divergence. The parser-success numerator advances only for captured full-result cases that pass recursively.
- [ ] Verify the scaffolded workspace membership, update reviewed dependencies and root `Cargo.lock` if needed, then wire the core adapter/registry, P7 Markdown/pages assembly, PyO3 and public Python `parse_hwpx`/`validate_hwpx`, native stub, immutable models, and error translation. Update only verified `parseHwpx`, `validateHwpx`, `ValidateIssue`, and `ValidateResult` manifest dispositions after installed-wheel tests pass. Any shared schema or error-contract change is a separate coordinator-approved decision.
- [ ] Run `cargo +1.97.0 test -p kordoc-hancom --lib --locked`, `cargo +1.97.0 test -p kordoc-hancom --test hwpx_integration --locked`, `cargo +1.97.0 test -p kordoc-core --locked`, and `uv run --python 3.10 pytest tests/parity/test_document_goldens.py tests/python/test_hwpx.py -q`; expected green with exact parity numerator and no skipped required case.

### Task H5: Fuzz, full gates, and merge evidence — coordinator with SOL review

**Files:** `fuzz/fuzz_targets/{hwpx_package,hwpx_xml}.rs`, `fuzz/Cargo.toml`, small synthetic `fuzz/corpus/` seeds, shared CI wiring, SSOT status/WIKI evidence.

- [ ] Add package fuzz assertions that arbitrary bytes never panic, exceed bounded extraction, bypass central-directory rejection, or count fewer than all records. Add XML fuzz assertions for DTD/entity refusal, depth 200, deterministic section-local `PARTIAL_PARSE`, and no leaked half-section. Add encrypted-manifest fuzz cases for iteration overflow/aggregate accounting. Every crash becomes a deterministic regression test before merging.
- [ ] Run 30-second local smoke for both targets using pinned nightly/cargo-fuzz; schedule longer existing weekly campaigns. Do not weaken time budgets, corpus cases, or safety limits.
- [ ] Run the [full local gate](../../operations/development.md): Rust fmt, strict Clippy, locked workspace tests and docs, coverage, Python pytest/Ruff/mypy, docs checker, wheel/sdist forbidden-path scan, audit/deny, and relevant existing fuzz targets. Build/install the wheel on Python 3.10 and run the HWPX API tests against that installed artifact. Record exact commands, outputs, input/expected hashes, covered dimensions, warnings/errors, performance/RSS, and missing restricted-corpus status.
- [ ] Request SOL review of package safety, crypto, section isolation, parity, and fixture provenance; then coordinator review of shared integration. Push only the focused feature branch, open a PR, and squash-merge only after `ci-gate`, `security-gate`, `wheels-gate`, `fuzz-gate`, and CodeQL succeed. The status page says HWPX complete only after the merged evidence supports it.

## Completion evidence checklist

- [ ] Full recursive IR, Markdown, metadata, outline, images, page behavior, warnings, options, validation result, encrypted errors, and malformed/security behavior have named passing Rust and installed-wheel Python cases.
- [ ] The document manifest reports its exact successful oracle-capture numerator and first-difference output; no detector or synthetic source-smoke result is counted as parser parity.
- [ ] A corrupt central directory cannot reach HWPX recovery; 500-record and 256 MiB HWPX limits are boundary tested; XML depth and PBKDF per-entry/aggregate boundaries are tested exactly.
- [ ] Crate dependency graph remains `kordoc-hancom -> kordoc-ir`, `kordoc-core -> kordoc-hancom + kordoc-ir`; core owns final Markdown and Python exposes no P1 progress callback.
- [ ] Only generated or independently licensed fixtures enter Git; no `kordoc/` file/path or oracle runtime requirement appears in source, test, wheel, sdist, or CI artifact.
- [ ] SSOT updates and append-only WIKI evidence accompany the implementation PR; required hosted gates pass on its final head before squash merge.
