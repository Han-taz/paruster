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

## Hosted run update

This dated addendum records hosted validation evidence without changing the historical snapshot above. Subsequent evidence and corrective commits were `d7fb6bd`, `1a74688`, `6c3a0d2`, `d13b0ac`, `485ceed`, and `a9df485`. Commit `a9df485` (`a9df4851e8fc1c8b2cd889fdec09e1f3fc111c1d`) is the last fully validated implementation head.

Timing correction: At the time of the historical snapshot, CI, bounded fuzzing, and native wheels had already succeeded; only Security was still in progress. The historical paragraph said all four hosted checks were in progress. That statement was inaccurate for the other three runs. The initial Security run subsequently failed for four workflow issues: ShellCheck checksum redirection, a zizmor installer artifact, non-default stacked dependency review, and a CodeQL dynamic release ref. The corrective commits fixed those workflow issues. The linked validation cycle against the last fully validated implementation head succeeded in all four hosted runs:

- [CI](https://github.com/Han-taz/paruster/actions/runs/36600464167)
- [Security](https://github.com/Han-taz/paruster/actions/runs/36600464228)
- [Native wheels](https://github.com/Han-taz/paruster/actions/runs/36600464274)
- [Bounded fuzzing](https://github.com/Han-taz/paruster/actions/runs/36600464235)

The hosted Native wheels run succeeded across all six wheel jobs.

Local release-candidate rebuild artifacts were `kordoc-0.1.0-cp310-abi3-macosx_11_0_arm64.whl` (SHA256 `e6a3117454deea9acf70ee86d2f20404d1ee04a37ee0707045f0f37d7226f35c`) and `kordoc-0.1.0.tar.gz` (SHA256 `23922328ce71ff671c3c83e145b08f1c781a9c32e35013a905d40f446aff7e1a`). These are local rebuild checksums; the hosted Native wheels run is reported separately above.

These URLs document the validation cycle for the implementation head above. This docs-only evidence commit needs its own hosted pass; the linked runs do not establish checks for a commit that includes this update. The stacked PR's dependency-review result was a scope-message only; it is not main-target proof. Main-target dependency-review proof and required-check enforcement remain pending PR #4. Product parity also remains pending as recorded in the live migration status.
