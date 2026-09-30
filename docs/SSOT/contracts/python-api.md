# Python API contract

`contracts/public-api.json` classifies every named value and type exported by the reviewed `src/index.ts` barrel. `contracts/oracle-public-exports.json` contains the exact value/type export snapshot and source evidence. A `foundation` disposition records an implemented, locally tested mapping; it does not claim real-document parser parity or hosted CI completion.

## Primary parsing API

```text
parse(input, options=None) -> ParseResult        # raises KordocError on failure
try_parse(input, options=None) -> TryParseResult # serializable success/failure
input := path-like | bytes | bytearray | memoryview | BinaryIO
```

`parse` dispatches the executable local HWPX candidate; unwired formats return
an unsupported-format failure. The successful boundary exposes
`result.document`, a frozen/slotted `Document` projection, while
`TryParseResult.to_dict()` preserves the flat frozen wire envelope. Typed
failures use `KordocError` subclasses and stable codes. CPU-bound native work
releases the GIL. Candidate implementation is separate from protected capability
and parity verification in the [migration ledger](../migration/status.md).

Serializable parse options are accepted as a snake_case mapping, strictly validated, converted to frozen camelCase names, and passed as an owned Rust DTO before native work releases the GIL. Omission stays distinct from explicit `false`, finite fractional page values are preserved until page projection, and `file_path` and unknown keys are rejected. To keep conversion bounded before JSON ownership transfer, page lists accept at most 100,000 entries and page-range/password strings at most 65,536 characters; the native boundary independently enforces the same limits. Callable OCR/progress adapters raise `NotImplementedError` until their owning task lands. `try_parse` converts stable document and input-normalization failures, including `FILE_NOT_FOUND` and `OUTPUT_TOO_LARGE`, into its flat result model.

The TypeScript value exports retain a single explicit disposition and Python mapping in `public-api.json`. `VERSION`, `detectFormat`, `detectOle2Format`, `detectZipFormat`, the four legacy signature predicates, `parse`, `blocksToMarkdown`, `blocksToPages`, `blocksToChunks`, and `classifyTableTree` are `foundation`; format-specific parsers and the other table exports remain `planned`. Python-only `try_parse`, `TryParseResult`, `Document`, and the typed exception hierarchy live in the manifest's separate `python_only_entries` inventory.

All 112 TypeScript type exports have an explicit category and mapping in `ir-schema.json`: 111 serializable input/model/result/enum/union types and the `OcrProvider` callable adapter. The schema records exact fields, requiredness, enums, recursive references, and serialization adapters. `Uint8Array`/`ArrayBuffer` payloads map to Python `bytes`, with JSON arrays of byte integers at serialization boundaries. TypeScript `onProgress`, `OcrProvider`, `ParseOptions.ocr`, and `ExtractRegionOptions.filter` map to Python callables. `filePath` remains internal and is not a public Python option.

The Node CLI and Node package entry machinery are classified as `removed-node-surface` in `public-api.json`. That disposition applies only to those product surfaces; it does not remove the remaining document library behaviors, which stay planned for the Python port.

## Python input normalization

The Python boundary accepts path-like values, `bytes`, `bytearray`, `memoryview`, and binary file-like objects. Path inputs are opened and normalized before entering the Rust core. File-like inputs are read as bytes without taking ownership of the caller's stream. Normalization must not silently reinterpret text strings as document bytes. `options` is optional and defaults to the documented library behavior.

Memory lengths are checked before copying. Paths are `stat`-checked and then read through a bounded reader to close the size-change race; file-like reads are bounded to the configured input cap plus one byte and begin at the caller's current position. The caller's stream remains open. Only immutable, already-bounded bytes cross the native boundary.

`TryParseResult` and `Document` are frozen and slotted. Image byte arrays become Python `bytes` while ordinary integer arrays remain sequences. `to_dict()` converts image bytes back to JSON arrays, emits camelCase protocol keys, omits unavailable values instead of emitting `null`, and preserves the result discriminator. The base `KordocError` and one explicit subclass per stable code expose safe messages.

## HWPX candidate API

