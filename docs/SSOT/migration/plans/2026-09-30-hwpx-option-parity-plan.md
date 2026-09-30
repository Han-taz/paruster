# HWPX option parity follow-up

Status: Implemented and independently reviewed locally; protected hosted publication gates are pending.

This follow-up restores observable behavior of the existing frozen options; it introduces no API, IR field, error code or schema. The reviewed executable H4 candidate and its documented gaps form the baseline. The ignored oracle is read-only research and is never a build, test or runtime dependency.

## Ownership and execution

- Coordinator: `core/src/{parse,lib,markdown}.rs`, root/core Cargo manifests and lockfile, Python integration tests, SSOT/WIKI, protected publication. The new private postprocessor is called after Markdown/pages assembly.
- Luna A: `hancom/src/hwpx/{sections,tables}.rs` and their colocated tests. Restore `keepTrailingEmptyCols` and `includeFieldPlaceholders` through the existing `ParseOptions` forwarded into section lowering. No package/crypto or shared-contract edits.
- Luna B: `core/src/postprocess.rs` and colocated tests. Preserve the coordinator's in-place recursive `scriptTags=false` stripping; add bounded `plain` then `htmlTables` Markdown/page transforms. Both transforms leave recursive IR unchanged. Omissions and explicit false preserve current default behavior.

The scripted-tag transform removes only literal lowercase sup/sub tags from Markdown, pages, block text, footnote text, spans, children, table captions, caption blocks, cell text and cell blocks. It preserves text order, Unicode, styles and unrelated markup. `plain` preserves script values with caret/underscore notation, removes the oracle-defined image/link/underline/bold markers and applies the recorded whitespace policy. `htmlTables` converts pipe tables with exact escaping and pretty-prints existing HTML tables, including nested structure and allowed inline tags. Ordering is script-tag policy, plain, then HTML tables.

## Required evidence

1. Each owner records actual failing regressions before implementation and reruns them to GREEN. Authored XML/IR/Markdown cases cover omitted/false/true, nested content, field guides and filled values, anchor columns, Unicode, escaped pipes, and option combinations.
2. The 256 MiB Markdown ceiling, bounded logical depth, table/security/allocation limits and pre-allocation accounting stay intact. Growth fails as `OUTPUT_TOO_LARGE`; no clipped success or weakened fixture is allowed.
3. Root adds Python entry-point regressions and checks source and independently installed wheel behavior. Existing seven full-result captures and twelve original input hashes remain untouched; protected document parity accounting changes only after separately reviewed representative capture evidence.
4. Run focused and locked workspace tests, strict Clippy/fmt, Python/contracts/parity, lint/types/docs, audit/deny, artifact inspection and relevant fuzz/hosted gates. Independent review precedes branch push/PR integration and protected squash merge. Whole-parser completion remains pending for remaining diagnostic, checksum interoperability and restricted-corpus gaps.

Exact `regex = 1.13.1` is coordinator-pinned for trusted linear-time Unicode/category patterns. Unsupported lookbehind/backreferences must be implemented with bounded scanners preserving the observed behavior; input-controlled regex compilation is prohibited.


## Reviewed implementation evidence

The native suites pass 109 core unit cases and 118 Hancom unit cases, with 19
distinct HWPX integration cases (the integration binary also repeats the 116
units). Five captured helper observations compare exactly. The previous
installed wheel failed all 21 authored Python entry-point regressions; a freshly
built candidate passes all 317 Python/contract/parity tests. Script stripping
also has recorded native RED-to-GREEN evidence.

Independent review drove regression fixes for nested CLICK_HERE overlap,
floating-table field boundaries, flat cell-text retention and HTML placeholder
visibility. Performance/resource fixes replace suffix rescans, character
vectors and pipe-cell collections with linear scans and borrowed streaming.
The pipe-heavy test feeds 2,000,000 delimiters to a 128-byte output cap and
requires a typed failure before cell materialization. Existing budgets, oracle
answers, H0 input hashes and protected scoring remain unchanged.
