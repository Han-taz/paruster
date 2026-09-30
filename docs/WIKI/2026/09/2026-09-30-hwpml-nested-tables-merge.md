---
id: 2026-09-30-hwpml-nested-tables-merge
date: 2026-09-30
status: merged
component: hancom
issue: null
pr: https://github.com/Han-taz/paruster/pull/33
commit: e0fde7db6ed97914ed78483fd2b5f0a58a5d99a3
related_ssot: [docs/SSOT/components/hwpml.md, docs/SSOT/migration/status.md]
related_decisions: [Private unique-anchor table lowering preserves frozen nested block trees.]
---

# Private nested tables pass protected merge

PR #33 squash-merges at 2026-09-30 14:07:25 UTC as
`e0fde7db6ed97914ed78483fd2b5f0a58a5d99a3`. Exact final head
`1ecd58ac209d9775b6f434f914f692aa53bc3d41` passes all five required
aggregates, Rust/Python/Actions CodeQL and all six PDF worker targets.
Independent native review is clean. Three complete block trees and Markdown
projections match the frozen H0 observations. Fresh installed wheel tests pass
358 cases plus ten subtests; wheel and sdist audits pass. Ambiguous attachments
and unsupported wrappers deliberately reject. Public registration and broader
recovery/corpus parity remain pending.

- [CI](https://github.com/Han-taz/paruster/actions/runs/36725757673)
- [Security](https://github.com/Han-taz/paruster/actions/runs/36725757594)
- [Native wheels](https://github.com/Han-taz/paruster/actions/runs/36725757729)
- [Bounded fuzzing](https://github.com/Han-taz/paruster/actions/runs/36725757733)
- [PDF worker wheels](https://github.com/Han-taz/paruster/actions/runs/36725759000)
