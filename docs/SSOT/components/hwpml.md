# HWPML component

Status: H0 authored fixture capture merged in PR #28. A private bounded Rust
text/metadata lowerer merged in PR #30; public parsing and table/recovery
qualification remain pending.

The H0 checkpoint contains eight deterministic CC0 XML fixtures and 13 complete
public results from the pinned read-only migration oracle. The
[fixture README](../../../tests/golden/document/hwpml/README.md) records exact
input/capture/generator hashes, oracle source identities and one-time runtime
provenance. Four offline contract tests regenerate the small inputs, pin full
capture/options inventory and stream-hash an oversized in-memory recipe.
Neither Node.js nor the oracle is needed by ordinary tests or delivery.

The captures cover metadata, BOM/namespaces, heading mapping, section pages,
ignored structural text, inline footnote text, nonbreaking spaces, nested and
merged tables, trailing-column options, empty bodies and XML error observations.
Recoverable undefined references produce literal text plus `MALFORMED_XML`;
unclosed XML produces sanitized `PARSE_ERROR`. The escaped DTD input is only a
literal-text control. A separate actual external reference at a harmless
nonexistent URI returns a literal reference and warning; this does not prove
that no file access was attempted. The generated 50 MiB+1 input returns
`DECOMPRESSION_BOMB` and is represented by a recipe rather than a large file.

The [H0 plan](../migration/plans/2026-09-30-hwpml-fixture-plan.md) keeps these
observations separate from native implementation. PR [#28](https://github.com/Han-taz/paruster/pull/28)
merged the fixture checkpoint as `5a077eb78704b8b60e03963a22297978f9854d63`.

## Private text lowerer candidate

The [text slice plan](../migration/plans/2026-09-30-hwpml-private-text-plan.md)
adds an unregistered `kordoc-hancom::hwpml` module. It reuses the bounded HWPX
XML reader through a narrow crate-private tree seam, but performs separate
HWPML traversal. The inclusive 50 MiB outer input cap runs before XML parsing
or optional same-length `&nbsp;` normalization. UTF-8, XML depth/node/text/tree
limits and a shared 256 MiB lowering-allocation budget remain hard boundaries;
DTD/custom entities and malformed syntax return typed failures. Failure returns
no partial document.

Valid text-only lowering reads direct document summary fields, outline
paragraph shapes, ordered paragraph `CHAR` text, inline footnote text and
one-based section page ordinals. Selected pages retain their original section
numbers. It emits source-neutral blocks and outline without Markdown or
invented page-count evidence. Entire section-level headers/footers are ignored;
paragraph-level picture/shape/autonumber text is omitted. Any structural table
visited in selected paragraph or section content returns `UNSUPPORTED_FORMAT`
until HWPML-specific coordinates, nesting and captions are implemented.

The three normal/empty authored inputs used by Rust tests are byte-identical
crate-local copies of H0 fixtures. Their original captures and all protected
parity accounting remain unchanged. This strict private slice does not claim
oracle parity for recoverable malformed XML or DTD cases, whose legacy behavior
differs from the security policy. No public registry, Python API, HWP3/HWP5
completion or MCP capability is added.

PR [#30](https://github.com/Han-taz/paruster/pull/30) merged the private text
slice as `5a8e27c4d67a3cfcff9f31bb5e9dcc304fda6f34` after required checks
and all six additional installed-worker targets passed on final head `fa8c461`.
See the [merge record](../../WIKI/2026/09/2026-09-30-hwpml-private-text-merge.md);
public HWPML registration and full parity remain pending.


## Private unique-anchor table checkpoint

The [nested-table plan](../migration/plans/2026-09-30-hwpml-nested-table-plan.md)
is approved for a bounded private slice against three unchanged H0 observations
of the authored 949-byte nested-table input. Default/false require 2×2 geometry;
true retains the anchored trailing column for 2×3. Qualification includes full
serialized blocks and ordered nested cell content, not flat text alone.
The source-specific Hancom adapter uses the merged IR-only shared builder,
one document-wide logical-cell budget and the existing lowering allocation
budget. Root verifies fixture-copy bytes and captured-IR Markdown projection.
Native lowering passes exact full-block comparison for all three captures and
independent review; latest-main artifact/hosted evidence remains pending.

Collision/text-span fallback, loose source dimension coercion, captions,
depth-9 flattening, malformed XML recovery, public registration and broad
HWPML parity remain outside this slice. Unsupported structured attachments
and captions fail closed rather than silently losing blocks. Selected nested
tables under inline note/header/footer wrappers and tables without direct cells
also reject; the source skips those tables. Shared
security limits and protected scoring remain unchanged.

## Collision observations and deliberate divergence

The [capture-only checkpoint](../migration/plans/2026-09-30-hwpml-collision-capture-plan.md)
adds three separately authored CC0 XML inputs and five complete frozen source
results. A nonblank coordinate collision can silently drop nested structure;
a repeated-text decoy can receive the nested blocks at the wrong coordinate.
The source succeeds in both cases. Private Rust deliberately rejects ambiguous
attachments with `UNSUPPORTED_FORMAT`, so these observations establish a
known divergence, not matching parity. The blank collision control preserves
owner blocks. Exact offline inventory/hash/regeneration and semantic checks
pass independent review. Existing H0 fixtures, protected scoring and native
behavior remain unchanged. Any later recovery policy requires a separate
coordinator-reviewed implementation change.

PR [#33](https://github.com/Han-taz/paruster/pull/33) merged private unique-anchor
tables as `e0fde7db6ed97914ed78483fd2b5f0a58a5d99a3` after required checks,
CodeQL and all six installed worker targets passed.
