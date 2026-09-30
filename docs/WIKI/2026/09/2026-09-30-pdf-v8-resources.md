---
id: 2026-09-30-pdf-v8-resources
date: 2026-09-30
status: locally-verified
component: pdf
issue: null
pr: null
commit: null
related_ssot: [docs/SSOT/components/pdf.md, docs/SSOT/migration/plans/2026-09-30-pdf-v8-resources-plan.md, docs/SSOT/migration/status.md]
related_decisions: [Embed PDF.js in V8; Rust owns execution and the remaining product implementation.]
---

# Embedded CMap/font factories retain the authored Korean probe

Luna B's authored CID/Helvetica PDF initially extracts only the ASCII text;
the baseline silently loses Korean glyphs because it has no CMap factory.
Custom PDF.js factories now return only exact allowlisted, embedded Rust bytes,
restoring `한글V8 resource probe`. Actual callback counters prove both CMap
and standard font supply rather than merely successful fallback extraction.

The 168 CMaps and 14 standard fonts total 1,940,006 bytes; all 182 name/path/
byte-count/SHA-256 mappings match the unchanged 188-file upstream provenance.
The 1,644-byte CC0 PDF has SHA-256
`62601a0563488196397e88773eadfd7e2259c56fa33777a797ce958045a72679`.
Its stdlib recipe is independently regenerated in a temporary directory by an
offline contract test. No oracle or JavaScript research helper is required.

Root/independent review caught oversized argument scanning and callbacks denied
before request accounting; both were fixed before publication. Every callback
now counts first and string length is checked before UTF-8 work. Inclusive
192 KiB item, 512 request and 8 MiB cumulative budgets precharge allocator-backed
payloads. Recorded rejection remains a typed failure despite PDF.js fallback.
All existing runtime limits, wire schema, fixtures and scoring are preserved.

The worker notice bundle adds CMap, Foxit and Liberation notices reused from
existing assets. The coordinator fetched the original npm tarball, checked its
pinned SHA-512 and verified all three member bytes: 36 notices/229,224 bytes.
Twenty helper tests include rejection of each missing resource notice. The
workflow adds a third resource smoke on all six targets and on the manylinux
baseline; no platform or prior probe is skipped.

Fresh root verification passes 179 PDF feature test executions, normal locked
workspace tests, strict all-target/all-feature workspace Clippy and rustdoc.
Independent review is CLEAN. The existing supervisor test's 100 ms marker
assertion failed during one worker run when startup missed the deadline; full
unchanged targets then pass in both root and independent runs. No deadline or
test assertion is relaxed. Hosted required checks remain decisive.

A freshly rebuilt macOS arm64 release-worker wheel passes exact architecture/
36-notice audit, all three installed PDF probes and 341 isolated CPython 3.10
API/contract/parity/tooling tests. Wheel and clean source-distribution artifact
audits pass; generated staging files are moved to ignored build storage.
Hosted six-target gates still follow before merge. This private checkpoint does not register PDF or complete layout/IR,
metadata/encryption/options/corpus or OS containment/ordinary build assembly.

Before PR publication, the candidate integrates current main after H0 fixture
PR #28 (`5a077eb78704b8b60e03963a22297978f9854d63`). The same freshly built
resource-worker wheel then passes all 345 current isolated Python tests, with
the expected duplicate-member warning and three notice subtests. Documentation
links/indexes and the authored diff against main also pass.
