---
id: 2026-10-01-pdf-base-text-merge
date: 2026-10-01
status: recorded
component: pdf
issue: null
pr: https://github.com/Han-taz/paruster/pull/36
commit: c5be99bd3cd4c0508a6e9237fb64c2e30a12d30a
related_ssot: [docs/SSOT/components/pdf.md, docs/SSOT/migration/status.md]
related_decisions: []
---

# Private PDF base geometry and text scalar merge

PR #36 merged at 2026-09-30T14:58:13Z with final head
`e8a11b1fcc7b73c0bbe6cd1900dfc32e4b75d801` and squash
`c5be99bd3cd4c0508a6e9237fb64c2e30a12d30a`.

All 42 checks succeeded: five aggregate gates, actual Rust/Python/Actions
CodeQL and all six installed-worker targets. CI run 36731663318, Security
36731663333, Native wheels 36731663334, Bounded fuzzing 36731663332 and PDF
worker wheels 36731663329 are successful. Each worker target ran release
geometry/metadata/scalar tests and all four existing installed probes.

Local evidence is 265 scoped feature executions, release native 9/10/13 tests,
and fresh base/optional worker wheel runs of 366 Python/helper cases plus ten
subtests. The unchanged 100ms supervisor marker race and later unchanged full
success remain recorded in the earlier geometry entry. No quality gate or
resource limit was weakened. The public PDF parser remains unregistered.
