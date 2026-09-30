---
id: 2026-09-30-hwpx-warning-parity
date: 2026-09-30
status: locally-verified
component: hwpx
issue: null
pr: null
commit: null
related_ssot: [docs/SSOT/components/hwpx.md, docs/SSOT/migration/status.md]
related_decisions: []
---

# Restore the captured HWPX unfinished-section warning

## Context and ownership

The unchanged H0 malformed-section capture retained a single message mismatch
after the seven ordinary/encrypted full results matched. Luna A owns private
XML/section implementation and native regressions; SOL performs read-only
safety review; the coordinator owns complete Python result comparisons,
documentation, verification and publication. No public schema changes.

## RED evidence

Before implementation, the newly included eighth frozen result fails exactly
at `/warnings/0/message`: expected the original Korean section-2 wrapper with
`unclosed xml tag(s): hs:sec, hp:p`; actual is
`section Contents/section1.xml could not be parsed`. Three public paths
(`try_parse`, `parse`, `parse_hwpx`) fail against the installed preceding
extension, while the seven previous results and all twelve input hashes pass:
**3 failed, 8 passed**. The focused native section-isolation regression also
fails on the exact same message while code/page and neighboring sections hold.
No frozen input, answer or scoring file is rewritten.

## Review findings

Early review identified that re-scanning every generic malformed fault could
misclassify an invalid entity followed by unfinished tags as an EOF error.
The primary parser must mark unfinished-tag EOF distinctly. Source ranges
must follow verified event boundaries rather than searching for a `<` that
could occur inside an attribute. Complete diagnostics must be charged before
allocation and valid XML must avoid extra name copies/scans.

## Local evidence

Final independent SOL review is CLEAN. The Hancom crate passes 125 library
unit tests and 144 integration-binary executions (125 repeated private unit
cases plus 19 distinct integration cases). The native full workspace, strict
all-target/all-feature Clippy, formatting and warning-free rustdoc pass on the
final implementation. New cases exercise primary EOF classification, invalid
entities/mismatched ends, borrowed attribute/name ranges, the 200-level limit,
exact UTF-8 diagnostic budget N/N-minus-one and an 8 KiB qualified prefix under
a reduced allocation budget. Valid XML adds no second scan or name copy.

Source-extension Python/contract/parity tests pass **320**. A freshly built
release `cp310-abi3-macosx_11_0_arm64` wheel installed into an isolated CPython
3.10.19 environment also passes all **320**, including all eight unchanged
complete answers and the twelve original H0 hashes. Wheel plus sdist pass the
artifact policy without oracle files. Ruff check/format and mypy pass.

Early review also removed redundant secondary attribute normalization: the
primary reader already validates attributes before it can reach unfinished-tag
EOF, so the diagnostic pass retains borrowed checks without another decoded
value allocation. Each diagnostic segment is charged before storage growth.

Final 30-second bounded campaigns complete without crashes on the frozen
implementation: XML **114,285** executions and package **200,923** executions.
Mutable fuzz corpus growth stays outside the repository; original seeds are
unchanged. Actionlint, zizmor, cargo-deny and cargo-audit pass without new
exceptions. The existing V8 build-time `paste` unmaintained advisory remains
visible and is not ignored. Documentation links/indexes and staged diff checks
pass. Protected hosted CI/security/six-wheel/fuzz checks remain pending.

## Remaining qualification

This slice closes only the known unfinished-tag message discrepancy. Other
malformed XML diagnostics, ODF compressed/decompressed checksum interoperability,
representative/restricted-corpus parity and full parser qualification remain
pending. Protected document-parser success accounting is unchanged.
