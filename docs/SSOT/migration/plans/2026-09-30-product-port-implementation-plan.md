# Rust/Python Full Product Port Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use `superpowers:subagent-driven-development` for each wave, `superpowers:test-driven-development` for every behavior change, and `superpowers:verification-before-completion` before every commit or PR hand-off. A SOL manager reviews each functional PR; Luna workers implement bounded tasks.

**Goal:** Replace the complete 64,745-line TypeScript/Node product with a Rust implementation exposed as a Python library on CPython 3.10-3.14, preserve the frozen 17-tool MCP protocol, prove parity against the read-only oracle, and remove Node only after the final release gates pass.

**Architecture:** A coordinator-owned parser seam dispatches immutable byte input to independently owned Rust crates. `kordoc-hancom`, `kordoc-pdf`, and `kordoc-office` parse into `kordoc-ir`; later crates implement transforms, forms, roundtrip editing, redaction, generation, rendering, and MCP operations. PyO3 remains a thin native boundary and Python owns ergonomic objects plus the stdio protocol adapter, never document logic. The ignored `kordoc/` checkout is a read-only development oracle and is never a source, build, test, package, or runtime dependency.

**Tech Stack:** Rust 1.97, Cargo, serde, quick-xml, zip, cfb, flate2, PyO3 `abi3-py310`, PDFium, ONNX Runtime, maturin, CPython 3.10-3.14, pytest 9.0.3, Ruff, mypy, cargo-nextest/llvm-cov/fuzz/audit/deny, CodeQL, GitHub Actions.

---

## Baseline and non-negotiable contracts

The oracle snapshot has 87 value exports, 112 type exports, 17 MCP tools, 13 stable error codes, and 238 test/fixture files (219 top-level TypeScript tests). Only version metadata, bounded detection, and the generic parse failure shell are foundation-complete. The single committed golden proves detection only. No parser, OCR path, transform, renderer, generator, mutator, or MCP handler may be called complete until its full result and failure behavior pass independently licensed or generated parity fixtures.

The coordinator exclusively owns `contracts/`, `crates/kordoc-ir/`, root workspace membership and lockfile integration, `crates/kordoc-core/src/parse.rs`, the PyO3 root module, public Python exports, `docs/SSOT/migration/status.md`, and shared workflow gates. Workers may not edit those paths unless their task explicitly grants ownership. Each merged behavior PR must update its component SSOT page and append a WIKI record. Frozen wire casing, optionality, error codes, MCP schemas, limits, envelopes, and security behavior are not implementation suggestions; they are acceptance criteria. This file is the master execution DAG: before starting any P1-P17 slice, its SOL manager writes a focused child plan in this directory with exact functions, fixture cases, failing test names/output, implementation steps, verification commands, and commit boundaries. Oversized slices such as PDF layout/tables, Office's three formats, generation, and rendering are split into `a/b/c` PRs by that child plan while retaining the dependency gate here.

Every fixture manifest entry records license/provenance, a deterministic generator when applicable, input and expected-output SHA-256, covered dimensions, oracle commit/digest, and the exact capture command. Restricted corpora run only on protected runners and are reported as unavailable when absent. Allowed normalization is limited to ZIP timestamps and XML attribute order. First differences use RFC 6901 JSON pointers.

Every capability gate requires oracle parity evidence; implementation tests alone do not establish compatibility.

## Dependency and PR DAG

```text
P0 parser seam ─┬─ P7 base projections/tables ─┬─ P1 HWPX ─ P4 HWP5 ─ P10 ─ P11
                │                              ├─ P2 PDF ── P6 OCR/formula
                │                              └─ P3 Office / P5 HWP3-HWPML
                └─ P14 MCP transport

P1..P6 extend P7's fixture matrix ─┬─ P8 diff/text ─ P10 roundtrip
                                   ├─ P9 forms ───── P10
                                   └─ P12 generation ─ P13 rendering

P1..P8/P14 ─ P15 MCP reads
P9..P12/P14 ─ P16 MCP edits
P12/P13/P14 ─ P17 MCP outputs
P1..P17 ─ P18 export/package sweep ─ P19 Node removal gate
```

After P0/P7, the default four-slot wave is one coordinator, one SOL functional manager, and two Luna workers on two disjoint slices (initially HWPX and PDF). When one slice reaches manager review, its Luna slot advances to Office while the SOL manager reviews/integrates the completed slice; later waves follow the DAG. Only simple, already decomposed tasks may replace the SOL slot with a third Luna worker followed by a separate SOL review turn. Root `Cargo.toml`, `Cargo.lock`, shared dispatch, bindings, public exports, and status integration are serialized by the coordinator after worker branches are ready.

## Common red-green-review gate

