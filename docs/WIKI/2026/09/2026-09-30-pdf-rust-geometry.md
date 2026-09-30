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
