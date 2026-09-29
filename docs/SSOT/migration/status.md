# Migration status

This page is the live capability ledger. A capability is complete only after its implementation, contract/parity evidence, and required quality gates have merged. Contract inventory alone never counts as implementation.

## Verified foundation

- Rust workspace and Python 3.10+ abi3 packaging scaffold
- recursive foundation IR and the 13 stable error codes
- bounded content detection for HWP3, ZIP-based HWPX/Office formats, CFB-based HWP/XLS, PDF, HWPML, PNG, JPEG, and WebP
- checked single-disk ZIP/ZIP64 preflight with input, entry-count, uncompressed-size, and local-extent bounds
- foundation dispatch that reports empty, oversized, security-rejected, and unsupported inputs without claiming parser completion

The Python facade, deterministic detector golden harness, CI matrix, fuzzing, coverage, wheels, and release automation remain in progress until their own tasks and gates pass.

## Pending product parity

Full oracle parity is pending. No real document parser, OCR pipeline, transformation, comparison, redaction, form operation, patching/generation flow, renderer, or MCP handler is complete merely because its contract has been frozen. Each stays pending until representative and adversarial oracle/golden fixtures pass in Rust/Python.

The 17 MCP tools remain planned and must preserve their frozen names, schemas, descriptions, envelopes, limits, security policy, and stdio discipline when implemented.

## Evidence policy

Detector-only synthetic goldens prove deterministic foundation plumbing, not document-output parity. The parity policy in [`../quality/parity.md`](../quality/parity.md) governs allowed normalization; first-difference evidence and fixture provenance are required before any capability moves from pending to verified.
