# Engineering WIKI

WIKI is an append-only engineering record. It preserves investigations, implementation sessions, incidents, experiments, and their evidence. It is historical and non-normative; current truth belongs in `docs/SSOT/`.

## Layout

Entries use `YYYY/MM/YYYY-MM-DD-<topic>.md`. Each entry begins with the metadata in [`TEMPLATE.md`](TEMPLATE.md).

## Append-only rules

1. Do not silently rewrite or delete a merged entry.
2. Add a dated correction or follow-up and link the earlier entry.
3. Link the issue, PR, commit, affected component, evidence, and relevant SSOT/ADR.
4. Never place current operational instructions or API contracts only in WIKI.
5. CI generates and validates the searchable index once tooling is introduced.

## Entries

- [`2026/09/2026-09-29-repository-bootstrap.md`](2026/09/2026-09-29-repository-bootstrap.md)
- [`2026/09/2026-09-29-foundation-planning.md`](2026/09/2026-09-29-foundation-planning.md)
- [`2026/09/2026-09-29-contract-freeze.md`](2026/09/2026-09-29-contract-freeze.md)
- [`2026/09/2026-09-29-contract-freeze-review.md`](2026/09/2026-09-29-contract-freeze-review.md)
- [`2026/09/2026-09-29-contract-schema-edge-cases.md`](2026/09/2026-09-29-contract-schema-edge-cases.md)
- [`2026/09/2026-09-30-foundation-integer-contract.md`](2026/09/2026-09-30-foundation-integer-contract.md)
- [`2026/09/2026-09-30-bounded-detection.md`](2026/09/2026-09-30-bounded-detection.md)
- [`2026/09/2026-09-30-rust-python-foundation.md`](2026/09/2026-09-30-rust-python-foundation.md)
- [`2026/09/2026-09-30-foundation-execution.md`](2026/09/2026-09-30-foundation-execution.md)
- [`2026/09/2026-09-30-product-port-planning.md`](2026/09/2026-09-30-product-port-planning.md)
- [`2026/09/2026-09-30-parser-seam.md`](2026/09/2026-09-30-parser-seam.md)
- [`2026/09/2026-09-30-parser-seam-merge.md`](2026/09/2026-09-30-parser-seam-merge.md)
- [`2026/09/2026-09-30-ir-projections.md`](2026/09/2026-09-30-ir-projections.md)
- [`2026/09/2026-09-30-ir-projections-merge.md`](2026/09/2026-09-30-ir-projections-merge.md)
- [`2026/09/2026-09-30-parser-wave-planning.md`](2026/09/2026-09-30-parser-wave-planning.md)
- [`2026/09/2026-09-30-parser-wave-scaffold.md`](2026/09/2026-09-30-parser-wave-scaffold.md)
- [`2026/09/2026-09-30-parser-wave-scaffold-merge.md`](2026/09/2026-09-30-parser-wave-scaffold-merge.md)
- [`2026/09/2026-09-30-hwpx-h0-merge.md`](2026/09/2026-09-30-hwpx-h0-merge.md)
- [`2026/09/2026-09-30-hwpx-h1a-merge.md`](2026/09/2026-09-30-hwpx-h1a-merge.md)
- [`2026/09/2026-09-30-hwpx-h2a.md`](2026/09/2026-09-30-hwpx-h2a.md)
- [`2026/09/2026-09-30-hwpx-h1b-local.md`](2026/09/2026-09-30-hwpx-h1b-local.md)
- [`2026/09/2026-09-30-hwpx-h4-local.md`](2026/09/2026-09-30-hwpx-h4-local.md)
- [`2026/09/2026-09-30-hwpx-h4-publication.md`](2026/09/2026-09-30-hwpx-h4-publication.md)
- [`2026/09/2026-09-30-port-resumed.md`](2026/09/2026-09-30-port-resumed.md)

