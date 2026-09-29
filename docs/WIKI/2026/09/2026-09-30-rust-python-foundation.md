---
id: 2026-09-30-rust-python-foundation
date: 2026-09-30
status: recorded
component: rust-python-foundation
issue: null
pr: https://github.com/Han-taz/paruster/pull/4
commit: b62d3a2
related_ssot:
  - docs/SSOT/migration/status.md
  - docs/SSOT/components/detection.md
  - docs/SSOT/contracts/python-api.md
  - docs/SSOT/quality/parity.md
related_decisions:
  - docs/SSOT/migration/2026-09-29-rust-python-port-design.md
  - docs/SSOT/migration/plans/2026-09-29-foundation-implementation-plan.md
---

# Rust and Python foundation delivery

## Context

PR [#4](https://github.com/Han-taz/paruster/pull/4) delivers the first executable migration slice after the contract freeze. It must establish safe, typed packaging and detector-level evidence without claiming that document parsers or MCP handlers already exist.

## Work performed

- Added the Rust workspace and CPython abi3 package supporting Python 3.10 and newer stable releases.
- Implemented recursive serde IR/error contracts, strict omission/null behavior, and JSON Schema-compatible unsigned integer decoding.
- Implemented bounded magic/container detection, single-disk ZIP/ZIP64 preflight, CFB stream refinement, and foundation parse failure dispatch.
- Added the bounded Python input facade, 13 typed exceptions, immutable serializable result snapshots, native stubs, and typed package exports.
- Added a CC0 synthetic PDF detector golden, manifest confinement, lexical attribute-only XML normalization, timestamp allowlisting, and deterministic RFC 6901 first-difference reporting.

## Evidence

- Independent SOL specification/quality reviews approved Tasks 4 through 7.
- `uv run --python 3.10 pytest -q`: 87 passed.
- Ruff check and format, mypy, locked Rust workspace tests, strict Clippy, Cargo formatting, and diff checks passed.
- Release-mode abi3 wheel SHA-256: `bb0a1924705a95f012bacca5c9145d75319c8a242be76f862f8a7e6e33bd6d4a`.
- Source distribution SHA-256: `530d1816ed35023717a7c2f0be10ea876c87c636029751915ef7c757bd2e938b`.
- Wheel contains the native extension, Python sources, typing marker/stub, license, metadata, and CycloneDX SBOM; wheel and sdist contain no TypeScript, `node_modules`, or ignored oracle source.

## Outcome

The installable foundation can classify supported container families safely and expose stable typed failures through Python. Detector evidence is reproducible without the local oracle checkout.

## Follow-ups

- Tasks 8-10 add mandatory GitHub Actions, artifact scanning, coverage, fuzzing, multi-platform wheels, security workflows, SBOM/attestation, and branch protection checks.
- Every real parser, transformation, renderer, OCR flow, and all 17 MCP handlers remain pending and must pass format-specific oracle/golden parity before being marked complete.
