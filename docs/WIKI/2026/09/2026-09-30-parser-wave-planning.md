---
id: 2026-09-30-parser-wave-planning
date: 2026-09-30
status: recorded
component: migration
issue: null
pr: pending
commit: pending
related_ssot:
  - docs/SSOT/migration/plans/2026-09-30-hwpx-implementation-plan.md
  - docs/SSOT/migration/plans/2026-09-30-pdf-implementation-plan.md
related_decisions:
  - A protected coordinator scaffold registers both parser crates before worker branches start.
  - P1 uses strict validated ZIP access and does not weaken the generic detector for corrupt central-directory recovery.
  - P2 keeps semantic parsing in pure Rust and confines optional PDFium use to raster and OCR boundaries.
---

# First parser wave planning

## Context

P7 shared projections merged and unblocked the first real-document parser wave.
HWPX and PDF can proceed in parallel, but each is large enough to require a
focused child plan and explicit shared-file ownership before implementation.

## Work performed

- Audited the ignored TypeScript oracle for HWPX package, XML, encryption,
  validation, metadata, page, image, note, table, and partial-parse behavior.
- Audited the PDF oracle for semantic extraction, layout, tables, quality,
  images, links, page selection, OCR boundaries, and resource limits.
- Split format-crate work from coordinator-owned workspace, lockfile, dispatch,
  public contract, Python binding, status, and shared workflow integration.
- Required a protected scaffold PR to register both crates and compilable module
  shells in the root workspace before Luna worker branches are cut, so every
  slice runs the normal locked workspace CI.
- Chose strict ZIP validation over unsafe archive recovery for P1 and a
  pure-Rust semantic parser with an optional raster-only PDFium boundary for P2.
- Defined SOL-managed, Luna-executed task DAGs in the two focused SSOT plans.

## Evidence

- P7 merge prerequisite: PR #11, commit
  `d473bae79f1de6f1aced29c59609a2cb49f0aa90`.
- P7 merge evidence documentation: PR #12, commit
  `141afa9dad8d9e6d91a224b2e87485ef4653135c`.
- The repository root ignores `/kordoc/`; the oracle remains untracked and is
  excluded from builds, packages, tests, CI artifacts, and runtime behavior.
- Plan verification is enforced by repository contract tests and the docs link
  checker before this planning branch can merge.

## Outcome

P1 and P2 have disjoint implementation boundaries and explicit serialized
coordinator checkpoints. After the planning PR, a protected parser-wave
scaffold must merge; implementation branches start from that scaffold commit
and can proceed concurrently without sharing worker-owned files.

## Follow-ups

- Add a separate append-only merge-evidence entry with the final PR, commit,
  and hosted checks; do not rewrite this planning record after merge.
- Execute P1 HWPX package/XML security foundations before semantic lowering is
  integrated.
- Execute the P2 bounded backend spike before committing to the full semantic
  and layout implementation.
- Record implementation, review, hosted CI, and merge evidence in new WIKI
  entries; do not rewrite this record after merge.