- [`2026/09/2026-09-30-pdf-substrate.md`](2026/09/2026-09-30-pdf-substrate.md)
- [`2026/09/2026-09-30-pdf-v8-decision.md`](2026/09/2026-09-30-pdf-v8-decision.md)
- [`2026/09/2026-09-30-hwpx-codeql-start-key.md`](2026/09/2026-09-30-hwpx-codeql-start-key.md)
- [`2026/09/2026-09-30-hwpx-hosted-candidate.md`](2026/09/2026-09-30-hwpx-hosted-candidate.md)

- [`2026/09/2026-09-30-hwpx-option-restoration.md`](2026/09/2026-09-30-hwpx-option-restoration.md)
- [`2026/09/2026-09-30-pdf-v8-runtime.md`](2026/09/2026-09-30-pdf-v8-runtime.md)

- [`2026/09/2026-09-30-hwpx-option-wheel-gates.md`](2026/09/2026-09-30-hwpx-option-wheel-gates.md)
- [`2026/09/2026-09-30-pdf-v8-worker.md`](2026/09/2026-09-30-pdf-v8-worker.md)

- [`2026/09/2026-09-30-hwpx-option-merge.md`](2026/09/2026-09-30-hwpx-option-merge.md)

- [`2026/09/2026-09-30-pdf-v8-worker-gates.md`](2026/09/2026-09-30-pdf-v8-worker-gates.md)
- [`2026/09/2026-09-30-pdf-v8-worker-merge.md`](2026/09/2026-09-30-pdf-v8-worker-merge.md)
- [`2026/09/2026-09-30-hwpx-warning-parity.md`](2026/09/2026-09-30-hwpx-warning-parity.md)

- [`2026/09/2026-09-30-hwpml-fixture-capture.md`](2026/09/2026-09-30-hwpml-fixture-capture.md)
- [`2026/09/2026-09-30-hwpml-private-text.md`](2026/09/2026-09-30-hwpml-private-text.md)

- [`2026/09/2026-09-30-hwpx-warning-merge.md`](2026/09/2026-09-30-hwpx-warning-merge.md)
- [`2026/09/2026-09-30-pdf-worker-wheel-feasibility.md`](2026/09/2026-09-30-pdf-worker-wheel-feasibility.md)
- [`2026/09/2026-09-30-pdf-worker-windows-checkout.md`](2026/09/2026-09-30-pdf-worker-windows-checkout.md)

- [`2026/09/2026-09-30-pdf-worker-windows-utf8.md`](2026/09/2026-09-30-pdf-worker-windows-utf8.md)

- [`2026/09/2026-09-30-pdf-worker-platform-newline.md`](2026/09/2026-09-30-pdf-worker-platform-newline.md)

- [`2026/09/2026-09-30-pdf-worker-wheel-merge.md`](2026/09/2026-09-30-pdf-worker-wheel-merge.md)
- [`2026/09/2026-09-30-pdf-v8-resources.md`](2026/09/2026-09-30-pdf-v8-resources.md)

- [`2026/09/2026-09-30-pdf-v8-resources-merge.md`](2026/09/2026-09-30-pdf-v8-resources-merge.md)

- [`2026/09/2026-09-30-pdf-v8-text-document.md`](2026/09/2026-09-30-pdf-v8-text-document.md)

- [`2026/09/2026-09-30-hwpml-private-text-merge.md`](2026/09/2026-09-30-hwpml-private-text-merge.md)

- [`2026/09/2026-09-30-pdf-v8-text-document-merge.md`](2026/09/2026-09-30-pdf-v8-text-document-merge.md)

- [`2026/09/2026-09-30-pdf-rust-metadata.md`](2026/09/2026-09-30-pdf-rust-metadata.md)

- [`2026/09/2026-09-30-shared-table-builder.md`](2026/09/2026-09-30-shared-table-builder.md)



- [`2026/09/2026-09-30-shared-table-builder-merge.md`](2026/09/2026-09-30-shared-table-builder-merge.md)
