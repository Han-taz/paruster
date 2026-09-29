# Parser Seam Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use `superpowers:subagent-driven-development`, `superpowers:test-driven-development`, and `superpowers:verification-before-completion`. P0 proves an injectable success path; it does not claim a real format parser or oracle success fixture.

**Goal:** Establish the acyclic IR-to-core parser boundary, exact parse option translation, immutable Python `Document` view, compatibility detection helpers, and a provenance-checked full-result parity harness without changing the production registry's current unsupported behavior.

**Architecture:** Format crates will depend only on `kordoc-ir` and return an internal `ParsedDocument` with structural evidence. `kordoc-core` detects the type, calls a core-owned registry adapter, and later projects that DTO into frozen `ParseSuccess`. P0 exercises the path with injected test parsers. PyO3 remains a detached serialization boundary; Python keeps the flat `TryParseResult` wire envelope and adds a derived immutable `document` view.

**Tech Stack:** Rust 1.97, serde, PyO3 0.29 `abi3-py310`, Python 3.10-3.14, pytest 9.0.3, maturin, Ruff, mypy.

## Commit 1: Internal parser DTO and options

**Files:** Create `crates/kordoc-ir/src/parsed.rs`, `crates/kordoc-ir/tests/parsed_contract.rs`; modify `crates/kordoc-ir/src/lib.rs`.

- [ ] Write failing tests importing `ParsedDocument`, `PageEvidence`, `ParseOptions`, and `PageSelection`. Cover recursive blocks/cells/captions, metadata, images, warnings, page evidence, page quality, quality summary, JSON roundtrip, omitted options versus explicit `false`, and rejection of `null`/unknown fields.
- [ ] Run `PYO3_PYTHON=.venv/bin/python3 cargo test -p kordoc-ir --test parsed_contract --locked`; expected failure is an unresolved import.
- [ ] Implement non-wire `ParsedDocument` without `file_type`, `success`, or `markdown`. It contains required blocks and optional page count/image-based/metadata/outline/warnings/images/page evidence/page quality/quality summary.
- [ ] Implement serializable internal options preserving `pages` as finite numeric values or the original range string and every oracle boolean as `Option<bool>`. Do not round or force integer pages before the document page count is known; the later page-range projector applies the oracle's `Math.round` semantics. Include password and non-callback OCR modes. Do not add `filePath`; callable OCR/progress adapters remain at the Python boundary.
- [ ] Re-run the focused test and commit `feat: add internal parser data contracts`.

## Commit 2: Injectable core dispatch and option validation

**Files:** Create `crates/kordoc-core/src/{parse,options}.rs`; modify `crates/kordoc-core/src/{lib,detect}.rs`. Keep injection/projector tests as private unit tests in `parse.rs`; use integration tests only for intentionally public production behavior.

- [ ] Write failing private unit tests for exact detected-format registry selection, unregistered format, all option names/defaults/unknowns, metadata override absence/presence, error propagation, panic sanitization, and validation ordering before parser invocation. Test-only injection remains private and never expands the public Rust API.
- [ ] Run `PYO3_PYTHON=.venv/bin/python3 cargo test -p kordoc-core --locked parse::tests`; expected failure is missing parser/registry APIs.
- [ ] Define a core-side parser adapter returning `ParsedDocument` and optional `extract_metadata`. Provide injected registry APIs for tests; format crates never implement or depend on the core trait.
- [ ] Keep the production registry empty. Empty input, 500 MiB limit, archive preflight, detection, detected `file_type`, and `UNSUPPORTED_FORMAT` behavior remain unchanged.
- [ ] Catch parser panics at the dispatch boundary, discard payloads, and return sanitized `PARSE_ERROR`. A test-only projector assembles exact `ParseSuccess`; P7 replaces it with real Markdown/pages projection.
- [ ] Re-run focused/core tests and commit `feat: add injectable parser dispatch seam`.

## Commit 3: Immutable Python success view and option boundary

