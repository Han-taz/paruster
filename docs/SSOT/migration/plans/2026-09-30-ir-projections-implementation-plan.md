# P7 Shared Projections Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use `superpowers:subagent-driven-development` or `superpowers:executing-plans` task by task, `superpowers:test-driven-development` for behavior, and `superpowers:verification-before-completion` before commits. This plan is an implementation handoff, not a claim of completed parity.

**Goal:** Produce deterministic Markdown, pages, chunks, table construction/classification, and shared Markdown table units from source-neutral IR, then expose the assigned Python values and models.

**Architecture:** `kordoc-core` owns projection after `kordoc-ir::ParsedDocument` returns from the private parser registry. Format crates depend on `kordoc-ir` only and never call `kordoc-core`. P7 keeps rendering policy separate from semantic table classification: normal parse output stays stable; visual crops require the later scene/region integration. The coordinator serializes changes to shared contracts, IR types, parser dispatch, binding roots, and manifest statuses.

**Tech Stack:** Rust 1.97, serde, proptest, PyO3 0.29, Python 3.10, pytest, maturin, Ruff, mypy.

---

## Preconditions and ownership

- Base this branch on merged P0. The current `feature/ir-projections` checkout has P0's `ParsedDocument`, `ParseOptions`, private registry, and test-only successful projector; verify those interfaces against `main` immediately before implementation. Do not make a parallel P0 edit.
- SOL P7 manager owns `crates/kordoc-core/src/{markdown,pages,chunks,table/**,markdown_units}.rs`, `crates/kordoc-core/tests/projections.rs`, and `docs/SSOT/components/normalization.md`. Workers receive disjoint files only. No worker edits `contracts/`, `crates/kordoc-ir/`, `crates/kordoc-core/src/{parse,lib}.rs`, `Cargo.toml`, `Cargo.lock`, `crates/kordoc-python/`, `python/kordoc/`, or `docs/SSOT/migration/status.md`.
- Coordinator integration owns the frozen schema/API review, `ParseSuccess` assembly in `parse.rs`, core exports, PyO3/Python facade and stub, `contracts/public-api.json` dispositions, package tests, SSOT status, and lockfile if dependencies change. Changes to `ir-schema.json`, error codes, or MCP schemas require explicit coordinator approval. Reuse existing IR types where possible.
- This plan does not move `extract_tables` into P7. P7 owns classification and crop selection policy in `table/visual.rs`; P17 composes that policy with the future Rust scene/region geometry, bounded crop bytes, and Python operation. The oracle's `extractTables` currently calls parse then render; copying that dependency into P7 would create a cycle or premature renderer coupling.
- P7 exports the single Rust Markdown table-unit reader as `kordoc_core::markdown_units` for P8, P10, and P12. Those downstream crates may depend on core; core must not depend on them. If a future core-to-transform dependency becomes necessary, the coordinator moves this same module into one leaf crate before that dependency is added. Consumers cannot copy the parser into their own modules.

## Oracle behavior and evidence map

The ignored, read-only `/Users/shkoh/Projects/paruster/kordoc/` is research evidence only. Never import its files, read it in CI, package it, or commit it. Capture expected values into independently licensed/generated test fixtures with provenance, hashes, oracle commit and source digest, and exact capture command before claiming oracle parity. Existing `tests/golden/document-manifest.json` has zero successful oracle document cases at P0; synthetic IR tests establish correctness of the new projection functions but do not increase that numerator.

The inspected oracle revision is `bb71f7fb0bf51dd456d27505a8c04772df182144`. Pin source SHA-256 in fixture records; examples: `src/chunks.ts` = `8de6844ea9c48f242cd0f5f136673eb5e8d93af44616953b9dbf02ae2d890056`, `src/page-markdown.ts` = `32019305ed8f4f41efc3a53ee4724c30685382c10a9681f8ef4caeadab3607d2`, `src/table/builder.ts` = `062aa444cef210dfeb674d52a8de01158612ca89cfbd747c8077de63941da021`, `src/table/classifier.ts` = `45145c852fd32217ad05c49c7efd286b8c3dea83ae5660d657d56d097b8764cd`, and `src/roundtrip/markdown-units.ts` = `b2f5d3681203b1b7ae6712dd12125808f01954fe2aaa5da3e25f9cabdf4a1d15`. Recheck them at capture time; a changed oracle revision requires new capture metadata.

