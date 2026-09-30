---
id: 2026-09-30-hwpml-fixture-capture
date: 2026-09-30
status: locally-verified
component: hwpml
issue: null
pr: null
commit: null
related_ssot: [docs/SSOT/components/hwpml.md, docs/SSOT/migration/plans/2026-09-30-hwpml-fixture-plan.md, docs/SSOT/migration/status.md]
related_decisions: [Keep the migration oracle local and read-only; Rust owns new product implementation.]
---

# Freeze HWPML inputs before native implementation

Luna A authored eight small CC0 XML inputs and froze 13 complete results from
the public legacy API in a disposable copy of pinned oracle source. SOL
independently verified regeneration, input/capture inventories and digests,
oracle source pins and the generated 50 MiB+1 observation. The coordinator
added four ordinary offline contract tests; they pass without an oracle or
JavaScript runtime. The fixture README records reproducible bytes and honest
capture/runtime provenance.

The initial escaped DTD fixture was insufficient to test entity-reference
behavior. Before publication, a separate actual reference to a harmless
nonexistent file URI was added. Its result retains the reference and a
`MALFORMED_XML` warning, without claiming a universal XXE guarantee or zero
attempted I/O. A separate unclosed-tag case records sanitized `PARSE_ERROR`.
The original six input bytes and first 11 captures remain unchanged.

Capture JSONL SHA-256 is
`aba0dd587753d2c898340be9fe10e4dfd79a535c9f1356c4c83f0ede250551c6`.
The oversized input is a streamed hash/recipe; no 51 MiB file is committed.
Git preserves exact authored XML bytes on Windows. The one-time JS capture
helper stays under ignored `build/oracle-capture/`; it is not delivered or
required by tests. No parser code, shared contract, protected score, original
fixture/answer or security gate is modified.

Native bounded text/page/heading lowering, HWPML-specific tables, malformed
recovery and public Python qualification follow in separate reviewed slices.
Hosted required gates remain required before protected merge.
