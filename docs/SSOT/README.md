# Single Source of Truth

This directory contains the current, normative description of paruster. A code change that alters architecture, a public contract, quality policy, operations, or migration status must update the owning page in the same PR.

## Current documents

- [`migration/2026-09-29-rust-python-port-design.md`](migration/2026-09-29-rust-python-port-design.md): approved product and migration design
- [`migration/plans/2026-09-29-foundation-implementation-plan.md`](migration/plans/2026-09-29-foundation-implementation-plan.md): executable contract, workspace, binding, parity, and CI foundation plan
- [`migration/plans/2026-09-30-product-port-implementation-plan.md`](migration/plans/2026-09-30-product-port-implementation-plan.md): full parser, transform, Python, MCP, packaging, and Node-removal execution DAG
- [`migration/plans/2026-09-30-parser-seam-implementation-plan.md`](migration/plans/2026-09-30-parser-seam-implementation-plan.md): focused P0 internal DTO, dispatch, Python success model, detector-helper, and parity-harness plan
- [`migration/plans/2026-09-30-ir-projections-implementation-plan.md`](migration/plans/2026-09-30-ir-projections-implementation-plan.md): focused P7 Markdown, page/chunk, table policy, and shared table-unit execution plan
- [`migration/plans/2026-09-30-hwpx-implementation-plan.md`](migration/plans/2026-09-30-hwpx-implementation-plan.md): focused P1 bounded HWPX package, XML, semantic lowering, validation, crypto, and Python integration plan
- [`components/hwpx.md`](components/hwpx.md): private HWPX package, XML, and section implementation candidate and current review boundaries
- [`migration/plans/2026-09-30-pdf-implementation-plan.md`](migration/plans/2026-09-30-pdf-implementation-plan.md): focused P2 pure-Rust PDF semantic, layout, table, quality, and optional raster-boundary plan
- [`migration/status.md`](migration/status.md): live implementation and parity status
- [Workspace architecture](architecture/workspace.md): current crate boundaries, data flow, and security boundaries
- [`contracts/errors.md`](contracts/errors.md): stable shared error code inventory and status semantics
- [`contracts/mcp.md`](contracts/mcp.md): frozen MCP schemas, envelopes, limits, security, and stdio requirements
- [`contracts/ir.md`](contracts/ir.md): recursive document IR, result, warning, and error wire contract
- [`contracts/python-api.md`](contracts/python-api.md): Python input, result, error, and public API translation contract
- [`contracts/compatibility-manifest.md`](contracts/compatibility-manifest.md): classified TypeScript-to-Python export and removal inventory
- [`components/detection.md`](components/detection.md): bounded format detection, container preflight, and foundation dispatch rules
- [`components/normalization.md`](components/normalization.md): bounded Markdown, page, chunk, and table-policy projection semantics
- [`components/pdf.md`](components/pdf.md): private bounded PDF object-reader decision, supported surface, and security evidence
- [`quality/parity.md`](quality/parity.md): allowed normalization and parity evaluation policy
- [Operations guide](operations/README.md): index for the current [development](operations/development.md) and [release](operations/release.md) procedures

## Organization

The current SSOT is organized into `architecture/` — current system boundaries
and data flow; `contracts/` for shared interfaces; `components/` for
implementation truth; `quality/` for validation policy; `operations/` —
development and release procedures; and `migration/` for plans and status. New
sections are added only when they have canonical content.
