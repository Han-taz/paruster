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

After merging the H0 squash and PDF worker-wheel mainline into this branch,
fresh macOS arm64 `cp310-abi3` wheel and sdist builds passed the artifact policy
(`validated 2 artifact(s)`). The sdist contains `src/hwpml.rs` and the three
crate-local XML test inputs with exactly the H0 SHA-256 bytes; the wheel omits
test fixtures, and neither archive contains migration-oracle source. The wheel
installed in a new isolated CPython 3.10.19 environment passes **325**
Python API, contract and parity tests. The installed package path is under
that environment's `site-packages`. Ruff check/format and mypy pass; the
separate worker-wheel helper suite passes **19** tests against its fixtures.
The private module has no Python export, so these installed-wheel results do
not establish HWPML document-output parity.

Locked workspace Rust tests, strict workspace all-target/all-feature Clippy,
and warning-free workspace rustdoc pass against CPython 3.10.19 after the
mainline merge. Documentation links/indexes also pass. The first local Rust
test invocation picked up system Python 3.9, below the project's abi3 3.10
minimum; setting `PYO3_PYTHON` to the fresh 3.10 environment and its matching
`PYTHONHOME` resolved that environment issue without source changes.

## Scope and follow-ups

The unchanged H0 captures document private valid-text behavior. The security
policy deliberately differs from legacy tolerant DTD/malformed recovery; no
full-result parity or protected success score is claimed. HWPML-specific table
coordinates/nesting, bounded recovery, public Python/core registration,
options and representative corpus parity require separate reviewed slices.
Hosted required checks remain pending until this branch's PR head passes them.