Generate small MIT-compatible IR inputs in a committed fixture generator under `tests/golden/document/generate_projection_inputs.py` and commit only generated input/expected JSON plus manifest, never oracle code. Required cases are `projection-ordered-blocks` (headings, lists, image, unsafe link, footnote and PUA), `projection-pages-gap` (leading/middle unnumbered and page 1/3 gap), `projection-chunks-tree` (two heading levels, list depth, standalone table, empty block), `projection-table-merged` (nested/merged HTML), `projection-classifier-matrix` (dense, sparse diagram, wrapper, uncertain), and `projection-html-units` (nested table plus escaped GFM pipe). Capture each oracle answer using a pinned, human-reviewed command saved verbatim in its manifest entry. For example, with the generated JSON at a confined path:

```bash
cd /Users/shkoh/Projects/paruster/kordoc
node --import tsx --input-type=module -e 'import { readFileSync } from "node:fs"; import { blocksToMarkdown } from "./src/table/builder.ts"; const blocks = JSON.parse(readFileSync(process.argv[1], "utf8")); process.stdout.write(JSON.stringify({markdown: blocksToMarkdown(blocks)}));' /Users/shkoh/Projects/paruster-worktrees/ir-projections/tests/golden/document/fixtures/projection-ordered-blocks.json
```

The capture is a local development action. Its committed expected JSON is evaluated in CI without Node or the oracle. Compute and record input/expected SHA-256 after capture; compare full output with no normalization unless an exact permitted ZIP/XML pointer applies. These synthetic source-contract cases remain outside the document-parser oracle numerator until a real document parser independently reproduces them.

| Rust owner | Oracle source and exact behavior to preserve | Primary oracle tests |
| --- | --- | --- |
| `markdown.rs` | `src/table/builder.ts:escapeGfm`, `sanitizeText`, `spansToMarkdown`, `blocksToMarkdown`, caption and `tableToMarkdown`: type/order-specific rendering; heading fallback level 2 and cap 6; footnotes, links with safe schemes, lists, image/separator, PUA/special text, GFM escaping, HTML for merged/structured tables, 1×1/one-column text paths. | `table-builder.test.ts`, `table-builder-integrity.test.ts`, `html-tables.test.ts`, `html-table-escape.test.ts`, `list-bullet-markdown.test.ts`, `table-cell-image.test.ts` |
| `pages.rs` | `src/page-markdown.ts:blocksToPages`: no numbered block means omitted `pages`; leading unnumbered blocks take first real page; later unnumbered blocks follow the preceding page; emit empty entries between observed min/max; preserve original block order within each page; accept format-specific finalizer only at coordinator integration. `PageEvidence` may account for truly empty source pages after coordinator-reviewed semantics, without inventing a page for a pageless format. | `page-markdown.test.ts` |
| `chunks.rs` | `src/chunks.ts:blocksToChunks`: section default merges consecutive text with identical heading/list breadcrumb, block mode emits one nonempty block each; tables stand alone; heading/list stack and marker heuristic; `c0001` IDs, inclusive top-level `blockRange`, first page in a merged run, optional raw cell matrix. Empty blocks do not participate. Oracle explicitly has **no token limit or overlap splitting**. Keep that semantic; enforce bounded output at API/MCP boundaries, not by silently splitting chunks. | `chunks-core.test.ts` |
| `table/builder.rs` | `src/table/builder.ts:buildTable`, `buildTableDirect`, `trimAndReturn`, `flattenLayoutTables`, `hasStructuredCellContent`: 200-column, default 10,000-row and 2,000,000-cell budgets; addressed/implicit anchors, bounded spans, collision text preservation, anchored empty trailing columns, intentional HWP3/HWP5-only layout flattening. Replace oracle `WeakSet` exclusion with explicit internal parser evidence, without adding a wire field. | `table-grid-split.test.ts`, `table-builder-integrity.test.ts`, `flatten-nested-table.test.ts` |
| `table/classifier.rs` | `src/table/classifier.ts:classifyTable`, `collectTableBlocks`; `src/table/analyze.ts:classifyTableTree`, `chooseTableRepresentation`: score order, two-decimal rounding, exact reason order, structural keyword gate 0.3, `semantic >= .55` and margin `.2`, `nonTabular >= .45` and margin `.15`; nested/child/caption DFS; classification opt-in; visual only for explicitly smart non-tabular. | `table-classifier.test.ts`, `table-analysis.test.ts`, `deep-nested-table.test.ts` |
| `table/label.rs` | `src/form/recognize.ts:isLabelCell`: Korean/English keyword and short-label heuristics; reject numeric-unit values, sentences, and company prefixes; strip superscript/footnote markers. P9 consumes this helper without copying it. | Form label cases in `form` oracle tests; add isolated P7 matrix. |
| `markdown_units.rs` | `src/roundtrip/markdown-units.ts:splitMarkdownUnits`, `parseGfmTable`, `parseHtmlTable`, `htmlCellInnerToLines`, `splitCellByTopLevelTables`: a single shared reader for P10 roundtrip and P12 HWPX generation. Preserve escaped pipes, `<br>`, nesting/row spans and cell ordering; reject malformed/deep/oversized HTML without evaluating scripts, entities, or remote resources. P10/P12 may add adapters, not forks. | `roundtrip-table-rows.test.ts`, `roundtrip-table-insert.test.ts`, `hwpx-html-table.test.ts`, `gen-table-grid.test.ts` |