Every task below follows the same sequence in addition to its focused steps:

- [ ] Add unit/property/adversarial and parity tests first; run the focused command and record the expected failure.
- [ ] Implement the smallest Rust behavior that passes, with bounded allocation, checked arithmetic, input limits, sanitized errors, and no panics on untrusted bytes.
- [ ] Run `cargo +1.97.0 fmt --all -- --check`, `cargo +1.97.0 clippy --workspace --all-targets --all-features --locked -- -D warnings`, `cargo +1.97.0 test --workspace --locked`, and `RUSTDOCFLAGS="-Dwarnings" cargo +1.97.0 doc --workspace --no-deps --locked`. On a machine whose `cargo` is not the rustup proxy, use the already verified Rust 1.97 binary without the `+1.97.0` selector.
- [ ] Build/install the wheel, then run `uv run --python 3.10 pytest tests/contracts tests/parity tests/python -q`; include `tests/mcp` once created. Run Ruff, mypy, docs, artifact, license, audit, deny, coverage, and relevant fuzz gates from `docs/SSOT/operations/development.md`.
- [ ] Append WIKI evidence, update current SSOT truth, request SOL review, push a focused branch, open a PR, wait for strict hosted CI/Security/Wheels/Fuzz/CodeQL, and squash-merge. Never push implementation directly to `main`.

Every new-crate or new-operation PR has a serialized coordinator integration checkpoint. For the parallel P1/P2 first wave, a protected coordinator scaffold registers both minimal crates, pins candidate dependencies, and regenerates `Cargo.lock` before worker branches, so every worker slice runs `--locked` workspace CI. Later new crates may use the original sequence in which the coordinator performs registration after crate-local red/green work. Format crates depend on `kordoc-ir`, never `kordoc-core`, and return an internal source-neutral `ParsedDocument` containing IR/metadata/assets/page evidence without final Markdown projection. At the capability checkpoint the coordinator alone verifies or updates root workspace membership and dependency pins, regenerates `Cargo.lock` when needed, wires `kordoc-core` dispatch and P7 projections to assemble the frozen `ParseSuccess`, adds the PyO3 binding plus Python wrapper/stub/model, assigns and updates the affected value and type entries in `contracts/public-api.json`, and adds wheel-installed Python parity tests. This dependency direction (`format -> ir`, `core -> format + ir`) forbids a Cargo cycle. Public exposure occurs only after locked workspace, wheel, and end-to-end gates pass. This keeps worker ownership disjoint without deferring usable Python APIs to P18.

Type ownership follows behavior ownership: P0 owns common parse/result/options/document types; P1/P2/P3/P4/P5/P6 own their format and OCR option/result types; P7 owns pages/chunks/table projection types; P8 owns diff/splice/metric types; P9 owns recognition and form-preserving types; P10 owns sessions/patch types; P11 owns redaction types; P12 owns generation/profile/lint types and the completed top-level fill result; P13 owns scene/render/region/print types; P14-P17 own MCP-only adapter types. Each PR changes its assigned `type_entries` from `planned` only when its Python model, stub, serialization, and installed-wheel tests pass. P18 is an exact zero-remaining audit, not the first implementation of these 112 mappings.

### Task P0: Successful parser seam, options, and production parity harness

**Branch:** `feature/parser-seam`  
**Files:** Create `crates/kordoc-ir/src/parsed.rs`, `crates/kordoc-core/src/{parse,options}.rs`, `tests/parity/test_document_goldens.py`, `tests/python/test_parse_success.py`, and a synthetic parser fixture builder; modify IR/core/PyO3/Python facade and model files, golden manifest, architecture, Python contract, parity policy, status, and WIKI.

- [x] Write failing Rust tests for registered parser dispatch, exact option translation, successful recursive `ParseResult` JSON roundtrip, unsupported option rejection, and panic containment.
- [x] Write failing Python tests for immutable successful `Document`, bytes/path/file-like equivalence, exception versus `try_parse` result semantics, binary images as `bytes`, omitted-versus-null fields, and GIL-released native calls.
- [x] Add coordinator-reviewed `Document` and successful result entries to the Python-only manifest, because the approved API requires `result.document` but the foundation manifest currently inventories only failure-shell models. Update contract tests and Python API SSOT in the same commit before exposing the class.
- [x] Define and roundtrip-test internal `kordoc_ir::ParsedDocument`, containing blocks plus source metadata/assets/page evidence but no projected Markdown. It is the acyclic return DTO for every format crate and is not a new public wire type.
- [x] Add a `Parser`/registry seam accepting `&[u8]`, detected `FileType`, bounded `ParseOptions`, and returning `ParsedDocument`; `kordoc-core` alone projects it into the frozen public `ParseSuccess`. Include an optional bounded metadata-only operation: HWPX, HWP5, and PDF implement it; other formats may use the oracle-compatible full-parse fallback only after the MCP layer enforces its 50 MiB limit. A test-only parser proves the success path without claiming a real format.
- [x] Extend the golden harness to compare full recursive IR, markdown, pages, metadata, outline, images, warnings, quality, errors, and option behavior; emit an RFC 6901 first-difference path.
- [x] Keep every real parser and all MCP tools `planned`; do not edit frozen schema content.
- [x] Assign and test shared detector helper exports here: `detect_ole2_format`, `detect_zip_format`, `is_hwpx_file`, `is_old_hwp_file`, `is_pdf_file`, and `is_zip_file`.
- [x] Focused commands: `cargo +1.97.0 test -p kordoc-core --locked parse` and `uv run --python 3.10 pytest tests/parity/test_document_goldens.py tests/python/test_parse_success.py -q`.

