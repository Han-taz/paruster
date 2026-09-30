---
id: 2026-09-30-pdf-worker-windows-utf8
date: 2026-09-30
status: locally-verified
component: pdf
issue: null
pr: https://github.com/Han-taz/paruster/pull/27
commit: null
related_ssot: [docs/SSOT/components/pdf.md, docs/SSOT/migration/status.md]
related_decisions: []
---

# Explicit UTF-8 under isolated installed-worker verification

The [second worker-wheel run](https://github.com/Han-taz/paruster/actions/runs/36712327283)
passes both Linux and both macOS targets. Both Windows targets now pass worker
build, wheel packaging, architecture/notice audit, install and Helvetica smoke.
Their Unicode probe fails while printing the successful Hangul/astral result:
Python reports a legacy `charmap` encoding error. No extraction mismatch is
reported. The aggregate gate correctly fails.

This corrects the environment-based UTF-8 assumption in the earlier
[Windows checkout record](2026-09-30-pdf-worker-windows-checkout.md).
Python isolated mode (`-I`) ignores `PYTHONUTF8`. All four native/container
probe commands now preserve isolation and explicitly select `-X utf8`.
A subprocess regression invokes the actual helper status formatter with both
flags and asserts exact UTF-8 bytes. Independent review, 19 helper tests and
339 isolated installed-wheel API/contract/parity/tooling tests pass locally.

The general Python CI also catches Ruff 0.16.9 UP022 in the new Git-attribute
regression; using `capture_output=True` resolves it without changing the test.
Pinned Ruff check/format and actionlint pass. No fixture, expected answer,
notice, native code, cap, target, scoring or quality gate is changed.
Hosted checks on this new head remain required before merge.
