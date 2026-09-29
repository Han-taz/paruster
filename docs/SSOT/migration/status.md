# Migration status

This page is the live capability ledger. A capability is complete only after its implementation, contract/parity evidence, and required quality gates have merged. Contract inventory alone never counts as implementation. Local verification is recorded separately from hosted CI results and repository branch-protection enforcement.

## Locally verified foundation

- Rust workspace and Python 3.10+ abi3 packaging scaffold
- recursive foundation IR and the 13 stable error codes
- bounded content detection for HWP3, ZIP-based HWPX/Office formats, CFB-based HWP/XLS, PDF, HWPML, PNG, JPEG, and WebP
- checked single-disk ZIP/ZIP64 preflight with input, entry-count, uncompressed-size, and local-extent bounds
- foundation dispatch that reports empty, oversized, security-rejected, and unsupported inputs without claiming parser completion
- internal acyclic `ParsedDocument`/options seam, private injectable registry, parser panic containment, and successful-result assembly tests
- immutable Python `Document` projection, bounded serializable option validation, and six legacy detection/refinement compatibility helpers with pinned runtime-oracle evidence
- provenance-checked full-result harness with source-smoke versus oracle-capture accounting

The parser seam has only injected/synthetic success evidence; the production registry remains empty and the document harness reports zero successful oracle captures. The earlier 88.63% line coverage and two 30-second fuzz campaigns without crashes describe the merged foundation baseline; this branch requires fresh hosted evidence. None of these foundation checks demonstrate document-output parity.

PR #5 implementation head `a9df485` passed the hosted foundation gates: [CI](https://github.com/Han-taz/paruster/actions/runs/36600464167), [Security](https://github.com/Han-taz/paruster/actions/runs/36600464228), [Native wheels](https://github.com/Han-taz/paruster/actions/runs/36600464274), and [Bounded fuzzing](https://github.com/Han-taz/paruster/actions/runs/36600464235). PR #5 documentation head `ad86e43` also passed all four hosted gates: [CI](https://github.com/Han-taz/paruster/actions/runs/36601581589), [Security](https://github.com/Han-taz/paruster/actions/runs/36601581506), [Native wheels](https://github.com/Han-taz/paruster/actions/runs/36601581485), and [Bounded fuzzing](https://github.com/Han-taz/paruster/actions/runs/36601581555). PR #4 remediation head `31212275` passed all four hosted checks: [CI](https://github.com/Han-taz/paruster/actions/runs/36603823278), [Security](https://github.com/Han-taz/paruster/actions/runs/36603823358), [Native wheels](https://github.com/Han-taz/paruster/actions/runs/36603823436), and [Bounded fuzzing](https://github.com/Han-taz/paruster/actions/runs/36603823386). The Security run includes a successful main-target Dependency review, confirming the pytest 9.0.3 remediation. Active protect-main ruleset `24182744` strictly requires `ci-gate`, `security-gate`, `wheels-gate`, `fuzz-gate`, and CodeQL, limits PR merges to squash-only, and states that later heads must pass the required checks. Product parity remains pending as detailed below.

## Pending product parity

Full oracle parity is pending. All document parsers, OCR pipelines, transformations, comparisons, redaction, form operations, patching/generation flows, renderers, and MCP handlers remain pending. None is complete merely because its contract has been frozen; each stays pending until representative and adversarial oracle/golden fixtures pass in Rust/Python.

All 17 MCP tools remain pending and must preserve their frozen names, schemas, descriptions, envelopes, limits, security policy, and stdio discipline when implemented.

## Evidence policy

Detector-only synthetic goldens prove deterministic foundation plumbing, not document-output parity. The parity policy in [`../quality/parity.md`](../quality/parity.md) governs allowed normalization; first-difference evidence and fixture provenance are required before any capability moves from pending to verified.