### Task P1: HWPX parser

**Branch:** `feature/parse-hwpx` — depends on P7  
**Files:** Create `crates/kordoc-hancom/src/hwpx/{mod,xml,sections,styles,tables,images,metadata,crypto}.rs`, colocated module tests plus `crates/kordoc-hancom/tests/hwpx_integration.rs`, HWPX goldens, and `docs/SSOT/components/hwpx.md`.

Execution is governed by the focused [P1 HWPX implementation plan](2026-09-30-hwpx-implementation-plan.md). Its stricter package-security decisions and serialized coordinator integration checkpoints are normative for this task.

- [ ] Test ZIP manifest/multi-section order, namespace-local names, paragraphs/spans, headings/outlines, nested tables/captions/spans, notes, images, metadata, page cache/fallback, warnings, and deterministic Markdown.
- [ ] Test traversal names, DTD/entities, XML depth, 500-entry and 256 MiB expansion limits, corrupt ZIP recovery, missing sections, encrypted ODF AES-256-CBC password/no-password/wrong-password, and partial-parse preservation.
- [ ] Implement bounded package access and streaming XML lowering; no Windows COM fallback enters the portable product.
- [ ] Implement and expose `parse_hwpx` plus the format-specific metadata-only path; add exact public signature and result tests.
- [ ] Own and expose `validate_hwpx` here; test required package entries, XML/package integrity, limits, encrypted inputs, and deterministic diagnostics before the public API status changes.
- [ ] Add parser and ZIP/XML fuzz targets; gate recursive IR and normalized ZIP/XML parity.
- [ ] Focused commands: `cargo +1.97.0 test -p kordoc-hancom --lib --locked` and `cargo +1.97.0 test -p kordoc-hancom --test hwpx_integration --locked`.

### Task P2: PDF text, layout, and tables

**Branch:** `feature/parse-pdf` — depends on P7  
**Files:** Create `crates/kordoc-pdf/src/{parser,document,layout,table,text,image,links,quality}.rs`, `crates/kordoc-pdf/tests/pdf_*`, PDF goldens, and `docs/SSOT/components/pdf.md`.

Execution is governed by the focused [P2 PDF implementation plan](2026-09-30-pdf-implementation-plan.md). Its pure-Rust semantic backend decision and optional raster-only PDFium boundary are normative for this task.

- [ ] Test page geometry, glyph ordering, CJK spacing, vertical/two-column/two-up text, headings, foot/endnotes, links, images, quality, page ranges, and image-based PDF failure semantics.
- [ ] Port table detection as small tested modules: ruled/borderless grids, cells, clips, bands, continuations, nested forms, headers, contacts, and text-box tables.
- [ ] Implement and expose `parse_pdf` plus metadata-only extraction. Keep PDFium strictly behind the approved rendering/raster/OCR boundary; semantic PDF parsing, IR policy, limits, and orchestration remain Rust-owned. Bundle and license the native runtime per wheel, release the GIL, cap pages/objects/bitmaps, and sanitize native failures.
- [ ] Add malformed-object, recursion, huge-page, decompression, and native boundary fuzz/property cases; record throughput and RSS against the oracle.
- [ ] Focused command: `cargo +1.97.0 test -p kordoc-pdf --locked`.

### Task P3: Office DOCX/XLSX/XLS parsers

**Branch:** `feature/parse-office` — depends on P7  
**Files:** Create `crates/kordoc-office/src/docx/{mod,relationships,styles,numbering,paragraph,tables,images,equations}.rs`, `src/spreadsheet/{mod,shared,xlsx,sheet}.rs`, `src/spreadsheet/xls/{mod,record,sst,cell,encoding}.rs`, Office tests/goldens, and `docs/SSOT/components/office.md`.