The wire definitions already exist in `contracts/ir-schema.json` (`PageMarkdown`, `DocChunk`, `ChunkOptions`, `ClassifyContext`, `TableClassificationSummary`, `TableRepresentation`, `IRTable`), and P7-assigned public entries are in `contracts/public-api.json`. Do not alter field casing/requiredness for convenience. `DocChunk.blockRange` is a two-number JSON tuple; `PageMarkdown.pageNumber`, IR page numbers, table spans/dimensions are bounded `u32`. If an implementation needs an additional public model, stop at coordinator review and update schema/SSOT in the same PR.

## Dependency DAG and red/green commits

```text
P0 merged
  └─ 0 coordinator P7 type seam
       ├─ A markdown + table builder ─┬─ B classifier/label/visual ─┐
       │                              ├─ C pages ───────────────────┤
       │                              └─ D chunks ──────────────────┤─ F coordinator parse/Python integration
       └─ E shared Markdown table units ────────────────────────────┘
F ─ P1..P6 fixture extensions; E ─ P10/P12; B ─ P17
```

Workers A and E can use disjoint core source/test modules after Task 0; B, C, and D depend on A's builder/Markdown signatures and can then run on disjoint files. A owns `table/mod.rs` module declarations; later workers request module wiring through manager integration rather than editing it concurrently. To avoid test-file conflict, each worker writes private `#[cfg(test)]` cases in its owned module and manager consolidates public projection tests in `tests/projections.rs`. Each commit below is focused and retains a passing crate-local gate after its red/green cycle. Coordinator's shared-file commit is serialized after manager review.

### Local execution record (2026-09-30)

Task 0 landed in `8f13e04`; workers' disjoint A-E source modules and manager projection tests were integrated together in `60e0a0d` rather than the five individual commit boundaries proposed below. Parser-result assembly landed in `0ab662d`, the Markdown spacing parity correction in `3f897fb`, and the Python binding/model/test integration in `c415ce0` with later local review fixes. The synthetic projection implementation and local test gate are green; six pinned runtime-oracle generated-IR captures pass exact comparison, while successful real-document oracle captures remain zero. Fixture files and this documentation are being finalized on the branch. Focused PR, hosted required checks, review, and squash merge are still pending, so a checkbox whose wording includes that terminal workflow remains open.

### Task 0: Coordinator type seam

**Files:** Coordinator alone modifies `crates/kordoc-ir/src/{document,lib}.rs`, `crates/kordoc-ir/tests/ir_contract.rs`, and, only if the frozen contract changes with approval, `contracts/ir-schema.json` and its owning SSOT page.

