# PDF substrate

This page defines the private PDF object-access substrate established by P2a
Task 0. It is not a public parser contract and does not claim PDF, Python, or
MCP capability. Higher-level page, glyph, layout, table, asset, and quality
semantics remain pending.

## Substrate decision

The pinned `lopdf = 0.45.0` candidate is rejected for runtime object loading.
Its document reader eagerly materializes ordinary objects and object streams,
copies stream payloads before caller-controlled charging, applies only
per-stream decompression limits, and runs predictor expansion outside that
limit. Its relevant parser phases are not public hooks. These properties
cannot satisfy pre-allocation, cumulative-byte, object-count, recursion, or
500 MiB no-second-copy gates. The dependency remains pinned only as an audited
candidate until a later change removes it or a separately reviewed use is
approved; the Task 0 runtime path does not call it.

`kordoc-pdf` instead owns a small borrowed-source reader. It locates the final
`startxref` in a bounded tail window, walks classic or xref-stream revision
chains, charges every distinct xref ID including free entries before map
insertion, and parses an indirect object only when requested. All reference
walks share active-ID and depth state. Trailer dictionaries, strings, comments,
and stream boundaries are lexed so data bytes cannot become references or hide
an encryption marker.

## Current supported surface

- classic xref tables, xref streams, incremental `/Prev` revisions, generation
  checks, and type-2 compressed-object lookup;
- borrowed ordinary-object values and charged owned compressed-object values;
- unfiltered and ASCIIHex stream payloads under 32 MiB per-stream and 256 MiB
  cumulative decoded ceilings;
- typed fail-closed rejection for unsupported or multi-filter streams,
  encryption, malformed lengths/offsets/generations, cycles, and budget
  violations;
- the shared `PdfBudget` counters frozen in the implementation plan, including
  source, object/dereference/depth, decoded bytes, operators, glyphs, text, IR,
  and pixels.

Flate/predictor decoding, indirect stream lengths, fonts/CMaps, page trees,
content operators, and all semantic IR lowering are deliberately deferred to
the remaining P2a tasks. Filtered xref streams therefore fail closed today.

## Security evidence

Deterministic CC0 fixtures and the reproducible per-probe error/RSS table live
in [`tests/golden/document/pdf/README.md`](../../../tests/golden/document/pdf/README.md).
The adversarial suite covers the inclusive source and decoded-byte boundaries,
the million-ID boundary for normal and free classic/xref-stream entries,
object and form recursion, compressed-container cycles and index overflow,
allocation-free oversized token/filter/index/reference inputs, encryption
trailer lexical evasions, stream-payload false references, and the 500 MiB
sparse probe. These are substrate checks only; they do not advance the
real-document parity numerator.
