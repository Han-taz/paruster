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
