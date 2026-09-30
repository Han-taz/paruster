---
id: 2026-09-30-hwpx-option-restoration
date: 2026-09-30
status: recorded
component: hwpx-options
issue: null
pr: null
commit: null
related_ssot:
  - docs/SSOT/components/hwpx.md
  - docs/SSOT/components/normalization.md
  - docs/SSOT/contracts/python-api.md
  - docs/SSOT/migration/plans/2026-09-30-hwpx-option-parity-plan.md
related_decisions: []
---

# Frozen HWPX option restoration

## Context

The executable H4 candidate merged in PR [#22](https://github.com/Han-taz/paruster/pull/22)
as `e10a2df2dd57d1b69f55afeb0a07295e0c29db7f`. Its five option gaps were
explicitly recorded in the earlier [candidate evidence](2026-09-30-hwpx-hosted-candidate.md).
The user requested continued porting with parallel workers. A SOL reviewer and
two Luna owners split core postprocessing and private Hancom lowering; the
coordinator owned Python integration, shared projections, dependencies and
publication. API/IR/error/MCP schemas were not changed.

## Work performed

`scriptTags=false`, `plain=true` and `htmlTables=true` now finalize document/page
Markdown in source order; script stripping also recurses through IR text.
`keepTrailingEmptyCols` uses real anchors after charging the complete logical
grid. `includeFieldPlaceholders` preserves flat IR text and marks matching
CLICK_HERE guide spans. Parameter metadata no longer leaks into body text.

The coordinator captured five authored Markdown helper observations by erasing
TypeScript syntax in memory with Node's `stripTypeScriptTypes` and importing
pure helpers from data URLs. The ignored oracle was not modified or packaged;
only CC0-authored inputs/outputs and source hashes are committed. Full parser
answers, all twelve H0 hashes and the protected parity numerator remain intact.

## Evidence

- All 21 new Python regressions genuinely failed against the previous isolated
  H4 wheel. Fresh candidate Python/contract/parity execution passes 317 tests.
- Core passes 109 unit tests plus integration groups; Hancom passes 118 units,
  and its integration binary passes 137 executions (118 repeated units and
  19 distinct integration cases). Strict scoped Clippy and formatting pass.
- Nested outer `AB`/inner `B` fields first failed because both spans were marked;
  the fix marks only the inner guide. Floating-table positive and negative
  cases compare all pre/post-flush field text. Cell IR retains guides while
  multi-column GFM/HTML hide marked spans.
- Review exposed repeated suffix searches, a full `Vec<char>` and an unbounded
  pipe-cell vector. The implementation now scans linearly, promptly drops
  prior stage buffers, streams characters/cells and checks output before
  append. A 2,000,000-pipe regression fails at its 128-byte cap without a cell
  collection. Production limits and scoring are unchanged.
- Five captured helper cases pass exactly, including Unicode, escaped pipes,
  HTML nesting and option ordering. Trusted regex is pinned at `1.13.1`;
  dependency policy passes without exceptions.

## Outcome

This is a focused option-restoration checkpoint. Hosted gates, independently
installed wheel evidence and the final PR are recorded in a later publication
entry after execution. Whole-parser parity remains pending for representative
and restricted corpora, malformed-section diagnostics and ODF checksum
interoperability.

## Follow-ups

Preserve the source's one-column flat-text guide visibility quirk. A malformed
Command length that splits an astral UTF-16 surrogate is retained unmarked
because the public IR requires valid Unicode scalars. Neither difference is
hidden by normalization. PDF.js/V8 private runtime meanwhile merged in PR
[#23](https://github.com/Han-taz/paruster/pull/23) as
`df7d4c9167e534c7ee44d5609f6f5babfcd2993d` after required hosted gates passed;
production PDF registration still requires its separate pending gates.
