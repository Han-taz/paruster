---
id: 2026-09-29-contract-freeze-review
date: 2026-09-29
status: recorded
component: shared-contracts
issue: null
pr: https://github.com/Han-taz/paruster/pull/3
commit: 5d97a59
related_ssot:
  - docs/SSOT/contracts/ir.md
  - docs/SSOT/contracts/python-api.md
  - docs/SSOT/contracts/compatibility-manifest.md
related_decisions: []
---

# Contract freeze review corrections

## Context

This append-only follow-up corrects the contract evidence from [`2026-09-29-contract-freeze.md`](2026-09-29-contract-freeze.md) after review of [PR #3](https://github.com/Han-taz/paruster/pull/3). The PR remains a draft; this entry records local follow-up work on the same contract branch and does not imply merge or runtime implementation.

## Work performed

- Classified all 112 captured public type exports individually. The machine-readable schema distinguishes 111 serializable input/model/result/enum/union types from the `OcrProvider` callable adapter and includes recursively reachable support definitions.
- Filled the serializable schemas for the public input and result surface, including chunking, form fill/validation, seal placement, redaction, table extraction/crops and visual policy, rendering/scenes/assets/options, source maps/sessions, generation, and profiles.
- Marked `ParseOptions.onProgress` as a planned Python callback adapter. `filePath` is the sole internal option. OCR callback adaptation and the JSON-only boolean/`"force"` values are documented separately.
- Standardized schema translation annotations on `x-python-translation` and made `ir-schema.json` a Draft 2020-12 schema whose root selects declared serializable public types.
- Strengthened offline contract tests for exact export/type set equality, unique classifications, field/required/enum parity, recursive references and support reachability, adapters, callback exclusions, and root-schema structure.
- Updated the normative IR, Python API, and compatibility pages; no parser or other runtime behavior was implemented.

## Evidence

- Initial red: `uv run --python 3.10 --with pytest==8.4.2 pytest tests/contracts/test_contract_inventory.py::test_ir_schema_is_recursive_complete_and_internally_consistent -q` — failed because `type_classifications` was absent. The follow-up red after adding type inventory also exposed an invalid `OcrProvider` JSON reference in the OCR union; it was corrected to a Python callable adapter outside the JSON union.
- Green: `uv run --python 3.10 --with pytest==8.4.2 pytest tests/contracts -q` — 19 passed.
- Schema validation: `uv run --python 3.10 --with jsonschema==4.25.1 python -c 'import json; from jsonschema import Draft202012Validator; s=json.load(open("contracts/ir-schema.json", encoding="utf-8")); Draft202012Validator.check_schema(s); v=Draft202012Validator(s); assert v.is_valid({"success":False,"fileType":"unknown","error":"unsupported","code":"UNSUPPORTED_FORMAT"}); assert v.is_valid({"pages":[1,2],"ocr":"force"}); assert not v.is_valid({"unknown-contract-object":True}); assert not v.is_valid(3); assert not v.is_valid({"filePath":"/tmp/a"}); assert not v.is_valid({"onProgress":"callback"}); print("Draft 2020-12 meta-schema valid; root accepts declared result/options and rejects unknown object, number, internal field, and serialized callable")'` — passed using ephemeral pinned `jsonschema==4.25.1`.
- `git diff --check` — passed. `git ls-files -- kordoc` returned no tracked oracle files. Contract tests do not require or read the ignored oracle checkout.
- Source evidence remains `src/index.ts` SHA-256 `85943942619d9c2b7ed0855bdb58eb0360fec028e84efdd0f719589ba6ce4e01` and `src/types.ts` SHA-256 `5b1e4b5793a04bd059600b6daf3b14b1b299958ee9f22633d9027c9d9956110c`.

## Outcome

The reviewed schema now covers the captured type surface and is usable as a validating JSON Schema root. Current normative details are in the linked SSOT pages; this record only captures the review correction and evidence. Parser behavior remains planned.

## Follow-ups

- PR #3 still requires its normal review, CI, and merge workflow before dependent runtime tasks proceed.
- The contract remains a specification, not evidence that the planned Python callbacks or document operations have runtime implementations.