- [ ] DOCX tests cover relationships, styles/numbering, hyperlinks, OMML, footnotes, controls, nested tables, images, metadata, missing required parts, traversal targets, DTD/depth, and 100 MiB expansion.
- [ ] XLSX tests cover rich/shared strings, styles, formulas, merged ranges, 1900/1904 dates, sparse coordinates through row 1,048,576, relationship confinement, 100 sheets, 200 columns, and the oracle-compatible dynamic 2,000,000-cell output budget (`sheetRowCap(cols)`), not a fixed 10,000-row cut.
- [ ] XLS BIFF8 tests cover BoundSheet offsets, SST/CONTINUE Unicode switches, code pages, RK/MULRK/numbers/formulas/dates/merges, corrupt chains, encryption, and explicit BIFF5 rejection.
- [ ] Add OOXML/BIFF fuzz targets. Detection-only PPTX remains unsupported and documented.
- [ ] Implement and expose `parse_docx`, `parse_xlsx`, and `parse_xls`; their metadata path may use the bounded full-parse fallback. Exact API tests cover each name.
- [ ] Focused command: `cargo +1.97.0 test -p kordoc-office --locked`.

### Task P4: HWP 5/CFB parser

**Branch:** `feature/parse-hwp5` — depends on P1  
**Files:** Create `crates/kordoc-hancom/src/{cfb,hwp5/mod,hwp5/record,hwp5/body,hwp5/assemble,hwp5/crypto,hwp5/images,hwp5/pages}.rs`, HWP5 tests/goldens, and `docs/SSOT/components/hwp5.md`.

- [ ] Test strict and lenient CFB, compressed/uncompressed record streams, metadata, styles/numbering, controls, recursive tables, pages, images, summary info, distribution/DRM sentinels, and partial recovery.
- [ ] Bound sections to 100 and enforce the oracle-compatible 100 MiB cumulative decompressed budget across all section streams; test cycles, overlaps, truncated chains, record-length overflow, password/no-password/wrong-password, and decompression bombs.
- [ ] Preserve MIT/BSD-3-Clause attribution for behavior derived from rhwp/volexity or reimplement independently; pass `cargo deny` before merge.
- [ ] Implement and expose `parse_hwp` with a format-specific metadata-only path and exact API tests.
- [ ] Focused command: `cargo +1.97.0 test -p kordoc-hancom --test hwp5_ir --test hwp5_security --locked`.

### Task P5: HWP 3 and HWPML parsers

**Branch:** `feature/parse-hwp3-hwpml` — depends on P7  
**Files:** Create `crates/kordoc-hancom/src/hwp3/{mod,header,reader,johab,controls,tables,crypto}.rs`, `src/hwpml.rs`, tests/goldens, and component pages.

- [ ] Test HWP3 signature/header, raw deflate, Johab and symbols, controls, drawings, tables, notes, headers/footers, outline numbering, DES password behavior, truncation, and 100 MiB expansion.
- [ ] Test HWPML BOM/UTF-8, namespaces, paragraphs/spans, nested tables, headings, malformed XML partial warning, empty body success, DTD/entities, depth, 50 MiB input, and row/column bounds.
- [ ] Carry required third-party attribution; add record/control/XML fuzz targets.
- [ ] Implement and expose `parse_hwp3` and `parse_hwpml`; their metadata path may use the bounded full-parse fallback. Add exact API tests.
- [ ] Focused command: `cargo +1.97.0 test -p kordoc-hancom --test hwp3_ir --test hwpml_structure --locked`.

### Task P6: Image OCR and formula recognition

**Branch:** `feature/ocr-formula` — depends on P2  
**Files:** Create `crates/kordoc-ocr/src/{lib,engine,image,pdf,crop,deskew,lines,postprocess,glyphs,models}.rs`, `crates/kordoc-pdf/src/formula/`, tests/goldens/model metadata, and `docs/SSOT/components/ocr.md`.

- [ ] Define safe OCR/model traits and test image parse, PDF automatic/explicit OCR, crops, deskew, ruling-line removal, line splitting, glyph restoration, formula detection/recognition/postprocess, cancellation, and progress.
- [ ] Implement and expose `parse_image` with exact API, limit, and error tests.
- [ ] Integrate ONNX Runtime through Rust; verify signed/checksummed versioned model bundles, offline resolution, architecture compatibility, bounded tensors/images, and deterministic missing-dependency errors.
- [ ] Record accuracy, latency, peak RSS, and a 10% regression gate; test malicious images/models and native failures without leaking paths.
- [ ] Focused command: `cargo +1.97.0 test -p kordoc-ocr --locked`.

### Task P7: Shared projections, page/chunk output, and table policy

