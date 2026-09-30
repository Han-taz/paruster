---
id: 2026-09-30-pdf-rust-metadata
date: 2026-09-30
status: implementation-candidate
component: pdf
issue: null
pr: null
commit: null
related_ssot: [docs/SSOT/components/pdf.md, docs/SSOT/migration/plans/2026-09-30-pdf-rust-metadata-plan.md]
related_decisions: [PDF.js supplies bounded raw fields; Rust owns product metadata policy.]
---

# Normalize bounded PDF metadata in Rust

The coordinator approves a private pure Rust normalizer with unchanged raw DTO,
worker protocol and public contracts. Luna B owns native normalization/tests and
new authored inputs/captures, SOL owns the focused plan and independent review,
and root owns offline provenance, SSOT/WIKI and publication.

Three separate CC0 PDFs freeze six metadata observations before native RED:
full-parse metadata projections and metadata-only results under oracle commit
`bb71f7fb0bf51dd456d27505a8c04772df182144`, parser source SHA
`2eb7018d17bf9bb3d39b7d2cb21145fe812a5257a239331aa13d44da9776bf70`,
and PDF.js 4.10.38. These are metadata observations, not complete ParseResult
captures. All source parse calls succeed before selecting metadata. Source
scripts remain research-only outside the repository; README records command,
script digest and license. Original fixtures and captures stay unchanged.

Recipe preparation changes only verification/write-mode scaffolding before
final freeze. Root initially detects the recipe hash changing and --write
rejecting an empty directory. B fixes fresh regeneration and default read-only
verification, updates actual recipe size/commands, and passes Ruff before final
pin. The 2985-byte recipe SHA is
`f4c19e1cbf63c80059329c7501ee58e51e56f15e902ff9d00217be1a20f3be8d`;
2703-byte six-record capture SHA is
`d79d30dc6b189bbfcdb4e05fbb8bec54de8c9740298c46ee1034ce7838413145`.
Two root offline contracts pass; copied recipe regeneration needs only stdlib.
Autocrlf=true index checkout preserves all five recipe/input/capture byte pins.
The focused native RED fails on the absent metadata module before implementation.

Captured rules retain duplicate keyword order and Some([]) for delimiter-only
nonempty strings. Partial D:2025120X yields 2025-12-01T00:00:00; invalid calendar
2025-13-99 is preserved. Native conversion must reproduce source trim/date
behavior with checked/fallible copies and defensive 4KiB/16KiB raw metadata
limits; allocation failures remain typed fatal errors. Geometry/layout/public
registration and broad PDF parity are outside this checkpoint. Native GREEN,
independent review and publication evidence remain pending.


## Frozen native candidate review

Ten focused native cases pass, including all six authored-PDF metadata
observations, explicit FEFF/NBSP trimming with U+0085 retention, ordered
duplicate/empty keywords, permissive dates and inclusive UTF-8 field/aggregate
limits. Final scoped feature suites pass 243 executions across thirteen groups;
strict all-target/all-feature scoped Clippy, fmt and offline recipe checks pass.
Root source review and SOL independent re-review are clean. Raw-field cap
validation precedes every output copy; strings and vectors grow fallibly.
Page count and mode are typed internal arguments from the existing private
extraction path; this does not implement a separate metadata-only worker.
Latest-main integration and fresh artifacts/hosted gates remain before PR.


## Latest-main and installed worker verification

The candidate merges protected table/PDF text mainline; a synthetic reviewed
text-candidate base resolves squashed ancestry while retaining append-only
WIKI history and both SSOT additions. Root merged locked workspace tests,
strict all-feature Clippy/rustdoc, fmt, Ruff (55 files), eight-source mypy,
docs, actionlint and zizmor pass. Fresh base wheel/sdist artifact audits pass;
private PDF sources remain outside base-sdist closure as documented by PR #32.

A fresh optional release-worker wheel passes architecture, exactly 36 unique
notices, executable mode and artifact scans. Its clean CPython 3.10 install
passes all four installed probes and 356 full Python/helper cases plus ten
subtests. Initial collection lacks jsonschema in the new dev environment;
installing its pinned declared version resolves it without test changes.
Root repeats the exact kind-4 installed comparison. The 64037952-byte worker
SHA remains `3361ff9ee40bed25990429bb6fbe12e3b1ca34bde2f85648c923a9a9a3c4b837`;
normalization is private Rust logic exercised by native tests, not a new wire
operation. Stage inputs are preserved in ignored build storage. Final-head
hosted gates remain required before merge.

## Main33 combined verification

Merged private nested-table main `e0fde7d` into this candidate. Fresh locked
workspace tests, strict all-feature Clippy/rustdoc and format checks pass.
Fresh base wheel and sdist audits pass. A new optional macOS ARM worker wheel
passes architecture/notices inventory, exact installed kind-4 probe and 360
isolated Python/helper cases plus ten subtests. Staged files are preserved
under ignored build storage. Native metadata code and frozen captures are unchanged.