`parse_hwpx(input, options=None) -> TryParseResult` accepts the same normalized
inputs/options as `parse`, but strictly rejects another detected format rather
than parsing it. It raises typed errors and exposes `.document` on success.
Core owns Markdown/page projections and keeps full source `page_count` and
metadata page count when selecting pages. Images become immutable Python
`bytes`, serialized back to integer arrays only by `to_dict()`.

`validate_hwpx(input, password=None) -> ValidateResult` returns frozen/slotted
`ValidateResult(ok, issues, entry_count)` and ordered
`ValidateIssue(message, path=None)` objects. Its exact wire fields are `ok`,
`issues`, and `entryCount`; issue `path` is omitted when unavailable. Actual
file counts are nonnegative integers excluding directory records. This does
not alter the existing JSON-number schema. Passwords are strings of at most
65,536 characters. Missing/wrong passwords remain typed `ENCRYPTED`; no
progress callback or selective metadata-only decryption is introduced.

Native parse results serialize directly through the capped writer without an
unbounded intermediate `serde_json::Value` copy. Native output-limit failures
become serializable typed `try_parse` failures; `parse` raises the corresponding
exception. Local installed-wheel tests cover these boundaries independently
of source import. Parser capability remains unpromoted while the lowering
allocation/full-result/hosted gates remain open.

## Source-neutral projections

The implemented public values are `kordoc.blocks_to_markdown(blocks) -> str`, `kordoc.blocks_to_pages(blocks, render=None) -> tuple[PageMarkdown, ...] | None`, `kordoc.blocks_to_chunks(blocks, options=None) -> tuple[DocChunk, ...]`, and `kordoc.tables.classify_table_tree(blocks) -> tuple[immutable IR mappings, ...]`. `ChunkOptions`, `DocChunk`, and `PageMarkdown` are frozen, slotted Python models with exact camelCase `from_dict`/`to_dict` roundtrips; present-null optional fields are rejected. `DocChunk`'s existing schema uses JSON `number` for page, range, and table dimensions, while values emitted by the Rust source are non-negative `u32` integers. The schema is unchanged. `classify_table`, `collect_table_blocks`, `choose_table_representation`, and other table helper mappings remain planned Python exports.

Projection input is a sequence of recursive IR mappings. Common concrete Python `dict`/`list` inputs are validated without a full deep copy; generic mappings/sequences are materialized after validation. The Python and native boundaries independently bound input nodes/bytes and raw JSON nesting to 256 levels, allowing the core's separate 64-level logical block/table recursion budget to decide semantic depth. Native projection output is serialized through a capped writer before Python JSON decoding. Markdown/chunk output is capped at 256 MiB UTF-8, with at most 100,000 emitted chunks and 100,000 pages across a min-to-max page span. Table policy enforces 200 columns and 2,000,000 cells; excess content returns `OUTPUT_TOO_LARGE` rather than being silently clamped or truncated. These limits and structural semantics are detailed in [normalization](../components/normalization.md).

`blocks_to_pages` returns `None` when no block has a numeric page number. Leading unnumbered blocks join the first real page; later ones inherit the preceding page, and gaps yield empty page entries. Its optional `render` callback runs synchronously while Python is attached to the GIL, receives a tuple of deeply immutable block mappings (including image bytes as `bytes`), must return `str`, and is invoked only for pages with blocks. The original callback exception propagates unchanged. The default renderer runs native projection with the GIL detached. The table-tree operation is opt-in and does not alter default Markdown; normal parse classification requires `classify_tables: True` in parse options. None of these pure projections establishes that a real document parser can produce a successful result.

## Compatibility changes

The compatibility manifest describes what is frozen, what is only planned, and the narrow removed Node surfaces. Shared API names, error codes, and wire schemas require coordinator approval before change. No API classification asserts that a future parser, generator, renderer, or transformation has been implemented.
## HWPX candidate option checkpoint

For generic `parse`, `try_parse`, and format-specific `parse_hwpx`, explicit
`images=False` removes the result image collection and recursive block
`imageData`, retaining placeholders and the existing Markdown/page projections.
Omission and explicit `True` preserve the default output. HWPX ignores the
PDF-only `tables` switch. Full option parity remains pending for `plain`,
`htmlTables`, `scriptTags`, `keepTrailingEmptyCols`, and `includeFieldPlaceholders`;
candidate registration does not mark those behaviors implemented.