**Branch:** `feature/ir-projections` — depends on P0 and merges before P1-P6  
**Files:** Create `crates/kordoc-core/src/{markdown,pages,chunks,table/mod,table/builder,table/classifier,table/visual}.rs`, projection tests, and `docs/SSOT/components/normalization.md`.

- [x] Test `blocks_to_markdown`, `blocks_to_pages`, `blocks_to_chunks`, `classify_table`, `classify_table_tree`, `collect_table_blocks`, `choose_table_representation`, `flatten_layout_tables`, and `has_structured_cell_content` across recursive/nested IR.
- [x] Implement one shared, tested HTML/GFM Markdown table-unit parser here for both roundtrip patching and HWPX generation; neither downstream crate may fork it.
- [x] Preserve ordering, escaping, empty blocks, page ranges, chunk overlap/limits, HTML table safety, label/layout/data classification, and default-output stability.
- [x] First prove the projections with synthetic recursive IR and merge them so every parser can populate mandatory Markdown/pages without a dependency cycle. Each P1-P6 PR then adds its fixtures to the same projection matrix without changing projection policy. Merged in PR #11 as `d473bae79f1de6f1aced29c59609a2cb49f0aa90`; all 33 hosted checks passed. P1-P6 are unblocked. See the [P7 merge evidence](../../../WIKI/2026/09/2026-09-30-ir-projections-merge.md).
- [x] Keep projection calls in `kordoc-core` after dispatch. Format crates return only `kordoc-ir`'s internal `ParsedDocument`; they never depend on or call `kordoc-core`.
- [x] Assign `is_label_cell` here with exact table-classification API tests. Property-test recursion and arbitrary Unicode.
- [x] Focused command: `cargo +1.97.0 test -p kordoc-core --test projections --locked`.

P7's source-neutral implementation passed review and merged in PR #11 as `d473bae79f1de6f1aced29c59609a2cb49f0aa90`. All 33 hosted checks passed, including CI run 36622144159, Fuzz run 36622144214, Wheels run 36622144316, and Security/CodeQL run 36622144308. Local evidence includes six exact pinned runtime-oracle generated-IR captures, 91 core unit and 20 all-feature integration tests, 8 native tests, 124 combined Python/parity tests, two bounded proptests, and strict Clippy/Ruff/mypy checks. Dedicated Markdown-unit and projection fuzz targets are part of the four-target CI matrix. The successful real-document parser numerator remains zero. P1-P6 are now unblocked and must add real-source fixtures to the projection matrix; parser parity remains pending until then. See the append-only [P7 merge evidence](../../../WIKI/2026/09/2026-09-30-ir-projections-merge.md).

### Task P8: Diff, splices, text redaction, and metric utilities

**Branch:** `feature/pure-transforms` — depends on P7  
**Files:** Create `crates/kordoc-transform/src/{lib,diff,text,splices,metrics}.rs`, tests, and `docs/SSOT/components/transformations.md`.

- [ ] Test `compare`, `diff_blocks`, `apply_splices`, `build_paragraph_splices`, `build_range_splices`, `redact_text`, `redact_markdown`, and stable diff ordering.
- [ ] Test `char_width_em1000`, `measure_text_width`, `simulate_wrap`, `simulate_wrap_keep_word`, `fit_ratio_for_fewer_lines`, `is_known_font`, `unknown_font_warnings`, `SPACE_EM_FIXED`, and `SPACE_EM_FONT`.
- [ ] Cover overlapping/out-of-bounds splices, Unicode graphemes, regex denial-of-service, deep IR, and deterministic output.
- [ ] Focused command: `cargo +1.97.0 test -p kordoc-transform --locked`.

### Task P9: Forms, seals, and built-in templates

**Branch:** `feature/forms` — depends on P1 and P7  
**Files:** Create the independently owned `crates/kordoc-forms/src/{lib,recognize,match,fill,hwpx,seal,templates}.rs`, licensed template assets, tests/goldens, and `docs/SSOT/components/forms.md`.

- [ ] Test `ValueCursor`, `extract_click_here_fields`, `extract_form_fields`, `extract_form_schema`, `infer_field_type`, `format_fill_value`, `fill_form_fields`, `fill_with_unique_guard`, `fill_hwpx`, and `place_seal_hwpx`.
- [ ] Test ambiguity, repeated/multivalue fields, XML escaping, click-here controls, style preservation, image confinement, same/output paths, and secure temporary output.
- [ ] Implement `BUILTIN_TEMPLATES`, `read_builtin_template`, `read_builtin_template_sample`, and `resolve_builtin_template` only with redistributable assets and recorded SHA-256/license.
- [ ] Keep the top-level `fill_form(..., output_format="hwpx")` generation path pending here: it calls `markdown_to_hwpx` and therefore completes in P12. P9 covers recognition, IR filling, and source-preserving HWPX fill only.
- [ ] Focused command: `cargo +1.97.0 test -p kordoc-forms --locked`.

