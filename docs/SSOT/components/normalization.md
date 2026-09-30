# Shared IR projections and table normalization

`kordoc-core` projects source-neutral `kordoc-ir::IrBlock` values into Markdown,
page Markdown, structural chunks, and table policy. Format parsers return IR;
they do not import or duplicate this projection layer. The Python and
`ParseSuccess` integration is a separate coordinator-owned boundary. These
functions can be tested with generated IR, but that does not establish parity
for a successful PDF, HWP, Office, or other source-document parser. The ignored
TypeScript `kordoc/` tree is a read-only migration oracle, never a runtime or
package dependency. See the [P7 implementation plan](../migration/plans/2026-09-30-ir-projections-implementation-plan.md)
and [parity policy](../quality/parity.md).

## Markdown, pages, and chunks

`blocks_to_markdown` renders blocks in input order. It preserves heading,
paragraph, list, image, separator, and table distinctions; captions, safe
links, footnotes, spans, escaped GFM syntax, and known PUA substitutions are
handled in the shared renderer. Tables with merged or structured cells render
as escaped HTML rather than flattening their topology into GFM. Projection
does not execute Markdown or HTML, fetch URLs, or resolve local paths. A link
with an unsafe scheme is emitted as text, not an active link.

`blocks_to_pages` groups by observed `page_number`. A leading unnumbered block
uses the first observed page; a later unnumbered block follows the preceding
page. Revisited page numbers retain source order within that page. Every
number from the observed minimum through maximum has an entry, including an
empty Markdown entry for a gap. With no numbered block, the blocks-only
projection returns `None` rather than inventing page 1. Internal
`PageEvidence` may extend the bounded range for known empty source pages when
numbered blocks exist; it never reassigns block content. A caller-supplied
renderer may apply format-specific Markdown finalization to each nonempty
page; gap pages remain empty without a renderer call. The Python boundary's
optional callback contract is synchronous, receives immutable block mappings,
and propagates callback exceptions; the default renderer stays native.

`blocks_to_chunks` emits deterministic `c0001`-style IDs, inclusive top-level
block ranges, optional first-known page numbers, and breadcrumbs from heading
and list stacks. Empty-rendering blocks do not participate. Tables always
stand alone; their raw `cells` text matrix is included only when requested.
Default `section` granularity merges consecutive text blocks with the same
breadcrumb and renders each whole run through the shared Markdown renderer;
`block` granularity emits one nonempty block per chunk. It does not split by
tokens or add overlap. A consumer needing size-specific RAG chunks must apply
its own policy downstream; the projection never silently changes granularity.

## Table construction and classification

The table builder places addressed or implicit cells with checked dimensions
and spans, preserves collision text, and retains anchored empty trailing
columns only when requested. Span or grid overflow returns a typed error;
the Rust port does not clamp a span or truncate excess rows to make a table
fit. This is an intentional no-loss security divergence from the oracle's
clamping and `rows.slice(0, maxRows)` behavior. In-budget table topology and
Markdown remain the parity target.

Classification is opt-in (`classify_tables: Some(true)` at the parse boundary).
It adds classification summaries to table IR without changing block order,
cell text, or default Markdown. Representation selection is separate from
classification: GFM and HTML remain the default semantic outputs, while a
visual representation requires explicit smart-visual policy and an eligible
non-tabular classification. Crop geometry and `extract_tables` belong to
later scene/region integration, not this pure projection layer.

Legacy layout-table flattening is an internal HWP3/HWP5 parser policy only.
Generic IR and HWPX select `Never`. HWPX preserves nested table topology so
roundtrip source-map ordinals remain stable. No flattening marker is added to
the public wire schema.

## Shared Markdown table units

`kordoc_core::markdown_units` is the single Rust reader for downstream P8,
P10, and P12 adapters. Its public functions are `split_markdown_units`,
`parse_gfm_table`, `parse_html_table`, `html_cell_inner_to_lines`, and
`split_cell_by_top_level_tables`. It preserves escaped GFM pipes, `<br>` line
boundaries, nested HTML table order, and row/column spans. Consumers may adapt
the returned units but must not fork the parser. The reader is bounded and
does not evaluate scripts, expand entities into executable content, perform
I/O, or fetch remote resources.

## Limits and failure policy

All projection inputs are treated as untrusted. Checked arithmetic and
preflight budgets reject oversized work with stable typed errors; no text,
cells, pages, or chunks are silently discarded. The P7 limits are:

| Resource | Limit |
| --- | ---: |
| Table columns | 200 |
| Table cells | 2,000,000 |
| Default table rows | 10,000; a narrow-grid parser may explicitly allow more while remaining under the cell budget |
| Recursive block/table depth | 64 |
| Projected page span, including gaps | 100,000 |
| Emitted chunks | 100,000 |
| Raw HTML per Markdown table unit | 8 MiB |
| Projected Markdown/chunk UTF-8 bytes | 256 MiB |

Exceeding a budget returns `OUTPUT_TOO_LARGE` before returning a partial
result. Malformed structures may instead return the corresponding typed parse
or security error. These are core projection limits; the MCP response envelope
retains its separately contracted 200,000-character cap. The frozen wire
shapes and error inventory are in the [IR contract](../contracts/ir.md),
[Python API contract](../contracts/python-api.md), and
[error contract](../contracts/errors.md).


## Parse-result option finalization

A private core postprocessor applies the existing options after assembling
source-neutral IR and document/page Markdown. Ordering is `scriptTags=false`,
then `plain=true`, then `htmlTables=true`. Sup/sub stripping compacts owned UTF-8
in place and recursively handles block text, notes, spans, children, table
captions and cell blocks. Plain/HTML conversion changes only Markdown strings;
IR, images and page-count evidence remain unchanged.

Unicode script values, image/link markers, underline/bold and whitespace rules,
escaped pipe cells, allowed inline HTML, nested HTML and option ordering are
checked against five exact captured source-helper observations. Trusted,
compile-time regex patterns and single-pass scanners avoid repeated suffix
searches. Stages replace their owned input promptly; pretty-printing streams
borrowed characters and pipe cells instead of building input-sized character
or column vectors. Each output append checks the existing 256 MiB UTF-8 ceiling
before allocation; logical recursion remains bounded to 64 levels. Failures
return `OUTPUT_TOO_LARGE`, never clipped success. No input-controlled regex,
HTML execution, filesystem access or network fetch is introduced.

Table placeholder policy keeps flat cell IR text. Multi-column GFM and HTML
rendering use non-placeholder spans when present. The source's special
one-column path uses flat cell text, including guides; this quirk is preserved
rather than folded into the multi-column rule.