**Files:** Modify `crates/kordoc-python/src/lib.rs`, `python/kordoc/{_models,_api,_native.pyi,__init__}.py`; create `tests/python/test_parse_success.py`; modify option assertions in existing Python tests.

- [ ] Write failing tests using a monkeypatched native success wire. Cover `result.document.markdown`, frozen/slotted behavior, recursive deep immutability, image byte conversion without converting ordinary integer arrays, absent versus null fields, bytes/path/file-like equivalence, typed failure raising, `try_parse()` flat serialization, strict option names/types, and explicit callable rejection.
- [ ] Run `uv run --python 3.10 --with pytest==9.0.3 pytest tests/python/test_parse_success.py -q`; expected failure is missing `Document`/`document`.
- [ ] Add frozen/slotted `Document` as a derived success-only view over `TryParseResult`; preserve the existing flat camelCase `to_dict()` wire shape and `parse()` return compatibility.
- [ ] Translate all frozen option fields to the native options mapping. Preserve omitted versus false. Reject `file_path`, unknown fields, callable OCR, and `on_progress` until their adapter task lands; never silently drop a field.
- [ ] Keep native production successes impossible in P0. Validate native detach/failure behavior; defer real installed-wheel success and performance evidence to P1/P7.
- [ ] Rebuild with maturin, run focused tests, and commit `feat: expose typed parser success boundary`.

## Commit 4: Compatibility detection helpers

**Files:** Modify `crates/kordoc-core/src/detect.rs`, `crates/kordoc-core/tests/detect_contract.rs`, PyO3/Python facade/stub/export files, public API status, and helper tests.

- [ ] Write failing Rust/Python tests for `detect_ole2_format`, `detect_zip_format`, `is_hwpx_file`, `is_old_hwp_file`, `is_pdf_file`, and `is_zip_file`.
- [ ] Preserve oracle edge semantics: `is_hwpx_file` is the `PK\x03\x04` signature alias, `is_old_hwp_file` is the four-byte OLE prefix, and malformed ZIP/OLE refinement returns `unknown` rather than leaking strict detector errors.
- [ ] Run focused detector and wheel-installed Python tests, update only verified helper dispositions from `planned`, and commit `feat: add compatibility detection helpers`.

## Commit 5: Full-result parity harness and current documentation

**Files:** Create `tests/golden/document-manifest.json`, `tests/golden/document/`, `tests/parity/test_document_goldens.py`, contract tests, component SSOT/WIKI evidence; modify public API manifest, Python/API/parity/workspace/status SSOT, indexes.

- [ ] Write failing tests requiring manifest ID, confined input/expected paths, SHA-256, license/provenance, deterministic generator, oracle commit/source digest/capture command, options, covered dimensions, and pointer-specific normalization allowlists.
- [ ] Keep detector `tests/golden/manifest.json` unchanged. Add an oracle-derived empty-input failure case; mark successful oracle document cases as zero/pending. Synthetic recursive success data tests the comparator only and never enters the parity numerator.
- [ ] Compare the complete result recursively with array order preserved and RFC 6901 first-difference diagnostics. Permit only ZIP timestamp/XML attribute normalization at explicitly listed pointers.
- [ ] Add Python-only `Document` to `contracts/public-api.json`; clarify existing `TryParseResult`. Do not duplicate the frozen TypeScript `ParseResult` type entry.
- [ ] Update current SSOT and append WIKI evidence. Run the full local gate and commit `test: add document parity harness`.

## Final gate

- [ ] Run Rust fmt, strict Clippy, workspace tests, rustdoc with warnings denied, coverage, Ruff, mypy, all Python tests, docs/artifact checks, audit/deny, and relevant fuzz targets from `docs/SSOT/operations/development.md`.
- [ ] Build and install the wheel on Python 3.10, verify no `kordoc/` path or oracle dependency appears in artifacts, request SOL review, push `feature/parser-seam`, and open a PR.
- [ ] Merge only after strict `ci-gate`, `security-gate`, `wheels-gate`, `fuzz-gate`, and `CodeQL` pass. P0 remains foundation infrastructure; P7/P1 provide the first real success path and oracle success parity.
