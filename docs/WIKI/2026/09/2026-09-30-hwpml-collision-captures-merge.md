---
id: 2026-09-30-hwpml-collision-captures-merge
date: 2026-09-30
status: merged
component: hancom
issue: null
pr: https://github.com/Han-taz/paruster/pull/35
commit: 829b46ad8a70eac20e8229d7d6faf99137227649
related_ssot: [docs/SSOT/components/hwpml.md, docs/SSOT/migration/status.md]
related_decisions: [Record source ambiguity without weakening private Rust attachment policy.]
---

# Separate collision evidence passes protected merge

PR #35 squash-merges at 2026-09-30 14:34:18 UTC as
`829b46ad8a70eac20e8229d7d6faf99137227649`. Final head
`e990960f916c25f252a88a0625de54a8ba2571b6` passes all five required
aggregates, actual CodeQL and six installed worker targets. Independent review
is clean. Fresh main34 base wheel tests pass 362 cases plus ten subtests;
artifact, offline recipe/result inventory and autocrlf bytepin checks pass.
No native parser, protected H0 fixture, scorer or runtime behavior changes.

- [CI](https://github.com/Han-taz/paruster/actions/runs/36729167293)
- [Security](https://github.com/Han-taz/paruster/actions/runs/36729167112)
- [Native wheels](https://github.com/Han-taz/paruster/actions/runs/36729167375)
- [Bounded fuzzing](https://github.com/Han-taz/paruster/actions/runs/36729167171)
- [PDF worker wheels](https://github.com/Han-taz/paruster/actions/runs/36729167265)
