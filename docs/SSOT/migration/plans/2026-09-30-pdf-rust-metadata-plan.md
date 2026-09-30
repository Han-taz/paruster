# Private Rust PDF metadata normalization

> **Implementation gate:** Coordinator reviews and approves the exact oracle semantics and authored parity-capture fixture below before any native implementation. Execute tasks test-first in a separate focused PR after the current private PDF text-document candidate is published.

**Goal:** Convert the bounded, raw seven-field PDF.js Info projection into existing `kordoc_ir::DocumentMetadata` in Rust while preserving the local migration oracle's metadata behavior.

**Architecture:** The existing embedded PDF.js/V8 worker and KPDF kind-3/4 transfer continue to return raw `PdfJsMetadata`. A private, pure Rust normalizer in `kordoc-pdf` takes that value and `page_count`, then returns the already frozen IR metadata type. It does not register a parser or alter geometry, worker framing, DTO, shared IR, Python, or MCP contracts.

**Tech stack:** Rust 1.97, existing `kordoc-ir` and `kordoc-pdf` crates, current pinned PDF.js 4.10.38 fixture machinery. No runtime Node, external filesystem, network, new dependency, or oracle import.

## Exact behavior to approve

The read-only source is local `kordoc/src/pdf/parser.ts`, `extractPdfMetadata` and `parsePdfDate` around lines 584–621. The oracle is research-only and remains outside Git, wheels, sdists, CI and runtime. `PdfJsMetadata` has nullable raw `title`, `author`, `creator`, `subject`, `keywords`, `creation_date`, `modified_date`; its current decoder already caps each UTF-8 field at 4 KiB and their aggregate at 16 KiB before materialization. This normalizer must not turn an upstream quota, parse, or deadline failure into a best-effort absence.

| Raw field | Existing oracle result | Proposed IR field |
| --- | --- | --- |
| `title`, `author`, `creator`, `subject` | ECMAScript `trim()`; omit empty result | `title`, `author`, `creator`, `description` |
| `keywords` | Only if original string is nonempty after `trim()`; split on each comma or semicolon, trim each token, remove empty tokens; retain original order, case, and duplicates | `Some(Vec<String>)` for any nonempty original after trim, including `Some([])` for delimiter-only input; `None` only when missing or whitespace-only |
| `creation_date`, `modified_date` | Search for first `D:` followed by four ASCII digits; then greedily read up to five optional two-digit components (month, day, hour, minute, second). Defaults are `01`, `01`, `00`, `00`, `00`; return `YYYY-MM-DDTHH:mm:ss`. Ignore any unmatched prefix/suffix, including timezone. Missing match omits field. No calendar/range validation. | `created_at`, `modified_at` |
| `page_count` | Native `doc.numPages` even for metadata-only | `page_count: Some(u32)` |
| parse mode | Full parse sets `pageMode: layout`; metadata-only does not | `Some(PageMode::Layout)` or `None` |

The date rule intentionally accepts examples such as `D:20251399` and produces `2025-13-99T00:00:00`; adding calendar validation would diverge from the source. `xxD:2025Z` yields `2025-01-01T00:00:00`. `D:2025120X` consumes the month pair `12`, then defaults the unmatched day to `01`, yielding `2025-12-01T00:00:00`; do not reject the whole value. JavaScript `\d` here means ASCII digits. Implement ECMAScript trim semantics explicitly where Rust `str::trim` differs, especially U+FEFF, rather than silently changing the parity rule. Defensively revalidate each raw field against the existing 4 KiB UTF-8 cap and all seven against the 16 KiB aggregate cap before allocating, including a DTO constructed inside Rust tests; upstream caps alone do not authorize bypass at this private entry point. Every owned `String`/`Vec` copy must use checked sizes and fallible reservation. Quota/allocation denial remains a typed existing resource error, never a dropped field.

The oracle's `getMetadata()` exception path is best-effort and already represented by seven `None` values in the private DTO. Do not catch normalizer errors as an additional fallback. A comma/semicolon-only Keywords string must produce `Some([])`; whitespace-only or missing Keywords must produce `None`.

## Authored parity evidence before implementation