- [x] Write failing `kordoc-ir` tests `doc_chunk_serializes_frozen_keys`, `chunk_options_rejects_unknown_and_null`, and `table_classification_roundtrips_exact_reason_order`. Assert `blockRange` length two, omission of optional `page`/`table.cells`, and camelCase fields. Run `cargo +1.97.0 test -p kordoc-ir --test ir_contract --locked`; expected RED is unresolved `DocChunk`/`ChunkOptions` or exact wire assertion.
- [x] Add the already-frozen P7 model types needed by worker code, without extending the schema or changing existing IR fields. Keep internal `CellContext`/builder options in core. Re-run focused and `cargo +1.97.0 test -p kordoc-ir --locked`; expected GREEN. Commit `feat: add frozen projection models` as a coordinator-owned commit before workers begin.

### Task A: Markdown and bounded table construction

**Files:** Create `crates/kordoc-core/src/markdown.rs`, `crates/kordoc-core/src/table/{mod,builder}.rs`; module-local tests. Manager later adds `tests/projections.rs` coverage.

- [x] Write named failing tests `markdown_preserves_block_order_and_escaping`, `markdown_renders_nested_table_as_html`, `markdown_rejects_unsafe_link_scheme`, `builder_places_addressed_spans_without_losing_collisions`, `builder_keeps_anchored_empty_cols_only_when_requested`, `builder_caps_grid_before_allocation`, and `flatten_layout_tables_only_with_explicit_parser_evidence`. Use `IrBlock`, `IrCell`, and `IrTable` from `kordoc-ir`; deserialize compact JSON fixtures if constructor fields obscure the case. Assert exact strings/IR topology, not contains-only snapshots.
- [ ] Run `cargo +1.97.0 test -p kordoc-core --locked markdown` and `cargo +1.97.0 test -p kordoc-core --locked table::builder`; expected RED is unresolved module/function or an exact assertion failure.
- [x] Implement `blocks_to_markdown(&[IrBlock]) -> Result<String, KordocError>` and `build_table(&[Vec<CellContext>], BuildTableOptions) -> Result<IrTable, KordocError>` with checked row×column/span arithmetic before allocation. `CellContext`/options are internal DTOs, not new wire fields. Structure rendering as escaping/sanitizing helpers plus block/table renderers; add bounded output accounting and deterministic `OUTPUT_TOO_LARGE`. Preserve intentional empty lines and captions. Use a safe URL scheme allowlist and HTML text/attribute escaping.
- [ ] Re-run focused tests and `cargo +1.97.0 test -p kordoc-core --locked`; expected GREEN. Commit `feat: add bounded Markdown and table projection` with only Task A files.

### Task B: Classification, traversal, label, visual policy

**Files:** Create `crates/kordoc-core/src/table/{classifier,label,visual}.rs`; module-local tests. Do not import a renderer.

- [x] Write failing tests `classifier_matches_dense_sparse_wrapper_and_ambiguous_matrix`, `classifier_preserves_reason_order_and_score_rounding`, `classifier_keyword_needs_structure`, `collect_tables_depth_first_including_caption_and_children`, `classify_tree_is_opt_in_and_preserves_original_text`, `representation_defaults_to_gfm_or_html`, `visual_policy_selects_only_requested_kinds`, and `label_cell_rejects_numeric_values_and_sentences`. Assert exact kind/score/reasons for generated matrices taken from the oracle tests.
- [ ] Run `cargo +1.97.0 test -p kordoc-core --locked table::classifier` (and `table::label`, `table::visual`); expected RED is unresolved module/function or exact assertion failure.
- [x] Implement `classify_table`, `classify_table_tree`, `collect_table_blocks`, `choose_table_representation`, `is_label_cell`, and `wants_crop`. Bound recursive traversal depth, table count and total visited cells; avoid recursive stack overflow on attacker-controlled IR. Keep classification absent by default; when requested, add only `classification` to tables and leave block order/content intact. Use a checked DFS with explicit stack if necessary.
- [ ] Re-run focused/core tests; expected GREEN. Commit `feat: add deterministic table policy` with only Task B files.

### Task C: Page projection

**Files:** Create `crates/kordoc-core/src/pages.rs`; module-local tests.

