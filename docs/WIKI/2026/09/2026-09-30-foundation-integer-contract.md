---
id: 2026-09-30-foundation-integer-contract
date: 2026-09-30
status: recorded
component: ir-contract
issue: null
pr: null
commit: null
related_ssot:
  - docs/SSOT/contracts/ir.md
related_decisions:
  - docs/SSOT/migration/2026-09-29-rust-python-port-design.md
---

# Foundation integer wire contract correction

## Context

The frozen TypeScript-derived schema represented every TypeScript `number` as a generic JSON number. Task 4's Rust IR correctly modeled semantic page identifiers, structural spans, and counters as `u32`, exposing a mismatch for negative, fractional, and out-of-range schema inputs.

## Work performed

The coordinator narrowed only the Task 4 foundation fields backed by Rust `u32` to JSON `integer` with the inclusive unsigned 32-bit range. Floating-point coordinates, ratios, scores, and unrelated planned definitions remain JSON numbers. A contract test enumerates the affected definitions and fields.

## Evidence

- `tests/contracts/test_contract_inventory.py::test_foundation_semantic_counters_are_unsigned_32_bit_integers`
- `contracts/ir-schema.json`
- `crates/kordoc-ir/src/document.rs`

## Outcome

The machine-readable schema, Rust serde model, and planned Python integer representation now accept the same numeric domain for the implemented foundation surface.

## Follow-ups

Apply the same explicit cross-language review to later numeric fields as their Rust implementations land; do not mechanically narrow floating-point measurements or scores.
