---
id: 2026-10-01-pdf-text-rewrites
date: 2026-10-01
status: recorded
component: pdf
issue: null
pr: null
commit: null
related_ssot: [docs/SSOT/components/pdf.md, docs/SSOT/migration/plans/2026-09-30-pdf-text-rewrites-plan.md]
related_decisions: []
---

# Private Rust PDF text rewrites

The approved user boundary remains bundled PDF.js in V8, with Rust-owned
execution and remaining processing. This slice ports only a pure, unwired
single-item text helper. It trims ECMAScript whitespace, selectively normalizes
214 Kangxi radicals, and applies the source numeric/uppercase literal-space
rules. Nonselected fullwidth/circled/astral characters and NEL remain unchanged.

Twenty actual normalizeItems text projections and 214 independently evaluated
source NFKC mappings are pinned CC0 evidence. Separate Hangul split-order
observations are preserved without claiming this helper implements splitting.
The offline recipe recreates only authored inputs; no test/build/runtime path
requires the ignored oracle or Node. The optional unicode-normalization 0.1.25
edge reuses the existing Unicode 17 lock entry.

RED failed because the planned helper file was absent; GREEN passes nine focused
cases including all captured mappings, inclusive 64KiB and invalid nonnegative
integral-font input boundaries. Independent review, strict Clippy and the full
worker feature suite pass. A transient unchanged 100ms startup marker failure
was followed by unchanged full-suite success; no timeout or test was modified.
Root added two exact-inventory/offline-regeneration contract tests. Final artifact
and six-target hosted evidence will be appended after qualification.

Full normalizeItems integration, split/deduplication/spacing/operator flags,
layout/IR, corpus qualification and public PDF registration remain pending.

## Coordinator verification

Fresh default workspace tests, release rewrite integration (9 executions), strict
all-target/all-feature workspace Clippy, denied-warning rustdoc, Ruff, mypy,
actionlint, zizmor and docs checks pass. Fresh base wheel and sdist audits and
optional-worker architecture/license audit pass. Base and worker installations
each pass 368 Python/helper cases plus ten subtests; all four installed-worker
probes pass. The supervisor's existing 100ms startup marker test reproduced a
39/40 coordinator failure on the first feature run; a manager's independent full
run passed unchanged. This timing investigation is separate and remains open;
this helper changes neither supervisor nor test. Hosted final-head gates remain
required before merge.
