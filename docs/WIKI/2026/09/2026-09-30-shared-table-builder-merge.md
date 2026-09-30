---
id: 2026-09-30-shared-table-builder-merge
date: 2026-09-30
status: merged
component: normalization
issue: null
pr: https://github.com/Han-taz/paruster/pull/31
commit: a8956457949f82e9a22281b5580ef1be4ee5c229
related_ssot: [docs/SSOT/architecture/workspace.md, docs/SSOT/components/normalization.md, docs/SSOT/migration/status.md]
related_decisions: [IR-only shared table sibling avoids a Hancom/core dependency cycle.]
---

# Shared bounded table construction passes protected merge

PR [#31](https://github.com/Han-taz/paruster/pull/31) squash-merges at
2026-09-30 13:35:11 UTC as `a8956457949f82e9a22281b5580ef1be4ee5c229`.
Final head `a9e9b392e77fd5bf0a3d39ecc4f044134810dc33` passes all required
aggregate gates, Rust/Python/Actions CodeQL and all six PDF worker wheels.
Independent SOL and coordinator reviews are clean.

- [CI](https://github.com/Han-taz/paruster/actions/runs/36721838971)
- [Security](https://github.com/Han-taz/paruster/actions/runs/36721839124)
- [Native wheels](https://github.com/Han-taz/paruster/actions/runs/36721838702)
- [Bounded fuzzing](https://github.com/Han-taz/paruster/actions/runs/36721839051)
- [Six PDF worker wheels](https://github.com/Han-taz/paruster/actions/runs/36721838932)

The extracted code remains inside the unchanged required 80% coverage gate;
local selected coverage passes at 91.75%. A temporary differential harness
matches full old/new IR or typed errors for 10,000 deterministic inputs.
Fresh macOS ARM abi3 wheel tests pass 348 cases plus three subtests and both
artifact audits. Six source/fixture files retain exact sdist bytes. This merge
changes no public IR or error contract and does not implement HWPML tables.
