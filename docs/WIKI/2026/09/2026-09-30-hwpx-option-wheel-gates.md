---
id: 2026-09-30-hwpx-option-wheel-gates
date: 2026-09-30
status: recorded
component: hwpx-options
issue: null
pr: null
commit: null
related_ssot:
  - docs/SSOT/components/hwpx.md
  - docs/SSOT/components/normalization.md
  - docs/SSOT/migration/status.md
related_decisions: []
---

# Option candidate installed-wheel and review gates

## Context

The [restoration record](2026-09-30-hwpx-option-restoration.md) captures the
initial option implementation. Subsequent review found two common-case issues:
fieldEnd rescanned preceding floating-table text parts, and ordinary table
paragraphs incurred an unnecessary copied span buffer before finding no guide.

## Work performed

OpenField now retains its starting part/local span offset and scans only its
own range. A borrowed CLICK_HERE preflight returns before copies/charges when
none exists. A 512-field/floating-table case and a zero-lowering-budget plain
cell preflight regression cover these paths. Final scoped review reports CLEAN,
with no Critical or Important issue within this checkpoint. The stable options
and schemas remain unchanged.

## Evidence

- Final Hancom: 118 unit cases and 137 integration-binary executions, including
  19 distinct integration cases. Core: 109 units plus integration groups.
- Locked workspace tests, all-target/all-feature warning-denied Clippy, fmt and
  warning-free Rust documentation pass after integrating PDF/V8 main.
- Fresh editable native extension and independent macOS arm64 release abi3
  wheel on CPython 3.10 each pass all 317 Python/contract/parity tests. Both
  wheel and sdist pass forbidden-path artifact inspection.
- Foundation line coverage is 91.63%; the new postprocessor is 96.57%. The
  unchanged gate remains 80%. Ruff, formatting, mypy and docs pass.
- Required audit and all four dependency-policy categories pass. The existing
  V8 build-time paste unmaintained advisory remains recorded; no ignore or
  policy relaxation was added.
- Matched release-wheel local smoke measurements (30 runs after five warmups)
  compare H4 with this candidate: a 73,567-byte authored paragraph document is
  3.461 ms versus 3.524 ms (+1.8%); a 144,853-byte 1,000-cell table is 10.095 ms
  versus 9.907 ms (-1.9%). Peak process RSS is 25.43/33.82 MB versus 24.84/33.36
  MB, respectively. This small two-input local smoke is not a restricted-corpus
  performance claim; it shows no unexplained >10% regression on these inputs.

## Outcome

Protected publication and hosted six-wheel/Python matrix gates must pass on
this candidate's actual final head before squash merge. Parser diagnostic,
ODF checksum and representative/restricted-corpus gaps remain pending. Earlier
entries are preserved; this follow-up records their later evidence.

## Follow-ups

The HWPX package fuzz target now additionally exercises actual core result
assembly with script stripping, plain and HTML conversion, explicit trailing
anchors and default guide suppression. It checks deterministic full results,
strict ZIP rejection and document/page output bounds without replacing its
existing independent package assertions.