- [x] Write failing tests `pages_omitted_without_page_numbers`, `pages_keep_leading_and_middle_unumbered_blocks`, `pages_emit_empty_gap_entries`, `pages_preserve_revisited_page_order`, `pages_reject_absurd_gap_before_allocation`, and `pages_keep_empty_source_evidence`. Use compact input such as paragraphs on pages 1 and 3 and assert page 2 has `markdown: ""`; a pageless document must return `None`.
- [ ] Run `cargo +1.97.0 test -p kordoc-core --locked pages`; expected RED is unresolved function or assertion.
- [x] Implement `blocks_to_pages(&[IrBlock], Option<&[PageEvidence]>, render: impl Fn(&[IrBlock]) -> Result<String, KordocError>) -> Result<Option<Vec<PageMarkdown>>, KordocError>` using Task A renderer. Validate ascending/bounded evidence and checked min/max before allocating. Preserve the oracle grouping rule in the normal range. At the public boundary, a projected min..max span above 100,000 pages returns `OUTPUT_TOO_LARGE` with no partial pages. Test 100,001 using two blocks and test the inclusive success edge through a test-only smaller budget (for example 3 versus 4), so the test does not allocate 100,000 strings. Reconcile page evidence with observed blocks only where the source knows an actual page. Format-specific Markdown cleanup is a caller callback, not a parser-side projection fork.
- [ ] Re-run focused/core tests; expected GREEN. Commit `feat: add page Markdown projection` with only Task C files.

### Task D: Structural chunks

**Files:** Create `crates/kordoc-core/src/chunks.rs`; module-local tests.

- [x] Write failing tests `chunks_breadcrumb_heading_and_list_pop`, `chunks_section_merges_only_same_breadcrumb`, `chunks_block_mode_skips_empty`, `chunks_table_is_independent_with_optional_cells`, `chunks_ranges_are_monotonic_and_inclusive`, `chunks_first_page_of_run_and_omission`, and `chunks_unicode_marker_property`. Assert `c0001` formatting and exact Markdown with a recursive/nested-table fixture. Cover marker `□`, `○`, `-`, `1.`, `가.`, circled numerals and Hangul.
- [ ] Run `cargo +1.97.0 test -p kordoc-core --locked chunks`; expected RED is unresolved function or assertion.
- [x] Implement `blocks_to_chunks(&[IrBlock], ChunkOptions) -> Result<Vec<DocChunk>, KordocError>` using Task A renderer and Task 0's frozen wire model. Keep top-level inclusive ranges, no token slicing/overlap, and table-cell matrix only when requested. Set an explicit max chunk count/output byte budget and return `OUTPUT_TOO_LARGE` rather than truncating or changing granularity. Do not edit `kordoc-ir` in this worker commit.
- [ ] Re-run focused/core tests; expected GREEN. Commit `feat: add structural chunk projection` with only Task D files.

### Task E: One Markdown table-unit parser

**Files:** Create `crates/kordoc-core/src/markdown_units.rs`; module-local tests. If downstream crates need direct access, coordinator chooses a dependency-safe shared crate/API before P10/P12, with no duplicate parser.

- [x] Write failing tests `units_split_text_gfm_html_image_separator`, `gfm_cells_preserve_escaped_pipe_and_br`, `html_rows_preserve_nested_table_order_and_spans`, `html_cell_lines_preserve_image_presence`, `html_rejects_unbalanced_or_oversized_input`, and `html_does_not_execute_or_fetch`. Include nested table, mixed quote styles, entity, malformed close, and adversarial depth cases.
- [ ] Run `cargo +1.97.0 test -p kordoc-core --locked markdown_units`; expected RED is unresolved function or assertion.
- [x] Implement `split_markdown_units`, `parse_gfm_table`, `parse_html_table`, `html_cell_inner_to_lines`, and `split_cell_by_top_level_tables` around one bounded tokenizer. Export them from `kordoc_core::markdown_units` as the only Rust reader P8/P10/P12 call. Return structured errors for malformed input; no regex-only nesting counter, entity expansion, scripting, I/O, or remote fetching. Keep GFM escaping inverse aligned with Task A `escape_gfm`.
- [ ] Re-run focused/core tests; expected GREEN. Commit `feat: add shared Markdown table units` with only Task E files.

