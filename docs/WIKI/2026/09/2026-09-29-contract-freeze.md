---
id: 2026-09-29-contract-freeze
date: 2026-09-29
status: recorded
component: shared-contracts
issue: null
pr: https://github.com/Han-taz/paruster/pull/3
commit: 8c9f45d
related_ssot:
  - docs/SSOT/contracts/ir.md
  - docs/SSOT/contracts/python-api.md
  - docs/SSOT/contracts/compatibility-manifest.md
  - docs/SSOT/quality/parity.md
related_decisions: []
---

# Python, IR, and parity contract freeze

## Context

Task 2 of the foundation plan freezes the TypeScript-to-Python API classification, recursive IR wire shape, and parity normalization policy before parser work. PR #3 carries the preceding Task 1 error/MCP inventories and its source evidence: https://github.com/Han-taz/paruster/pull/3.

## Work performed

- Captured every named value and type export from `kordoc/src/index.ts`, including multiline type-only re-exports; classified every export in `contracts/public-api.json`.
- Classified `parse` as the foundation API shell that returns unsupported until parser implementations land. Format parsers and all other library behaviors remain planned. The Node CLI/package entry machinery is explicitly removed; `filePath` is internal.
- Captured the recursive JSON wire model, optionality, names, enums, results, warnings, and errors in `contracts/ir-schema.json`, including supporting recursive types not separately exported by the barrel.
- Recorded only ZIP timestamps and XML attribute ordering as parity normalizations. Oracle answers, scoring, benchmark policy, and fixture populations remain unchanged.
- No runtime parser or other behavior was implemented by this contract freeze.

## Evidence

- Oracle barrel SHA-256: `85943942619d9c2b7ed0855bdb58eb0360fec028e84efdd0f719589ba6ce4e01` (`src/index.ts`). Oracle type source SHA-256: `5b1e4b5793a04bd059600b6daf3b14b1b299958ee9f22633d9027c9d9956110c` (`src/types.ts`).
- Export snapshot contains 87 value names (11 directly declared exports plus 76 named re-exports) and 112 type names; the recorded extraction command was rerun against the oracle and matched both sets exactly. The manifest is checked for exact set equality, uniqueness, disposition, and mappings.
- Red test: `uv run --python 3.10 --with pytest==8.4.2 pytest tests/contracts/test_contract_inventory.py::test_public_api_manifest_classifies_the_exact_oracle_exports -q` failed because the API manifest was missing (`1 failed`).
- Green contract suite: `uv run --python 3.10 --with pytest==8.4.2 pytest tests/contracts -q` — 19 passed.
- JSON syntax check: `python3 -m json.tool` on all three new contract files — passed.

## Outcome

The Python API and recursive IR compatibility surface are machine-readable and linked from the SSOT index. Contract tests validate complete inventory coverage and internal schema consistency without opening the local oracle. Runtime implementation remains pending.

## Follow-ups

- PR #3 must complete required review and merge before Task 3 begins.
- Future changes to shared API names, mappings, fields, error/warning enums, or parity rules require coordinator review.
