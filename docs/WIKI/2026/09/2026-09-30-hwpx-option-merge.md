---
id: 2026-09-30-hwpx-option-merge
date: 2026-09-30
status: recorded
component: hwpx-options
issue: null
pr: https://github.com/Han-taz/paruster/pull/24
commit: 0cd5b261371aaf697c11db825bb22500db369782
related_ssot:
  - docs/SSOT/components/hwpx.md
  - docs/SSOT/migration/status.md
related_decisions: []
---

# Protected HWPX option checkpoint merge

## Context

The [wheel/review record](2026-09-30-hwpx-option-wheel-gates.md) describes the
focused candidate's implementation and local evidence. This follow-up records
publication without changing that earlier history.

## Work performed

The focused branch was pushed and PR #24 opened. Initial hosted Python runs
exposed a test import relying on `python -m pytest` adding the repository root
to sys.path. The fixture import was corrected for the pytest console entry
point used by CI, with no behavior, fixture answer, or gate change. Both source
and independently installed wheel console pytest reruns pass 317 tests.

## Evidence

Final head `d267f47c4332fe026b0f496d3bc716fbc507fe5c` has no pending or failed
checks. `ci-gate`, `security-gate`, `wheels-gate`, `fuzz-gate` and Rust/Python/
Actions CodeQL passed; six native wheel targets and CPython 3.10–3.14 passed.
Local 30-second HWPX package/XML and projection fuzz campaigns each exited 0;
the package target includes ordered option result assembly. Generated mutations
were preserved as ignored investigation artifacts, never promoted to fixture
answers or committed.

## Outcome

Protected squash merge completed at `2026-09-30T11:08:49Z` as
`0cd5b261371aaf697c11db825bb22500db369782`. All five documented options now reach
the merged Rust/Python HWPX candidate. Original H0 hashes, complete-result
answers, security limits, schemas and protected success numerator are unchanged.

## Follow-ups

Full HWPX capability remains pending for representative/restricted-corpus
evidence, malformed-section diagnostics and ODF checksum interoperability.