### Task F: Manager review and coordinator integration

**Files:** Manager creates `crates/kordoc-core/tests/projections.rs` and `docs/SSOT/components/normalization.md`; coordinator modifies `crates/kordoc-core/src/{lib,parse}.rs`, `crates/kordoc-ir/src/document.rs` only if approved types are still absent, `crates/kordoc-python/src/lib.rs`, `python/kordoc/{__init__,_api,_models,_native.pyi,tables}.py`, `contracts/public-api.json`, `tests/python/test_projections.py`, `tests/parity/test_document_goldens.py`, `docs/SSOT/{contracts/python-api.md,architecture/workspace.md,migration/status.md,README.md}`, and a new append-only WIKI entry.

- [x] Manager adds integration tests named `projected_success_contains_exact_markdown_pages`, `opt_in_classification_does_not_change_default_markdown`, `recursive_projection_is_deterministic`, and `projection_limits_are_typed_failures`. Run `cargo +1.97.0 test -p kordoc-core --test projections --locked`; expected RED until exports/assembly are wired.
- [x] Coordinator reviews shared-model signatures against `contracts/ir-schema.json`, supplies missing `DocChunk`/`ChunkOptions` and other P7 type models, and wires production/test registry assembly to project only after parser success. Preserve `ParseSuccess` optional omission, metadata, images, warnings, quality and page evidence. `classify_tables: Some(true)` is the only classification trigger. No production parser is claimed complete here.
- [x] Coordinator adds Python tests `test_blocks_to_markdown_exact`, `test_blocks_to_pages_omission_and_gap`, `test_blocks_to_chunks_structural_types`, `test_table_policy_nested`, `test_projection_errors_are_typed`, and `test_success_document_projection_from_test_adapter`. Exercise each assigned public value in `contracts/public-api.json` and model roundtrips against installed-wheel output; update only entries proven implemented. `extract_tables` remains planned for P17. Do not mark parser-specific success parity complete from test injection.
- [x] Resolve the `blocksToPages` optional TypeScript `render` callback at the Python boundary: its frozen manifest maps the value but does not type a Python callable adapter. Document whether the Python API accepts a callable or exposes only the default renderer, and obtain coordinator approval before setting that value to implemented. Treat `DocChunk`'s schema `number` fields as exact integer-valued outputs while preserving the frozen schema until coordinator review.
- [x] Add independently generated/licensed fixture recipes and, where permitted, oracle-captured expected outputs to `tests/golden/document/`; update the manifest with SHA-256, options, dimensions, source digest, oracle commit and exact capture command. Run the projection and document parity harnesses; report the six successful generated-IR captures separately from the zero successful real-document oracle cases. P1-P6 extend the document matrix with real source documents.
- [x] Update component SSOT and append WIKI evidence in the same behavior PR. Run the full gate below; manager reviews worker commits, coordinator reviews shared contract/API integration, then PR/review/required CI/squash merge. Coordinator integration commit: `feat: expose P7 projections through Python and parse results`. PR #11 merged as `d473bae79f1de6f1aced29c59609a2cb49f0aa90` after all 33 hosted checks passed; merge evidence is recorded in the append-only [P7 merge entry](../../../WIKI/2026/09/2026-09-30-ir-projections-merge.md).

## Security and limit decisions requiring review

All projection inputs may be untrusted even when IR came from a parser. Use checked arithmetic and budgets before allocating pages, table grids, traversal stacks, chunk arrays, or strings. P7 budgets are: 200 columns and 2,000,000 cells per table; 10,000 default rows, with the oracle-compatible dynamic row allowance when narrow columns still fit the cell budget; 64 levels of logical block/table recursion; a 256-level raw JSON preflight ceiling before logical normalization; 100,000 projected pages across min..max; 100,000 emitted chunks; 8 MiB raw HTML per Markdown table unit; and 256 MiB UTF-8 bytes per projected Markdown/chunk result. The Python/native boundary must preflight serialized input before allocating JSON and cap each emitted result while writing it. Test each boundary and one beyond with compact constructed input or a configurable test-only budget so tests do not allocate the maximum. Do not silently discard text, table cells, spans, chunks, or pages to meet a budget; return typed `OUTPUT_TOO_LARGE`/security-limit errors according to frozen codes. Current oracle builder's `rows.slice(0, maxRows)` truncates, while project policy forbids silent loss: record this as a compatibility difference and obtain coordinator approval before fixing expected parity or selecting an error. These caps are public-boundary safety rules; oracle-normal outputs stay byte exact, and the MCP layer retains its own contracted 200,000-character response cap. Record the same numbers in `docs/SSOT/components/normalization.md` when implementation lands. Keep URL schemes and HTML escaping safe; never execute Markdown/HTML, resolve paths, or fetch resources in projection. Fuzz/proptest recursive IR, spans, arbitrary Unicode, and malformed HTML with bounded generation.

