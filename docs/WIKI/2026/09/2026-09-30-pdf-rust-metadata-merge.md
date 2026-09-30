---
id: 2026-09-30-pdf-rust-metadata-merge
date: 2026-09-30
status: merged
component: pdf
issue: null
pr: https://github.com/Han-taz/paruster/pull/34
commit: bc61d32a8fbf2fdea2e2ace3e36104d3eb1e56c6
related_ssot: [docs/SSOT/components/pdf.md, docs/SSOT/migration/status.md]
related_decisions: [Normalize bounded upstream Info metadata in Rust.]
---

# Rust metadata passes six-target merge

PR #34 squash-merges at 2026-09-30 14:24:45 UTC as
`bc61d32a8fbf2fdea2e2ace3e36104d3eb1e56c6`. Final head
`f1a096821d2f305ceef0fbba09a533e7443c4de2` passes all five required
aggregates, Rust/Python/Actions CodeQL and six installed PDF worker targets.
Independent native review is clean. Three CC0 PDFs/six frozen metadata
projections, ten focused native cases and 243 scoped feature executions pass.
Fresh combined worker wheel passes 360 isolated Python/helper cases plus ten
subtests; base wheel/sdist audits and installed kind-4 probe pass. DTO, wire and
public APIs remain unchanged. Geometry/layout/corpus, production containment
and ordinary worker/source-build assembly remain separate checkpoints.

- [CI](https://github.com/Han-taz/paruster/actions/runs/36727795634)
- [Security](https://github.com/Han-taz/paruster/actions/runs/36727796082)
- [Native wheels](https://github.com/Han-taz/paruster/actions/runs/36727795966)
- [Bounded fuzzing](https://github.com/Han-taz/paruster/actions/runs/36727795504)
- [PDF worker wheels](https://github.com/Han-taz/paruster/actions/runs/36727795710)
