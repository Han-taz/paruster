# Single Source of Truth

This directory contains the current, normative description of paruster. A code change that alters architecture, a public contract, quality policy, operations, or migration status must update the owning page in the same PR.

## Current documents

- [`migration/2026-09-29-rust-python-port-design.md`](migration/2026-09-29-rust-python-port-design.md): approved product and migration design
- [`migration/plans/2026-09-29-foundation-implementation-plan.md`](migration/plans/2026-09-29-foundation-implementation-plan.md): executable contract, workspace, binding, parity, and CI foundation plan
- [`contracts/errors.md`](contracts/errors.md): stable shared error code inventory and status semantics
- [`contracts/mcp.md`](contracts/mcp.md): frozen MCP schemas, envelopes, limits, security, and stdio requirements
- [`contracts/ir.md`](contracts/ir.md): recursive document IR, result, warning, and error wire contract
- [`contracts/python-api.md`](contracts/python-api.md): Python input, result, error, and public API translation contract
- [`contracts/compatibility-manifest.md`](contracts/compatibility-manifest.md): classified TypeScript-to-Python export and removal inventory
- [`quality/parity.md`](quality/parity.md): allowed normalization and parity evaluation policy

## Planned sections

- `architecture/`: current system, data flow, crate boundaries, security boundaries
- `contracts/`: IR, Python API, MCP tools, errors, compatibility manifest
- `components/`: format and feature implementation truth
- `quality/`: test matrix, parity gates, benchmarks
- `operations/`: development, CI, release
- `migration/`: roadmap and live migration status
- `decisions/`: numbered ADRs; supersede rather than rewrite accepted rationale
- `generated/`: checked generated reference derived from code and schemas

An empty planned section is not created until it has canonical content.