### Task P10: HWPX/HWP roundtrip sessions and patching

**Branch:** `feature/roundtrip` — depends on P1, P4, P7, and P8  
**Files:** Create the independently owned `crates/kordoc-roundtrip/src/{lib,session,source_map,zip_patch,ole_patch,table}.rs`, tests/goldens, and `docs/SSOT/components/roundtrip.md`.

- [ ] Test `HwpxSession`, `open_hwpx_document`, `scan_section_xml`, `patch_hwpx`, `patch_hwpx_blocks`, and `patch_hwp` with exact source maps, whitespace, notes, nested tables, row insertion, and empty cells.
- [ ] Reject unsupported structural changes explicitly; perform checked atomic output, preserve untouched entries/streams, reparse results, and verify source integrity.
- [ ] Test malformed source maps, overlapping edits, traversal, symlink races, same-file handling, and ZIP/OLE output limits.
- [ ] Focused command: `cargo +1.97.0 test -p kordoc-roundtrip --locked`.

### Task P11: Whole-document redaction

**Branch:** `feature/redaction` — depends on P8 and P10  
**Files:** Create the independently owned `crates/kordoc-redact/src/{lib,rules,hwpx,hwp5,doc,scrub,name_address}.rs`, tests/goldens, and `docs/SSOT/components/redaction.md`.

- [ ] Implement and test `DEFAULT_REDACT_RULES`, nested body redaction, metadata/preview/summary scrubbing, name/address rules, dry-run review, exact counts, and sanitized reporting.
- [ ] Require explicit same-file rejection, atomic output, encrypted/DRM behavior, recursive residual sensitive-byte scan, and reparse validation.
- [ ] Property/fuzz test rules and containers; prove no original secret remains in supported output surfaces.
- [ ] Focused command: `cargo +1.97.0 test -p kordoc-redact --locked`.

### Task P12: HWPX generation, profiles, equations, and government documents

**Branch:** `feature/generate-hwpx` — depends on P1, P7, and P9  
**Files:** Create the independently owned `crates/kordoc-generate/src/` modules for package/header/section/page/table/image/equation/profile/presets/gongmun/gaejosik/lint, tests/goldens, and `docs/SSOT/components/generation.md`.

- [ ] Test `markdown_to_hwpx`, `hwpx_to_profile`, `PRESET_ALIAS`, `normalize_gongmun_preset`, profile roundtrip, headings/lists/tables/images/charts/equations, IDs/styles, line fitting, and deterministic packages.
- [ ] Complete and expose top-level `fill_form`, including its generated-HWPX output path, by composing the already merged P9 matching/filling operations with `markdown_to_hwpx`; add cross-crate parity tests here.
- [ ] Test `lint_gongmun_text`, `gongmun_lint_warnings`, `incompatible_gongmun_warnings`, `lint_munche_text`, `munche_lint_warnings`, and `uses_gaejosik_munche` against Korean document fixtures.
- [ ] Confine image reads, cap package sizes/counts, normalize ZIP/XML for structural comparison, and require generated-document reparse parity. Preserve Apache-2.0 equation attribution if implementation is derived.
- [ ] Focused command: `cargo +1.97.0 test -p kordoc-generate --locked`.

### Task P13: Layout, rendering, regions, PDF/SVG/raster, and print

**Branch:** `feature/rendering` — depends on P1, P4, and P12  
**Files:** Create `crates/kordoc-render/src/{lib,scene,layout,reflow,document,html,svg,pdf,raster,regions,hwp5,print}.rs`, visual tests/baselines, and `docs/SSOT/components/rendering.md`.

- [ ] Test `render_document`, `render_document_to_scene`, `render_html`, `render_scene_to_html`, `render_hwpx_to_svg`, `render_hwp5_pages`, `extract_rendered_regions`, `blocks_to_pdf`, and `markdown_to_pdf`.
- [ ] Implement Rust-owned `extract_tables` orchestration and its Python export by composing P7 classification with shared scene/region geometry and bounded crop/report output. P17 only formats this native result for MCP.
- [ ] Validate cached-layout versus reflow, typography, nested tables/images/equations, page geometry, crop coordinates, output formats, transparent backgrounds, and print behavior.
- [ ] Bound canvas dimensions/pixels/pages, isolate native raster/PDF failures, compare geometry plus approved perceptual hashes, and record latency/RSS.
- [ ] Focused command: `cargo +1.97.0 test -p kordoc-render --locked`.

