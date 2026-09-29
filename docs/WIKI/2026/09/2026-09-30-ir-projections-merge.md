---
id: 2026-09-30-ir-projections-merge
date: 2026-09-30
status: recorded
component: shared-ir-projections
issue: null
pr: 11
commit: d473bae79f1de6f1aced29c59609a2cb49f0aa90
related_ssot:
  - docs/SSOT/components/normalization.md
  - docs/SSOT/contracts/python-api.md
  - docs/SSOT/migration/plans/2026-09-30-ir-projections-implementation-plan.md
  - docs/SSOT/migration/plans/2026-09-30-product-port-implementation-plan.md
  - docs/SSOT/migration/status.md
related_decisions:
  - docs/SSOT/migration/2026-09-29-rust-python-port-design.md
---

# P7 shared projections merge evidence

## Context

The initial P7 implementation record documented local verification and left
hosted CI and merge evidence open. This append-only follow-up records the
protected GitHub Flow integration result.

## Work performed

PR [#11](https://github.com/Han-taz/paruster/pull/11) merged P7's bounded
Markdown, page and chunk projections, table construction/classification
policy, shared Markdown table-unit reader, Python mappings, recursive
property tests, oracle projection captures, and fuzz targets. The SSOT now
records P7 as merged and allows P1-P6 parser implementation to proceed.

## Evidence

- GitHub squash merge commit:
  `d473bae79f1de6f1aced29c59609a2cb49f0aa90`.
- All 33 PR checks succeeded.
- Hosted [CI](https://github.com/Han-taz/paruster/actions/runs/36622144159),
  [Fuzz](https://github.com/Han-taz/paruster/actions/runs/36622144214),
  [Wheels](https://github.com/Han-taz/paruster/actions/runs/36622144316), and
  [Security/CodeQL](https://github.com/Han-taz/paruster/actions/runs/36622144308)
  runs succeeded.
- Six generated-IR oracle projection captures pass exactly. This does not
  increase the successful real-document parser numerator, which remains 0.

## Outcome

P7 is merged. P1-P6 are unblocked to build parser integrations on the shared
projection APIs, while real-document parser parity remains pending until
source-document fixtures pass.

## Follow-ups

P1-P6 must add representative and adversarial source-document fixtures to the
shared matrix and retain the zero-loss projection policy.
