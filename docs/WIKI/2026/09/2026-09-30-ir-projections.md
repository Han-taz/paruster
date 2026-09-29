---
id: 2026-09-30-ir-projections
date: 2026-09-30
status: recorded
component: shared-ir-projections
issue: null
pr: pending
commit: pending
related_ssot:
  - docs/SSOT/components/normalization.md
  - docs/SSOT/contracts/python-api.md
  - docs/SSOT/migration/plans/2026-09-30-ir-projections-implementation-plan.md
  - docs/SSOT/migration/status.md
related_decisions:
  - docs/SSOT/migration/2026-09-29-rust-python-port-design.md
---

# Shared IR projections local implementation

## Context

P0 supplied a source-neutral parser handoff but no shared Markdown, page,
chunk, or table normalization. P7 establishes those pure projections before
the independent source parsers begin.

## Work performed

The branch added bounded Rust Markdown/page/chunk projection, table
construction, classification and label/visual policy, and one shared
Markdown table-unit reader. ParseSuccess assembly now projects after
successful dispatch; the production registry remains empty. Python exposes
the three block projections, the mapped
`kordoc.tables.classify_table_tree` operation, and frozen chunk/page models.
An optional page renderer runs synchronously with deeply immutable blocks;
the default path releases the GIL. Table classification remains opt-in and
does not change default Markdown. HWPX and generic IR do not flatten tables.

## Evidence

- Task 0 type seam: `8f13e04`; consolidated source projections:
  `60e0a0d`; parser assembly: `0ab662d`; Markdown spacing correction:
  `3f897fb`; Python integration: `c415ce0`. Later local binding fixes and
  fixture/documentation changes are still on the feature branch.
- Six generated-IR projection cases were captured from pinned runtime
  oracle revision `bb71f7fb0bf51dd456d27505a8c04772df182144` with
  input/expected hashes, source digests, and exact capture commands in
  `tests/golden/projection-manifest.json`. All six compare exactly without
  normalization. The projection parity suite has nine passing tests,
  including manifest and denominator assertions.
- The local gate recorded 91 core unit tests plus 20 all-feature core
  integration tests, 8 native binding tests, and 124 combined Python/parity
  tests. Two bounded proptests exercise arbitrary Unicode and recursive IR.
  Dedicated Markdown-unit and projection fuzz targets each completed a
  30-second local campaign without a crash, and the hosted matrix now contains four
  targets. Strict Clippy, Ruff, and mypy checks passed locally. These are local
  results, not hosted CI or merge evidence.
- The successful real-document oracle numerator is **zero**. Synthetic IR
  captures prove pure projection only; they do not prove a source parser.

## Outcome

P7's implemented pure projection and assigned Python mappings are locally
verified on `feature/ir-projections`. A PR, required hosted gates, review,
and squash merge remain pending. No parser, renderer, or MCP tool is newly
claimed complete.

## Follow-ups

- Record hosted check and merge evidence in a later append-only entry.
- P1-P6 add real-document oracle captures to the separate document-result
  harness; downstream P8/P10/P12 reuse the one table-unit reader.
