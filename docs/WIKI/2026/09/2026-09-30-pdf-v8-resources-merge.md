---
id: 2026-09-30-pdf-v8-resources-merge
date: 2026-09-30
status: merged
component: pdf
issue: null
pr: https://github.com/Han-taz/paruster/pull/29
commit: f7528a6a40b1df80617c961f2225c40eeb4b7f68
related_ssot: [docs/SSOT/components/pdf.md, docs/SSOT/migration/status.md]
related_decisions: [Embed PDF.js in V8; Rust owns execution and remaining product code.]
---

# Embedded PDF resources pass all six installed worker targets

PR [#29](https://github.com/Han-taz/paruster/pull/29) merged by protected
squash at 2026-09-30 12:57:09 UTC as
`f7528a6a40b1df80617c961f2225c40eeb4b7f68`. Independent runtime/budget/
provenance review is clean. The exact final head
`11fbabd35a126debe98547427da257e9fe28bcc5` passes all required checks and the
separate six-target installed PDF-worker matrix. The coordinator checks the
exact head before merge and verifies the resulting merged state afterward.

Hosted evidence:

- [CI](https://github.com/Han-taz/paruster/actions/runs/36717402702): locked Rust, strict Clippy/docs, CPython 3.10–3.14, contracts, parity and coverage.
- [Security](https://github.com/Han-taz/paruster/actions/runs/36717402703): dependency review, audit/deny, workflow scans and CodeQL.
- [Native wheels](https://github.com/Han-taz/paruster/actions/runs/36717402649): all six base-wheel targets.
- [Bounded fuzzing](https://github.com/Han-taz/paruster/actions/runs/36717402723): all six campaigns and aggregate gate.
- [PDF-worker wheels](https://github.com/Han-taz/paruster/actions/runs/36717402757): all six native targets pass architecture, 36-notice inventory, installed ASCII/Unicode/resource probes; Linux also runs under manylinux 2.28.

The fixture requires actual CMap and standard-font callbacks and returns exact
`한글V8 resource probe`. Request/item/cumulative resource caps and sticky typed
rejection remain enforced. Existing legacy worker frames stay unchanged.
Windows ARM64 remains a CPython 3.12 feasibility execution, with native 3.10
support unproven there. The checkpoint remains private: PDF public registration,
full IR/layout/metadata/options/corpus parity, OS containment and ordinary
worker assembly are still separate work. No frozen oracle answer or scoring
rule changes.
