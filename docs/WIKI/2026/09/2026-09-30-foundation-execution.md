---
id: 2026-09-30-foundation-execution
date: 2026-09-30
status: recorded
component: ci-foundation
issue: null
pr: https://github.com/Han-taz/paruster/pull/5
commit: 33196b5
related_ssot:
  - docs/SSOT/operations/development.md
  - docs/SSOT/operations/release.md
  - docs/SSOT/migration/status.md
  - docs/SSOT/migration/plans/2026-09-29-foundation-implementation-plan.md
related_decisions:
  - docs/SSOT/migration/2026-09-29-rust-python-port-design.md
---

# Foundation quality and release-candidate execution

## Context

PR [#5](https://github.com/Han-taz/paruster/pull/5) is a draft against `feature/rust-python-foundation`; the foundation implementation PR [#4](https://github.com/Han-taz/paruster/pull/4) remains open. PRs [#1](https://github.com/Han-taz/paruster/pull/1), [#2](https://github.com/Han-taz/paruster/pull/2), and [#3](https://github.com/Han-taz/paruster/pull/3) were merged earlier. This record captures the foundation CI/security/release-candidate work and local evidence at the time of review. It does not replace current policy in SSOT.

## Work performed

- Added the pinned CI, wheel, bounded-fuzz, security, and non-publishing release-candidate workflows, plus artifact and documentation guardrails.
- Added reproducible development and release-candidate operations guides and a WIKI entry index for this record.
- The relevant commits, in execution order, are:
  - `a4e4f38` — bounded foundation fuzz targets.
  - `4bb9622` — foundation quality and native-wheel workflows.
  - `247f336` — artifact and documentation guardrails.
  - `afce280` — non-publishing release-candidate workflow.
  - `50cc75d` — internal workspace dependency version pins.
  - `9868850` — pinned security and dependency policy.
  - `c6ba4d3` — development and release operations guides.
  - `33196b5` — disable persisted checkout credentials.

## Evidence

- Local foundation line coverage was **88.63%**, above the 80% policy threshold.
- The 31-second `detect_format` fuzz campaign completed **12,287,343 runs** with no crash. The 31-second `zip_preflight` campaign completed **8,590,277 runs** with no crash.
- `actionlint` 1.7.12 completed cleanly after the official release artifact checksum was verified. `zizmor` 1.30.1 reported no findings.
- `cargo deny` exited 0; it emitted only warnings that the allowed license identifiers BSD-2-Clause, BSD-3-Clause, and ISC were unused by the dependency graph.
- `cargo audit` checked 86 dependencies against 1,277 advisories and reported no vulnerabilities.
- At the time this entry was recorded, these hosted checks were **currently in progress**, not passed: [CI](https://github.com/Han-taz/paruster/actions/runs/36598937256), [Security](https://github.com/Han-taz/paruster/actions/runs/36598937186), [Native wheels](https://github.com/Han-taz/paruster/actions/runs/36598937310), and [Bounded fuzzing](https://github.com/Han-taz/paruster/actions/runs/36598937227). Their final conclusions must be appended before the PR is merged.

## Outcome

The foundation now has local quality, security, artifact, and non-publishing candidate checks with reproducible operator guidance. Hosted workflow evidence remains pending until the four runs above finish. This is foundation validation only: document parsers, OCR, transformations, rendering, and all 17 runtime MCP tools remain pending; format-specific parsing and product behavior must not be described as delivered.

## Follow-ups

- Append the final hosted workflow conclusions to this historical record before merge; do not revise this entry silently after it is merged.
- Keep release publication disabled until the compatibility inventory has no `planned` entries, all 17 MCP runtime tools are implemented and verified, and two consecutive release-candidate workflows pass for the same immutable commit.
