---
id: 2026-09-30-pdf-v8-worker-gates
date: 2026-09-30
status: recorded
component: pdf-v8-worker
issue: null
pr: null
commit: null
related_ssot:
  - docs/SSOT/components/pdf.md
  - docs/SSOT/migration/plans/2026-09-30-pdf-v8-worker-plan.md
related_decisions: []
---

# Private worker final local gates

## Context

The [worker checkpoint record](2026-09-30-pdf-v8-worker.md) preserves initial
RED/GREEN implementation evidence. Later review exposed two response-decoding
issues and an I/O-thread cleanup issue; this entry records the corrections.

## Work performed

Borrow-only diagnostics rejected the writer's valid escaped quotes/backslashes.
A custom string visitor now retains only 512 sanitized characters while
handling Unicode/JSON escapes. Internally tagged serde enums first buffer their
content; the pinned derive source confirmed this. The receiver now uses a
direct envelope struct, so the bounded page visitor executes while decoding
rather than after buffering a hostile subtree. A result-first/status-last
near-4MiB frame proves only the first 200 strings are visited; page 201 is
rejected without materialization.

The supervisor joins all I/O threads even if the first panics, replacing a
short-circuit `.all()` pattern. Both the old join and EOF/exit race were run
RED and fixed GREEN. The new Unicode artifact uses canonical 20-byte CRLF xref
records; the Rust reconstruction verifies their width and exact bytes. Its
valid binary comment, recipe/provenance and hash are checked without a Git
attribute exception. Original fixtures and expected text remain unchanged.

## Evidence

After integrating protected HWPX PR #24 main, coordinator-run scoped
`cargo test -p kordoc-pdf --features pdfjs-worker-tests --locked` passes 135
executions across nine groups. Counts include repeated path-module unit tests,
so they are not claimed as 135 distinct cases. Real supervised Helvetica and
Hangul/astral probes pass, as do abort, blocked I/O, EOF/exit, caps, malformed
frames and concurrency cases.

Locked default workspace tests, all-target/all-feature warning-denied Clippy,
fmt, warning-free Rust docs and diff checks pass. All 317 Python/contract/parity
tests pass using the CI pytest console invocation. Ruff/formatting, mypy, docs,
actionlint, pinned zizmor, dependency policy/audit and two default wheel/sdist
artifact scans pass. The existing V8 paste unmaintained advisory is preserved
with no ignore. The base wheel has no worker/PDF registration; its successful
build is not evidence of six-platform V8 worker packaging.

## Outcome

Independent SOL review is CLEAN for this private checkpoint. Required hosted
gates must pass on the actual final feature head before protected squash merge.
No public schema, original fixture answer, security limit or scoring is changed.

## Follow-ups

OS memory/process-group limits, fixed sibling resolution, development/sdist
worker builds and six installed worker wheels remain pending, along with PDF
resource factories, complete host compatibility and full-result semantic parity.
