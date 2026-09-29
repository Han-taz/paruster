---
id: 2026-09-30-parser-wave-scaffold-merge
date: 2026-09-30
status: recorded
component: parser-wave
issue: null
pr: https://github.com/Han-taz/paruster/pull/14
commit: 86e4b78ce0fd2f40a238910eca46733fa84d317a
related_ssot:
  - docs/SSOT/architecture/workspace.md
  - docs/SSOT/migration/status.md
related_decisions:
  - The protected parser-wave scaffold is merged and HWPX H0 and PDF P2a may start.
  - Scaffold membership does not claim parser, Python, parity, or MCP completion.
---

# First parser wave scaffold merge evidence

## Context

This append-only follow-up records the hosted evidence omitted from the
pre-merge scaffold entry. PR #14 merged through the protected GitHub Flow after
all required and informational checks completed successfully.

## Merged result

- PR: [#14](https://github.com/Han-taz/paruster/pull/14)
- Squash commit: `86e4b78ce0fd2f40a238910eca46733fa84d317a`
- Merged at: `2026-09-29T20:30:34Z` (`2026-09-30` Asia/Seoul)
- HWPX and PDF crates are protected root-workspace members.
- Neither crate exposes a parser entry point or production registration.

## Hosted evidence

- [CI](https://github.com/Han-taz/paruster/actions/runs/36626307802): Rust,
  contracts, docs, golden, coverage, and CPython 3.10-3.14 passed.
- [Security](https://github.com/Han-taz/paruster/actions/runs/36626307873):
  CodeQL for Rust, Python, and Actions plus dependency review, cargo-audit,
  cargo-deny, actionlint, and zizmor passed.
- [Native wheels](https://github.com/Han-taz/paruster/actions/runs/36626307868):
  six Linux, macOS, and Windows architecture targets passed.
- [Bounded fuzzing](https://github.com/Han-taz/paruster/actions/runs/36626307825):
  all four existing targets and `fuzz-gate` passed.
- Required `ci-gate`, `security-gate`, `wheels-gate`, `fuzz-gate`, and CodeQL
  checks all passed before squash merge.

## Outcome

The HWPX H0 interface/fixture task and PDF P2a substrate gate are unblocked.
The real-document oracle-success numerator remains zero; all parser and MCP
capabilities remain pending until their implementation and parity gates merge.
