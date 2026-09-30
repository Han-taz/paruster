---
id: 2026-09-30-pdf-v8-worker
date: 2026-09-30
status: recorded
component: pdf-v8-worker
issue: null
pr: null
commit: null
related_ssot:
  - docs/SSOT/components/pdf.md
  - docs/SSOT/migration/plans/2026-09-30-pdf-v8-worker-plan.md
  - docs/SSOT/migration/status.md
related_decisions: []
---

# One-document native PDF.js worker checkpoint

## Context

The user's PDF.js-in-V8 instruction is preserved. The preceding private runtime
merged in PR [#23](https://github.com/Han-taz/paruster/pull/23) as
`df7d4c9167e534c7ee44d5609f6f5babfcd2993d` after required hosted gates passed.
Its in-process feasibility limits did not contain fatal native abort/OOM. A SOL
manager and two Luna workers separately owned the real binary/integration,
framing/Unicode fixture and supervisor/hostile-child helper. The coordinator
owned Cargo/module wiring, CI and SSOT/publication.

## Work performed

The optional `pdfjs-worker` binary reads one bounded KPDF v1 request, reuses the
existing private V8 probe and emits one bounded result/error frame. A Rust
supervisor supplies an explicit trusted absolute path, two worker slots,
monotonic deadline monitoring, bounded concurrent pipe I/O and kill/wait/join
cleanup. Stderr never enters the protocol. No public parser/API/schema or
production wheel placement is added.

A separate test-only executable exercises blocked pipes, abort/nonzero exit,
flooding, truncation, sleep and EOF-before-exit behavior. None of those controls
is present in the production worker. The authored CC0 Type0/Identity-H fixture
uses embedded ToUnicode to produce exact `한글🧪` without external resources.
The Python recipe and independent Rust byte reconstruction agree; normal
builds/tests do not execute Python or Node to generate it.

## Evidence

- Eight initial protocol tests ran genuinely RED against unimplemented framing
  stubs, then GREEN. Review-driven failures exposed page-count mismatches and
  an oversized empty-page array: custom decoding now rejects page 201 before
  materializing it and validates count equality.
- Initial supervisor RED included absent implementation; the deterministic
  EOF-before-exit regression was also run against the old Option-taking loop
  and failed with `PDF.js worker output failed`. The corrected loop retains
  response/exit values until both are ready.
- Review strengthened the blocked-stdin case to 1 MiB, waits for both active
  children before probing a third slot, and joins every I/O thread even if an
  earlier join failed. Existing runtime input/output/heap/buffer/watchdog caps
  remain unchanged.
- Actual Cargo-built worker extraction compares exactly to direct V8 for both
  Helvetica and ToUnicode Unicode probes. Malformed/wrong-magic/over-cap frames
  return bounded typed errors; an aborting child leaves the parent alive.
- Initial Unicode extraction and exact recipe reconstruction already passed
  with the existing host shim; this is new evidence, not an invented RED claim
  or a broad Unicode/font/resource guarantee.

## Outcome

This checkpoint proves private native-process supervision for two synthetic
probes. Final full local and protected hosted gates are recorded after they
complete. Default wheels expose no PDF parser and do not bundle the worker.

## Follow-ups

Production still requires OS-specific memory/process-group containment,
calibrated peak RSS, fixed sibling binary resolution, six target-correct
installed worker wheels, and equivalent development/sdist builds. Process
separation alone does not enforce machine-wide memory or strict spawn/scheduler
latency. CMap/font factories, complete host semantics, metadata/operators,
IR/layout/options/encryption and representative full-result parity remain
pending. The ignored oracle was neither modified nor required by code,
fixtures, builds, tests, packaging or CI.