`PageEvidence` can describe an empty source page that `blocksToPages` alone cannot see; the coordinator must decide precedence when evidence conflicts with block page numbers. `flattenLayoutTables` needs an internal parser flag to replace the oracle's `WeakSet` and must run only on the intended HWP3/HWP5 layout boxes. HWPX flattening is disabled: preserving table topology keeps roundtrip source ordinals stable. These decisions stay out of public wire schema unless coordinator explicitly approves a contract change.

### Coordinator resolutions before Task 0

- P7 may complete with runtime-captured oracle projection answers over generated IR while the real-document success numerator remains zero. This proves the pure projections only; P1-P6 still must add successful source-document parity.
- Rust returns typed `OUTPUT_TOO_LARGE` before the oracle's silent row truncation would occur. The generated matrix covers shared in-budget behavior, and the component SSOT records this intentional security/no-loss divergence.
- Observed block page numbers are authoritative for content grouping. Internal `PageEvidence` contributes only known empty page numbers to the bounded output union; it never reassigns blocks. The public blocks-only call retains the oracle rule and returns `None` when no block has a page number.
- Python `blocks_to_pages(blocks, render=None)` preserves the optional renderer. The callback runs synchronously with a tuple of immutable block mappings for each page and must return `str`; the default path stays native and GIL-detached. Callback exceptions propagate unchanged, and callback mode never runs after native detach.
- Layout flattening uses an internal `LegacyLayoutFlattening` policy selected only by future HWP3/HWP5 adapters. HWPX and generic projections always select `Never`; no internal marker becomes a wire field.

## Verification and merge gate

Run focused red/green commands above after each task. Before claiming P7 complete, run:

```bash
PYO3_PYTHON=.venv/bin/python3 cargo +1.97.0 test -p kordoc-core --test projections --locked
cargo +1.97.0 fmt --all -- --check
cargo +1.97.0 clippy --workspace --all-targets --all-features --locked -- -D warnings
cargo +1.97.0 test --workspace --locked
RUSTDOCFLAGS=-Dwarnings cargo +1.97.0 doc --workspace --no-deps --locked
cargo +1.97.0 llvm-cov -p kordoc-ir -p kordoc-core --locked --fail-under-lines 80
PYO3_PYTHON=.venv/bin/python3 uv run --python 3.10 maturin develop
uv run --python 3.10 pytest tests/contracts tests/parity tests/python -q
uv run ruff check python tests scripts
uv run ruff format --check python tests scripts
uv run mypy python/kordoc scripts
uv run python scripts/check_docs.py
uv run maturin build --release --locked --out target/wheels
uv run maturin sdist --out target/wheels
uv run python scripts/check_artifacts.py target/wheels/*.whl target/wheels/*.tar.gz
cargo deny --locked check
cargo audit --file Cargo.lock
```

Also run relevant 30-second projection/HTML fuzz smoke targets once added; run `actionlint .github/workflows/*.yml` and `zizmor .github/workflows`. On machines where `cargo` is not the rustup proxy, use the verified Rust 1.97 binary without `+1.97.0`. Before review, inspect `git ls-files` and built wheel/sdist inventory for zero oracle paths. Push only `feature/ir-projections`; open a focused PR, await strict `ci-gate`, `security-gate`, `wheels-gate`, `fuzz-gate`, CodeQL and review, then squash merge. No P1-P6 parser PR may depend on P7 until this gate merges.
