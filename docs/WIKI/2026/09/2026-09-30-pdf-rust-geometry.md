---
id: 2026-09-30-pdf-rust-geometry
date: 2026-09-30
status: recorded
component: pdf
issue: null
pr: null
commit: null
related_ssot: [docs/SSOT/components/pdf.md, docs/SSOT/migration/status.md]
related_decisions: [Keep V8 engine evidence and project base geometry in Rust.]
---

# Fractional CropBox projection in Rust

A new CC0 PDF distinguishes round-before-shift from shift-before-round and
retains 100 by 200 dimensions at page rotation 90. The pinned V8 DTO and focused
source projection agree on the base position (1.75, 1.25). Pinned engine vectors
cover negative zero, both tie signs and the float below 0.5. Genuine missing-API
RED precedes nine focused GREEN tests; all 252 scoped feature executions and
strict Clippy pass independent review. The offline contract separately pins
recipe/input/captures/vector bytes, stdlib regeneration and malformed-byte
rejection. This helper allocates no input-sized data and changes no DTO, worker
wire or public contract. Broader item/layout/operator parity remains pending.

## Main34 artifacts and unchanged supervisor timing investigation

Fresh locked default workspace tests, strict all-feature Clippy/rustdoc,
format/Ruff/mypy/docs checks pass. A fresh base wheel/sdist passes both audits;
isolated Python tests pass 362 cases plus ten subtests. Five provenance bytepins
survive real autocrlf=true checkout. Release native geometry/metadata checks
pass 9 and 10 tests, respectively. The six worker builders now execute those
release native tests in addition to all four existing installed probes.

The first local full feature run failed the unchanged supervisor sleep test's
100ms child-start marker assertion. Focused Cargo invocation also reproduced
it; the same binary directly passed. Cargo variants, paths, working directory,
printed environment, process group and nice values did not explain the
difference. Temporary entry/writer instrumentation was restored byte-exact.
All 13 unchanged test executables directly passed 252 cases, then the complete
original Cargo feature command passed 252 cases including 40 supervisor tests.
Cold process startup is a hypothesis; the OS mechanism remains unproven.
No source, timeout, security limit, fixture or gate was changed to obtain this
result. Logs are retained locally for the later process-containment checkpoint.
