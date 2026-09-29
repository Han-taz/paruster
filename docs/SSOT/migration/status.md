# Migration status

This page is the live capability ledger. A capability is complete only after its implementation, contract/parity evidence, and required quality gates have merged. Contract inventory alone never counts as implementation. Local verification is recorded separately from hosted CI results and repository branch-protection enforcement.

## Locally verified foundation

- Rust workspace and Python 3.10+ abi3 packaging scaffold
- recursive foundation IR and the 13 stable error codes
- bounded content detection for HWP3, ZIP-based HWPX/Office formats, CFB-based HWP/XLS, PDF, HWPML, PNG, JPEG, and WebP
- checked single-disk ZIP/ZIP64 preflight with input, entry-count, uncompressed-size, and local-extent bounds
- foundation dispatch that reports empty, oversized, security-rejected, and unsupported inputs without claiming parser completion

The Python facade and deterministic detector golden harness are verified foundation capabilities. The checked-in quality workflows and dependency/security policy are present and their local policy checks have passed. Locally, `kordoc-ir` and `kordoc-core` have 88.63% line coverage, and two 30-second fuzz campaigns completed without crashes. These are local evidence only; synthetic detector goldens and bounded fuzz runs do not demonstrate document-output parity.

Hosted workflows and required-check enforcement remain pending until PR #5 has run. This status does not claim that GitHub-hosted checks have passed or that branch protection requires them.

## Pending product parity

Full oracle parity is pending. All document parsers, OCR pipelines, transformations, comparisons, redaction, form operations, patching/generation flows, renderers, and MCP handlers remain pending. None is complete merely because its contract has been frozen; each stays pending until representative and adversarial oracle/golden fixtures pass in Rust/Python.

All 17 MCP tools remain pending and must preserve their frozen names, schemas, descriptions, envelopes, limits, security policy, and stdio discipline when implemented.

## Evidence policy

Detector-only synthetic goldens prove deterministic foundation plumbing, not document-output parity. The parity policy in [`../quality/parity.md`](../quality/parity.md) governs allowed normalization; first-difference evidence and fixture provenance are required before any capability moves from pending to verified.
