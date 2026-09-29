---
id: 2026-09-30-product-port-planning
date: 2026-09-30
status: recorded
component: migration
issue: null
pr: pending
commit: pending
related_ssot:
  - docs/SSOT/migration/plans/2026-09-30-product-port-implementation-plan.md
related_decisions:
  - docs/SSOT/migration/2026-09-29-rust-python-port-design.md
---

# Full product port planning

## Context

The Rust/Python foundation merged without implementing document parsers or the frozen MCP handlers. The remaining oracle is a 64,745-line TypeScript product with 87 value exports, 112 type exports, 17 MCP tools, and 238 test/fixture files.

## Work performed

The coordinator and three read-only audits mapped the parser families, PDF/OCR and service layers, frozen API/tool inventory, security limits, licenses, and dependency order. The resulting SSOT plan defines twenty independently reviewable PR tasks, a four-slot managed execution schedule, exact crate ownership, red-green steps, parity evidence, and the final Node-removal gate.

## Evidence

- `contracts/public-api.json`, `contracts/mcp-tools.json`, and `contracts/ir-schema.json`
- Local oracle source and test inventory read without copying or modifying `kordoc/`
- Failing-first contract test `tests/contracts/test_product_port_plan.py`

## Outcome

Product work begins with the coordinator-owned successful parser seam. Hancom, PDF/OCR, Office, and MCP transport then proceed in dependency-safe parallel waves.

## Follow-ups

- Replace the pending PR and commit fields with a new append-only follow-up entry after merge; do not rewrite this record after it merges.
- Execute P0 through P19 and update the live migration status only with merged evidence.
