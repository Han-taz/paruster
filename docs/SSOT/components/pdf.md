# PDF runtime and historical substrate

## Current runtime direction

The September 30 user decision requires PDF.js embedded in V8 for semantic extraction. The active [runtime plan](../migration/plans/2026-09-30-pdf-v8-runtime-plan.md) pins `v8 = 152.2.0` and upstream `pdfjs-dist = 4.10.38`, matching the oracle version without copying oracle assets. The private `pdfjs-v8` feature is a feasibility probe; default builds expose no PDF parser. Rust owns binary input, execution/output limits, typed results, and downstream IR/layout policy. Production PDF registration requires supervised native-worker failure containment, full-result parity, resource factories, and six-target installed-wheel evidence. The existing borrowed-source reader remains private historical research and is not developed as a competing semantic backend.

## Private runtime evidence

The `pdfjs-v8` feature now executes the checked-in main/worker modules in a fresh isolate and returns exact page count and text for the authored one-page Helvetica fixture. The executing V8 engine reports `15.2.124.1-rusty`; the bridge checks PDF.js `4.10.38` at runtime. The scoped suite has 8 passing executions: extraction, prefixed corrupt input, initialized-context host-I/O absence, inclusive serialized-output boundary, actual watchdog termination, custom allocator unit boundary, actual JS ArrayBuffer over-cap rejection/release, and V8 version.

Private feasibility limits are 32 MiB binary input, 200 pages, 4 MiB serialized UTF-8 output, 192 MiB V8 old-generation heap, 128 MiB V8-owned ArrayBuffer backing stores and a 10-second watchdog. The custom allocator charges input before copying and rejects further buffers before allocation. The actual over-cap JS probe produces `RangeError`, records rejection, maps to existing `OUTPUT_TOO_LARGE`, and releases all charged bytes when its isolate drops. Heap/native allocations outside that allocator and fatal OOM are not contained by this in-process spike; these values are not a production acceptance/RSS guarantee.

Default builds and the public registry still expose no PDF parser. The narrow ReadableStream, URL and structuredClone shims prove this fixture only; complete transfer/Web API semantics, text decoding, CMap/standard-font factories, metadata/operator evidence, complex documents and native process isolation remain pending. The checked-in upstream source/resources total 5,106,488 bytes across 188 provenance-pinned files, with all applicable licenses included and a default-feature offline integrity test. Licensed resources are ready for future factories; their presence is not evidence those factories are implemented.

Dependency policy passes without changes. `cargo audit --file Cargo.lock` reports the upstream V8 build-time `paste 1.0.15` unmaintained warning [RUSTSEC-2024-0436](https://rustsec.org/advisories/RUSTSEC-2024-0436.html); it reports no listed vulnerability and no ignore is added. Production release must revisit upstream replacement and keep advisory checks intact.

## Historical private reader

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
the superseded P2a design. They are not active implementation tasks under the V8 decision. Filtered xref streams therefore fail closed today.

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
