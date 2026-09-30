---
id: 2026-09-30-hwpml-private-text
date: 2026-09-30
status: locally-verified
component: hwpml
issue: null
pr: null
commit: null
related_ssot: [docs/SSOT/components/hwpml.md, docs/SSOT/migration/plans/2026-09-30-hwpml-private-text-plan.md, docs/SSOT/migration/status.md]
related_decisions: [Keep HWPML text lowering private until tables, recovery and public parity qualify.]
---

# Bound the first private HWPML text lowerer

## Context

HWPML H0 merged in PR [#28](https://github.com/Han-taz/paruster/pull/28)
as `5a077eb78704b8b60e03963a22297978f9854d63`, freezing eight authored
XML inputs and 13 complete public oracle observations. This slice follows that
capture checkpoint without registering a parser or changing shared contracts.

## Work performed

The Hancom crate now has a private text/metadata lowerer using the existing
bounded XML tree through a narrow internal seam. It reads direct summary
fields and outline shapes, emits ordered paragraph/heading blocks with
section page ordinals, and applies page selection before section lowering.
It enforces the 50 MiB input preflight before optional same-length `&nbsp;`
normalization, uses the existing XML limits and lowering-allocation meter,
rejects DTD/custom entities, and fails transactionally on malformed input or
unsupported selected structural tables. Three Rust test inputs are exact
crate-local copies of H0 fixtures so source builds need no external oracle.

## Review and local evidence

The independent review identified two semantic edge cases before freeze:
inline table preflight had skipped a wrapper whose paragraph text walker
visited, and a valid reversed page range could fall through to a numeric
prefix. Both have focused regressions. The final native candidate passes 142
Hancom library tests, 144 integration-binary executions, all-target strict
Clippy, formatting and a clean authored diff check. A 50 MiB + 1 input with
`&nbsp;` verifies preflight before normalization budgeting; reduced-budget
N/N-minus-one and XML resource tests verify typed hard failures.

## Scope and follow-ups

The unchanged H0 captures document private valid-text behavior. The security
policy deliberately differs from legacy tolerant DTD/malformed recovery; no
full-result parity or protected success score is claimed. HWPML-specific table
coordinates/nesting, bounded recovery, public Python/core registration,
options and representative corpus parity require separate reviewed slices.
Hosted required checks remain pending until this branch's PR head passes them.
