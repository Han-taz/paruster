# Single Source of Truth

This directory contains the current, normative description of paruster. A code change that alters architecture, a public contract, quality policy, operations, or migration status must update the owning page in the same PR.

## Current documents

- [`migration/2026-09-29-rust-python-port-design.md`](migration/2026-09-29-rust-python-port-design.md): approved product and migration design

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
