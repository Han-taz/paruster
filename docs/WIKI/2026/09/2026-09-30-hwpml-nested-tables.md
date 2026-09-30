---
id: 2026-09-30-hwpml-nested-tables
date: 2026-09-30
status: implementation-candidate
component: hwpml
issue: null
pr: null
commit: null
related_ssot: [docs/SSOT/components/hwpml.md, docs/SSOT/migration/plans/2026-09-30-hwpml-nested-table-plan.md]
related_decisions: [Private unique-anchor H0 qualification precedes ambiguous-table retention and public HWPML registration.]
---

# Lower authored nested tables through the shared bounded builder

The coordinator approves a private unique-anchor slice after shared-table
PR #31 merges. Luna A owns HWPML native lowering/tests and the exact crate-local
H0 input copy; SOL owns the focused plan and independent review; root owns
Cargo, offline provenance, SSOT/WIKI and publication. Existing shared contracts,
fixtures, captures, scoring and table placement remain unchanged.

The 949-byte H0 XML copy retains SHA
`3255695c5b3a0a0cd677687d9e27fa9248f30118a7ebf0be6afc19287e86c308`.
Root's copy contract first fails against the absent fixture directory, then
passes after A copies the exact authored bytes. Six H0 inventory/copy checks
and three captured-IR Markdown projections pass offline (nine total). A's
native focused RED reproduces current selected-table UNSUPPORTED_FORMAT
before implementation. Root adds the already pinned workspace serde_json as
Hancom's dev dependency for full captured-block equality; the root lockfile
changes only that dependency edge, with no external upgrades. Narrow -text
attributes preserve the copied XML's provenance across Windows checkouts.

The first qualification covers three unchanged captures: default/false 2×2,
true 2×3, original spans and flat text with paragraph then nested-table blocks.
The adapter must retain one shared 2M logical-cell budget for all tables,
the 256MiB lowering budget, checked/fallible allocation and bounded recursion.
Shared crate geometry tests prove the inclusive logical-cell boundary; small
Hancom multi/nested-table tests and independent source review verify budget
reuse without huge allocations or a production test-limit constructor.
Ambiguous attachments, unsupported captions and depth 9 reject explicitly.
Those deliberate private limitations do not establish oracle parity for such
documents. Native GREEN, independent review and publication remain pending.


## Reviewed native candidate

All three H0 cases pass complete serde-serialized block equality. Native
geometry preflight now rejects actual row counts and unsupported column/span
bounds before retaining cell payloads. Root review prompts precharge/geometry
adversarial checks; SOL review finds selected tables under inline note/header/
footer wrappers and no-direct-cell tables. Those now reject explicitly with
authored regression checks, preserving no-table CHAR behavior. The source
skips such tables; this is a documented private divergence rather than parity.

Final independent review is clean, with fresh Hancom 147 unit plus 144
integration executions, strict scoped Clippy/fmt/diff checks. Root locked
workspace tests, all-target/all-feature Clippy, warning-free rustdoc and fmt
also pass. Root Ruff (54 files), eight-source mypy/docs checks and 352 Python/
helper tests plus three subtests pass using the existing merged worker wheel;
a fresh native candidate artifact and latest-main join remain before PR.
Autocrlf=true index checkout preserves the 949-byte table-copy pin. No shared
contract/fixture/scoring limit is weakened.


## Latest-main fresh artifact verification

The candidate merges protected PDF text-document PR #32; only additive
attribute/WIKI index conflicts need resolution. Native HWPML source remains
frozen. Merged locked workspace tests, all-target/all-feature Clippy, rustdoc,
fmt, Ruff (55 files), eight-source mypy, docs, actionlint and zizmor pass.
A fresh macOS ARM abi3 wheel installed into isolated CPython 3.10 passes 358
Python/contracts/parity/helper tests and ten subtests. Wheel/sdist audits pass;
five HWPML source/manifest/fixture members retain exact bytes in the sdist.
Hosted final-head gates and all six worker targets remain before merge.
