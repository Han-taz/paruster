---
id: 2026-09-30-hwpml-private-text-merge
date: 2026-09-30
status: merged
component: hwpml
issue: null
pr: https://github.com/Han-taz/paruster/pull/30
commit: 5a8e27c4d67a3cfcff9f31bb5e9dcc304fda6f34
related_ssot: [docs/SSOT/components/hwpml.md, docs/SSOT/migration/status.md]
related_decisions: [Private bounded Rust HWPML lowering precedes table/recovery qualification and public integration.]
---

# Private HWPML text lowering passes protected merge

PR [#30](https://github.com/Han-taz/paruster/pull/30) merges by squash at
2026-09-30 13:11:24 UTC as
`5a8e27c4d67a3cfcff9f31bb5e9dcc304fda6f34`. Exact final head
`fa8c46147d5013cd73c2382ea02bf371bfbbe6b1` passes required CI and all six
additional PDF-worker wheel targets after incorporating the resource-factory
mainline. Independent SOL and coordinator reviews are clean.

- [CI](https://github.com/Han-taz/paruster/actions/runs/36718613171): locked Rust, strict lints/docs, contracts, CPython 3.10–3.14, parity and coverage.
- [Security](https://github.com/Han-taz/paruster/actions/runs/36718613184): audit/deny, dependency/workflow checks and CodeQL.
- [Native wheels](https://github.com/Han-taz/paruster/actions/runs/36718613094): all six targets.
- [Bounded fuzzing](https://github.com/Han-taz/paruster/actions/runs/36718613068): all campaigns and aggregate.
- [PDF-worker wheels](https://github.com/Han-taz/paruster/actions/runs/36718613091): all six installed workers retain the three exact PDF probes.

Final local evidence includes Hancom 142 library and 144 integration test
executions, locked workspace tests, all-target/all-feature Clippy, warning-free
rustdoc, a fresh macOS ARM abi3 wheel/sdist with two artifact audits, 326 isolated
CPython 3.10 Python tests and 20 notice/helper tests. Three copied crate-local
XML inputs retain their H0 hashes, and no archive contains migration-oracle
source. This private lowerer remains unregistered; tables, malformed recovery,
public Python integration and full HWPML parity remain pending. Protected
scoring and original H0 fixtures/captures remain unchanged.
