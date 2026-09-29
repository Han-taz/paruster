---
id: 2026-09-30-parser-seam
date: 2026-09-30
status: recorded
component: parser-foundation
issue: null
pr: pending
commit: pending
related_ssot:
  - docs/SSOT/migration/plans/2026-09-30-parser-seam-implementation-plan.md
  - docs/SSOT/architecture/workspace.md
  - docs/SSOT/contracts/python-api.md
  - docs/SSOT/quality/parity.md
related_decisions:
  - docs/SSOT/migration/2026-09-29-rust-python-port-design.md
---

# Parser seam implementation

## Context

The foundation could detect formats but every parse attempt ended at `UNSUPPORTED_FORMAT`. Independent format crates needed an acyclic structural handoff, exact option ownership, a typed Python success surface, and a full-result harness.

## Work performed

P0 added an IR-owned `ParsedDocument`/options DTO, a core-owned private registry with validation ordering and panic containment, successful result assembly tests, the immutable Python `Document` projection, strict option translation, six legacy detector helpers, and a provenance-checked document harness. The production registry intentionally remains empty.

## Evidence

- TDD red runs exposed unresolved DTO/model/helper APIs and the native one-versus-two-argument option mismatch.
- Rust recursive IR, adversarial dispatch, panic, and detector-helper tests.
- Wheel-installed Python input/options/model/helper/parity tests.
- Exact outputs for all six compatibility helpers were captured from the pinned TypeScript runtime across synthetic signature, malformed-container, deterministic ZIP, and generated CFB cases; no oracle fixture bytes were copied.
- Source-contract empty-input smoke is excluded from oracle parity; successful oracle cases remain zero.

## Outcome

The seam is locally implemented without claiming any real parser or MCP handler. Hosted PR evidence and exact commit are pending.

## Follow-ups

- Append a merge evidence record instead of rewriting this entry after merge.
- P7 supplies real Markdown/page projection; P1 supplies the first successful format parser and oracle capture.
