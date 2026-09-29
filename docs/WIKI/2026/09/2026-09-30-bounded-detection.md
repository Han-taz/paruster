---
id: 2026-09-30-bounded-detection
date: 2026-09-30
status: recorded
component: kordoc-core
issue: null
pr: null
commit: 0144c8c
related_ssot:
  - docs/SSOT/components/detection.md
  - docs/SSOT/contracts/errors.md
related_decisions:
  - docs/SSOT/migration/2026-09-29-rust-python-port-design.md
---

# Bounded foundation format detection

## Context

The Python foundation needs content-based format detection before any individual parser can be advertised. Container inspection must remain deterministic and bounded for untrusted inputs.

## Work performed

Task 5 added ordered magic detection, refined ZIP/CFB classification, checked ZIP and ZIP64 central-directory preflight, size/count limits, and a core-local parse dispatch error that preserves the detected file type. The implementation retains a generated property-test regression seed and does not extract archive members.

## Evidence

- `cargo test -p kordoc-core --locked`: 7 unit, 7 contract, and 1 property test passed.
- `PYO3_PYTHON=/Users/shkoh/.local/bin/python3.11 cargo test --workspace --locked`: passed.
- Strict workspace Clippy and formatting checks passed.
- Independent SOL review covered classic 65,535-entry ZIPs, ZIP64 locator and disagreement cases, multidisk rejection, compressed extents, exact ZIP/OLE markers, and stable error dispatch.

## Outcome

`kordoc-core` now exposes bounded `detect_format`, foundation `try_parse` dispatch, the canonical `FileType`, and shared input/archive limits. All actual document parsers remain pending.

## Follow-ups

Expose the byte-only core through PyO3 and the bounded Python facade, then reuse the preflight path under the non-default fuzzing feature in the CI phase.
