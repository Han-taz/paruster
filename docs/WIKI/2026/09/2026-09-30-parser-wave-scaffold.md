---
id: 2026-09-30-parser-wave-scaffold
date: 2026-09-30
status: recorded
component: parser-wave
issue: null
pr: pending
commit: pending
related_ssot:
  - docs/SSOT/architecture/workspace.md
  - docs/SSOT/migration/plans/2026-09-30-hwpx-implementation-plan.md
  - docs/SSOT/migration/plans/2026-09-30-pdf-implementation-plan.md
  - docs/SSOT/migration/status.md
related_decisions:
  - Both parser crates enter the protected root workspace before behavior work.
  - Scaffold crates expose no parser entry points and are not production-registered.
---

# First parser wave scaffold

## Context

The approved P1/P2 plans require every parallel worker slice to compile and
test in the protected root workspace. Neither format crate existed after the
planning PR, so worker branches would otherwise escape locked workspace CI or
collide on root integration files.

## Work performed

- Registered minimal `kordoc-hancom` and `kordoc-pdf` workspace members.
- Added compilable module shells without publishing placeholder parser or
  validator result shapes.
- Kept each format crate dependent on `kordoc-ir` and independent of core,
  Python, the ignored oracle, Node, pdfjs, and PDFium.
- Pinned reviewed candidate XML, RustCrypto, and pure-Rust PDF dependencies for
  the planned security and substrate spikes.
- Recorded that `lopdf`'s eager loader is not yet an approved bounded substrate;
  P2a must pass the child plan's allocation/object/decompression exit gate or
  replace it before semantic implementation.
- Added scaffold tests and workspace dependency assertions without exposing a
  parser through the production registry or Python API.

## Evidence

- Rust workspace formatting, strict Clippy, tests, and warning-free rustdoc.
- Contract, documentation, dependency-license, audit, and artifact gates.
- Hosted CI/Security/Wheels/Fuzz/CodeQL evidence will be recorded in a separate
  append-only merge entry.

## Outcome

HWPX H0 and PDF P2a may start from the scaffold merge commit. HWPX H1a/H2a
start only after H0 freezes their private interfaces. Parser capability and
real-document parity remain pending.

## Follow-ups

- Add a separate append-only merge-evidence entry after the scaffold PR merges.
- Execute the HWPX and PDF child-plan task DAGs without changing the production
  registry until their full parity checkpoints pass.