### Task P14: MCP stdio transport and safe file layer

**Branch:** `feature/mcp-transport` — depends on P0 and may run beside parsers  
**Files:** Create `crates/kordoc-mcp/` for operation-neutral protocol types/security helpers, `python/kordoc/mcp/{__init__,server,paths,protocol}.py`, `tests/mcp/test_{transport,paths,detect}.py`, entry point, and `docs/SSOT/components/mcp-server.md`.
**Assigned MCP tools:** `detect_format`

- [ ] Test exact initialize/tools-list framing, import safety, idempotent startup/shutdown, cancellation, concurrent requests, stdout protocol-only discipline, stderr logging, and sanitized errors.
- [ ] Enforce nonempty `KORDOC_ROOT` confinement, independent `KORDOC_OFFLINE`, canonical input paths, traversal/symlink rejection, output ancestor and leaf rechecks, no-follow/atomic creation, extension allowlists, and generation image confinement.
- [ ] Make only `detect_format` live initially; it reads 512 bytes first and full content only for bounded ZIP/OLE inspection. All other handlers remain explicitly unavailable, not fake successes.
- [ ] In the same coordinator integration commit, change only `detect_format` from `planned` to implemented, update the all-planned contract assertion to an exact per-tool status assertion, and update migration status. A listed tool and a live tool remain distinct states.
- [ ] Focused commands: `cargo +1.97.0 test -p kordoc-mcp --locked` and `uv run --python 3.10 pytest tests/mcp/test_transport.py tests/mcp/test_paths.py tests/mcp/test_detect.py -q`.

### Task P15: MCP read and comparison tools

**Branch:** `feature/mcp-read-tools` — depends on P1-P8 and P14  
**Files:** Create `python/kordoc/mcp/tools_parse.py`, `tools_compare.py`, and `tests/mcp/test_read_tools.py`; update MCP SSOT/status/WIKI.
**Assigned MCP tools:** `parse_document`, `parse_metadata`, `parse_pages`, `parse_table`, `compare_documents`, `parse_chunks`

- [ ] Implement exact frozen schemas/envelopes for `parse_document`, `parse_metadata`, `parse_pages`, `parse_table`, `compare_documents`, and `parse_chunks` over the same native operations as Python.
- [ ] Preserve 500 MiB document input, 50 MiB metadata input, option/default behavior, page/table indexing, 200,000-character truncation only where contracted, errors, progress, and output-file rules.
- [ ] Run exact tools/list snapshots, bytes-versus-path parity, all-format success/failure fixtures, truncation boundaries, and concurrency/cancellation tests.

### Task P16: MCP form and edit tools

**Branch:** `feature/mcp-edit-tools` — depends on P9-P12 and P14  
**Files:** Create `python/kordoc/mcp/tools_forms.py`, `tools_edit.py`, and `tests/mcp/test_edit_tools.py`; update MCP SSOT/status/WIKI.
**Assigned MCP tools:** `parse_form`, `fill_form`, `place_seal`, `patch_document`, `redact_document`

- [ ] Implement `parse_form`, `fill_form`, `place_seal`, `patch_document`, and `redact_document` with exact schemas, defaults, envelopes, allowlists, and native errors.
- [ ] Test unique guards, image confinement, atomic output, symlink races, overwrite policy, redaction-only same-file rejection, reparsing, residual PII, and the contracted redaction response truncation.

### Task P17: MCP render, crop, table, profile, and generation tools

**Branch:** `feature/mcp-output-tools` — depends on P12-P14  
**Files:** Create `python/kordoc/mcp/tools_render.py`, `tools_generate.py`, and `tests/mcp/test_output_tools.py`; update MCP SSOT/status/WIKI.
**Assigned MCP tools:** `render_document`, `crop_regions`, `extract_tables`, `extract_profile`, `generate_document`

- [ ] Implement `render_document`, `crop_regions`, `extract_tables`, `extract_profile`, and `generate_document` with exact output-file behavior and envelopes.
- [ ] Test all formats, page/region bounds, maximum eight render/table crop images, binary result metadata, profile fidelity, generation input/image confinement, offline behavior, and deterministic sanitized failures.

### Task P18: Audit Python exports/types and complete wheel/release qualification

**Branch:** `release/product-parity` — depends on P1-P17  
**Files:** Modify Python facade/stubs/models, PyO3 bindings, public manifests/status, tests, wheel/release workflows, SBOM/provenance/license files, and operations docs.