1. Add a separate stdlib-only recipe and CC0 fixture directory `crates/kordoc-pdf/tests/fixtures/pdfjs_metadata/`. Do not edit the current `pdfjs_text_document` recipe, input or SHA-256. Include padded text, empty Creator/Subject, mixed and repeated keyword delimiters, delimiter-only Keywords, an unanchored partial date (`D:2025120X`), an invalid calendar date, and missing/non-string ModDate. Generate separate explicitly named small inputs where one Info dictionary cannot express every case. Pin license, exact recipe/input byte counts and SHA-256 in the new fixture README.
2. Capture both `parsePdfDocument` and `extractPdfMetadataOnly` outcomes from the read-only local oracle against those authored PDF bytes under a temporary ignored directory **before** adding RED tests or implementation. Freeze a JSONL record per input/mode with the exact normalized metadata, including absence versus empty keyword array, `pageMode`, page count and date strings; each record identifies input SHA-256, oracle package version/commit and relevant source SHA-256. Review and pin capture JSONL bytes/hash offline. The capture script is research-only and must not enter Git, wheels, sdists, or CI. Check that full parsing of each authored PDF succeeds before promoting a full-parse expectation; otherwise use metadata-only capture for that case and state why.
3. Reproduce the approved outcomes with Rust unit tests constructed from private DTO values and one end-to-end private PDF.js DTO-to-normalizer test on the authored PDF. The test must fail on the pre-normalizer baseline, then pass with production conversion; it may not compute expected values from the Rust implementation. Keep any captured expected JSON byte-pinned and offline, with no runtime oracle dependency.

## Implementation sequence after approval

### Task 1: Freeze focused RED cases

- [ ] Add `crates/kordoc-pdf/src/v8_runtime/metadata.rs` unit tests (or an adjacent private test module) for every table row, `None` versus empty keyword vector, ordered duplicate keywords, ECMAScript whitespace including U+FEFF, partial/unanchored dates, invalid date components, missing date, and both parse modes. Assert the exact existing IR serialization shape via `serde_json`, including omitted absent fields.
- [ ] Add authored fixture and offline recipe/hash/capture checks in `crates/kordoc-pdf/tests/fixtures/pdfjs_metadata/` and `crates/kordoc-pdf/tests/pdfjs_metadata.rs`. Run `cargo test -p kordoc-pdf --features pdfjs-worker-tests --locked pdfjs_metadata` and record the expected RED result without weakening existing checks.

### Task 2: Implement the private conversion

- [ ] Implement `normalize_pdf_metadata(raw: &PdfJsMetadata, page_count: u32, mode: PdfMetadataMode) -> Result<DocumentMetadata, KordocError>` in `crates/kordoc-pdf/src/v8_runtime/metadata.rs`; expose only through the existing private runtime module. Use a tiny explicit mode enum (`FullParse`, `MetadataOnly`), not a public option or wire field. Keep the worker, DTO decoder, frames and existing parse registry untouched.
- [ ] Revalidate all input metadata fields against the same exact upstream per-field and aggregate caps; use checked, fallible reservation for each output string and keyword collection. Avoid a regex dependency by scanning bounded bytes for the first source-compatible date match; format exactly 19 ASCII bytes. Preserve the source's permissive partial matching and omit only when no four-digit year match exists.
- [ ] Run focused unit and authored-pdf tests GREEN. Check that an upstream over-cap metadata value still returns the existing typed fatal error and is never converted to `None`.

### Task 3: Verify and document the private checkpoint

- [ ] Run `cargo test -p kordoc-pdf --features pdfjs-worker-tests --locked`, `cargo clippy -p kordoc-pdf --all-targets --all-features --locked -- -D warnings`, `cargo fmt --all -- --check`, and the repository's offline fixture/contract checks. Run broader workspace gates required by CI without changing thresholds. Use a separate target directory if another worktree is compiling V8.
- [ ] In the same eventual implementation PR, update `docs/SSOT/components/pdf.md`, `docs/SSOT/migration/status.md`, and the SSOT index with the exact private capability and remaining geometry/layout/public-registration gaps. Append WIKI evidence without rewriting older entries. Use a focused branch, review, required CI, and squash merge.

## Explicit exclusions and acceptance boundary

This checkpoint returns only `DocumentMetadata` from an already bounded private DTO. It does not interpret `view_box`, rotations or text-item transforms; convert raw items to blocks; emit warnings; implement OCR, XMP, operator-list semantics, or public `parse_metadata`; or claim corpus-level PDF parity. Geometry is a separate approved wave. Release-wheel or OS containment claims cannot be inferred from a pure metadata normalizer.
