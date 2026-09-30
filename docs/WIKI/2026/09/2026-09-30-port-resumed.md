---
id: 2026-09-30-port-resumed
date: 2026-09-30
status: candidate-review
component: hwpx
issue: null
pr: null
commit: 17a158a2de14b01b500d60c818ecf70683db65a4
related_ssot: [docs/SSOT/components/hwpx.md, docs/SSOT/migration/status.md]
related_decisions: []
---

# Port resumed through authenticated GitHub CLI

## Context

The previous H2a record preserves a connected-tool approval failure. A fresh
CLI check confirms active GitHub authentication, repository ADMIN permission,
remote access and successful feature-branch publication. This corrects the
current operational assumption; earlier historical records remain unchanged.
The PDF fixture lint fix was published to PR #20 without changing generated
fixture hashes or relaxing its gates.

## Work performed

Recovered the isolated H2a, H1b, H2b and H3 commits and the uncommitted H4
candidate into permanent feature worktrees. The original temporary checkouts
remain intact. No oracle files are implementation or runtime dependencies.

Fresh review identified allocation amplification in the earlier standalone
H2a lowerer: inherited note prefixes could be copied for every note without an
aggregate bound. The H3 candidate already includes a pre-allocation lowering
meter and charged leaf-text traversal. The private PR therefore includes that
join; it does not publish the earlier unbounded intermediate slice alone.

## Evidence

Independent SOL review of committed private Hancom candidate `17a158a` reports
no remaining Critical or Important issue. Focused locked tests pass 103 library
tests and 122 integration-binary executions, including repeated unit cases;
there are 19 distinct integration cases. Reduced-limit tests cover exact
allocation boundaries, nested note text, repeated large note-prefix copies,
and cumulative metadata fallback. Resource exhaustion remains a hard
`OUTPUT_TOO_LARGE` failure rather than a partial parse.

Python/contract tests on the private checkpoint pass 223 cases. Formatting,
strict workspace Clippy, warning-free Rust documentation, Python lint,
formatting, typing, and documentation links/indexes pass locally. Hosted
verification and protected squash merge remain required. H4 public wiring is
a separate stacked checkpoint, not a completed capability in this record.

## Outcome

Porting work and feature-branch publication are resumed. HWPX capability and
the protected document-parser oracle-success numerator remain pending.