- [ ] Audit that the following frozen values are already real, tested Python exports from their owning PR, or stop for a coordinator-reviewed contract correction: `BUILTIN_TEMPLATES`, `DEFAULT_REDACT_RULES`, `HwpxSession`, `PRESET_ALIAS`, `SPACE_EM_FIXED`, `SPACE_EM_FONT`, `__version__`, `ValueCursor`, `apply_splices`, `blocks_to_chunks`, `blocks_to_markdown`, `blocks_to_pages`, `blocks_to_pdf`, `build_paragraph_splices`, `build_range_splices`, `char_width_em1000`, `choose_table_representation`, `classify_table`, `classify_table_tree`, `collect_table_blocks`, `compare`, `detect_format`, `detect_ole2_format`, `detect_zip_format`, `diff_blocks`, `extract_click_here_fields`, `extract_form_fields`, `extract_form_schema`, `extract_rendered_regions`, `extract_tables`, `fill_form`, `fill_form_fields`, `fill_hwpx`, `fill_with_unique_guard`, `fit_ratio_for_fewer_lines`, `flatten_layout_tables`, `format_fill_value`, `gongmun_lint_warnings`, `has_structured_cell_content`, `hwpx_to_profile`, `incompatible_gongmun_warnings`, `infer_field_type`, `is_hwpx_file`, `is_known_font`, `is_label_cell`, `is_old_hwp_file`, `is_pdf_file`, `is_zip_file`, `lint_gongmun_text`, `lint_munche_text`, `markdown_to_hwpx`, `markdown_to_pdf`, `measure_text_width`, `munche_lint_warnings`, `normalize_gongmun_preset`, `open_hwpx_document`, `parse`, `parse_docx`, `parse_hwp`, `parse_hwp3`, `parse_hwpml`, `parse_hwpx`, `parse_image`, `parse_pdf`, `parse_xls`, `parse_xlsx`, `patch_hwp`, `patch_hwpx`, `patch_hwpx_blocks`, `place_seal_hwpx`, `read_builtin_template`, `read_builtin_template_sample`, `redact_markdown`, `redact_text`, `render_document`, `render_document_to_scene`, `render_html`, `render_hwp5_pages`, `render_hwpx_to_svg`, `render_scene_to_html`, `resolve_builtin_template`, `scan_section_xml`, `simulate_wrap`, `simulate_wrap_keep_word`, `unknown_font_warnings`, `uses_gaejosik_munche`, and `validate_hwpx`.
- [ ] Audit that all 112 frozen type mappings and Python-only result/error models were materialized and tested by their owning PRs; fail if any remains `planned`. Re-run `stubtest`, mypy consumer fixtures, signature/default snapshots, serialization roundtrips, and public `__all__` inventory tests without adding first-time feature implementations here.
- [ ] Mark all 17 MCP tools implemented only after their task tests pass; validate their inventory exactly once.
- [ ] Build and native-smoke six abi3 wheel targets on CPython 3.10 and 3.14; run the supported-version API/MCP matrix for every 3.10-3.14 minor. Verify bundled PDFium/ONNX libraries, model bundle, auditwheel/delocate equivalents, licenses, SBOMs, checksums, attestations, wheel-size/import-latency/performance budgets, and absence of `kordoc/` or Node assets.
- [ ] Run one full release-candidate workflow from the immutable main commit, fix only through PRs, then run a second consecutive full release-candidate workflow on the same commit.

### Task P19: Node removal gate

**Branch:** `cleanup/remove-node` — depends on P18 and two consecutive full release-candidate workflows  
**Files:** Dedicated removal-only PR touching tracked Node/npm references, manifests, workflows, root docs, compatibility disposition, status, and WIKI.

- [ ] Prove every value/type/tool classification and every representative end-to-end Python/MCP workflow; confirm no unclassified fuzz crash, parity delta, security finding, unsupported wheel, or unexplained >10% regression remains.
- [ ] Search tracked files, built wheels/sdists, SBOMs, workflows, docs, and runtime processes for Node/npm/npx/tsx/TypeScript dependencies. Remove only obsolete Node library/general CLI machinery; preserve the Python `kordoc-mcp` entry point.
- [ ] Re-run the complete local and hosted gates and artifact scans. The ignored oracle remains local and ignored; it is never committed, packaged, uploaded, or required by CI.
- [ ] Merge only after SOL manager and coordinator approval and all protected checks pass. This is the Node removal gate; no earlier PR may perform it.

## Completion definition

The port is complete only when P0-P19 are merged through GitHub Flow; every frozen public value/type and all 17 MCP tools have passing behavior, error, security, and parity evidence; every native wheel installs on its target; CPython 3.10-3.14 passes; two consecutive full release-candidate workflows pass on the same immutable commit; and the dedicated Node-removal PR passes all protected checks. A plan checkbox, compiled crate, listed handler, synthetic detector case, or locally green subset is never sufficient by itself.
