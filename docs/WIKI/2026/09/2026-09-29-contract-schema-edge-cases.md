---
id: 2026-09-29-contract-schema-edge-cases
date: 2026-09-29
status: recorded
component: shared-contracts
issue: null
pr: https://github.com/Han-taz/paruster/pull/3
commit: 851e849
related_ssot:
  - docs/SSOT/contracts/ir.md
  - docs/SSOT/contracts/python-api.md
  - docs/SSOT/contracts/compatibility-manifest.md
related_decisions: []
---

# Contract schema edge-case corrections

## Context

This append-only follow-up to [`2026-09-29-contract-freeze-review.md`](2026-09-29-contract-freeze-review.md) records four schema defects found during further review of [PR #3](https://github.com/Han-taz/paruster/pull/3). It adds no runtime implementation claim.

## Work performed

- Added the inherited optional `pages`, `reflow`, and `reflowMode` fields to `ExtractRegionOptions` classification and schema.
- Replaced the ambiguous `oneOf` for `MarkdownToHwpxOptions.images` values with a single JSON byte-array shape annotated with its TypeScript input types (`Uint8Array` and `ArrayBuffer`).
- Constrained the `FaceClass` template-literal branch to strings beginning `font:` while preserving the three literal alternatives.
- Made `OcrProvider` reject all JSON instances and captured its exact TypeScript argument names/types, MIME enum, Promise return, and Python callable translation. `ParseOptions.ocr` still accepts its serializable `true` and `"force"` values.
- Added actual Draft 2020-12 validation examples for these cases using an ephemeral pinned `jsonschema==4.25.1` dependency.

## Evidence

- Red: `uv run --python 3.10 --with pytest==8.4.2 --with jsonschema==4.25.1 pytest tests/contracts/test_contract_inventory.py::test_ir_schema_validates_inherited_options_adapters_and_face_class tests/contracts/test_contract_inventory.py::test_ocr_callable_is_not_json_and_preserves_callable_signature -q` — 2 failed as expected: inherited `ExtractRegionOptions` fields were rejected, and `OcrProvider` incorrectly accepted JSON `null`.
- Green: `uv run --python 3.10 --with pytest==8.4.2 --with jsonschema==4.25.1 pytest tests/contracts -q` — 21 passed.
- The focused tests call `Draft202012Validator.check_schema` and validate representative region options, image byte arrays, `FaceClass` literals/template strings, callable rejection, and valid boolean/`"force"` OCR options.
- `git diff --check` — passed. `git ls-files -- kordoc` — empty. No tests or build manifests depend on the ignored oracle checkout.
- Oracle evidence is unchanged: `src/index.ts` SHA-256 `85943942619d9c2b7ed0855bdb58eb0360fec028e84efdd0f719589ba6ce4e01`; `src/types.ts` SHA-256 `5b1e4b5793a04bd059600b6daf3b14b1b299958ee9f22633d9027c9d9956110c`.

## Outcome

The four reviewed edge cases now have schema coverage and executable validation examples. Normative behavior remains documented in the SSOT pages; this entry is historical evidence only. No parser, OCR engine, rendering, or other runtime feature was implemented.

## Follow-ups

- PR #3 remains subject to normal review, CI, and merge requirements.
- Continue treating callable adapters as Python API contracts rather than JSON-serializable values.
