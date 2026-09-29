# Python API contract

`contracts/public-api.json` classifies every named value and type exported by the reviewed `src/index.ts` barrel. `contracts/oracle-public-exports.json` contains the exact value/type export snapshot and source evidence. These snapshots freeze compatibility scope only; they do not claim that the mapped functions or models are implemented.

## Primary parsing API

```text
parse(input, options=None) -> ParseResult        # raises KordocError on failure
try_parse(input, options=None) -> TryParseResult # serializable success/failure
input := path-like | bytes | bytearray | memoryview | BinaryIO
```

`parse` is the foundation API shell and returns an unsupported-format failure until parsers land. The successful boundary exposes `result.document`, a frozen/slotted `Document` projection, while `TryParseResult.to_dict()` preserves the flat frozen wire envelope. Typed failures use `KordocError` subclasses and stable codes. CPU-bound native work releases the GIL.

Serializable parse options are accepted as a snake_case mapping, strictly validated, converted to frozen camelCase names, and passed as an owned Rust DTO before native work releases the GIL. Omission stays distinct from explicit `false`, finite fractional page values are preserved until page projection, and `file_path` and unknown keys are rejected. To keep conversion bounded before JSON ownership transfer, page lists accept at most 100,000 entries and page-range/password strings at most 65,536 characters; the native boundary independently enforces the same limits. Callable OCR/progress adapters raise `NotImplementedError` until their owning task lands. `try_parse` converts stable document and input-normalization failures, including `FILE_NOT_FOUND` and `OUTPUT_TOO_LARGE`, into its flat result model.

The TypeScript value exports retain a single explicit disposition and Python mapping in `public-api.json`. `VERSION`, `detectFormat`, `detectOle2Format`, `detectZipFormat`, the four legacy signature predicates, and `parse` are `foundation`; format-specific parsers remain `planned`. Python-only `try_parse`, `TryParseResult`, `Document`, and the typed exception hierarchy live in the manifest's separate `python_only_entries` inventory.

All 112 TypeScript type exports have an explicit category and mapping in `ir-schema.json`: 111 serializable input/model/result/enum/union types and the `OcrProvider` callable adapter. The schema records exact fields, requiredness, enums, recursive references, and serialization adapters. `Uint8Array`/`ArrayBuffer` payloads map to Python `bytes`, with JSON arrays of byte integers at serialization boundaries. TypeScript `onProgress`, `OcrProvider`, `ParseOptions.ocr`, and `ExtractRegionOptions.filter` map to Python callables. `filePath` remains internal and is not a public Python option.

The Node CLI and Node package entry machinery are classified as `removed-node-surface` in `public-api.json`. That disposition applies only to those product surfaces; it does not remove the remaining document library behaviors, which stay planned for the Python port.

## Python input normalization

The Python boundary accepts path-like values, `bytes`, `bytearray`, `memoryview`, and binary file-like objects. Path inputs are opened and normalized before entering the Rust core. File-like inputs are read as bytes without taking ownership of the caller's stream. Normalization must not silently reinterpret text strings as document bytes. `options` is optional and defaults to the documented library behavior.

Memory lengths are checked before copying. Paths are `stat`-checked and then read through a bounded reader to close the size-change race; file-like reads are bounded to the configured input cap plus one byte and begin at the caller's current position. The caller's stream remains open. Only immutable, already-bounded bytes cross the native boundary.

`TryParseResult` and `Document` are frozen and slotted. Image byte arrays become Python `bytes` while ordinary integer arrays remain sequences. `to_dict()` converts image bytes back to JSON arrays, emits camelCase protocol keys, omits unavailable values instead of emitting `null`, and preserves the result discriminator. The base `KordocError` and one explicit subclass per stable code expose safe messages.

## Compatibility changes

The compatibility manifest describes what is frozen, what is only planned, and the narrow removed Node surfaces. Shared API names, error codes, and wire schemas require coordinator approval before change. No API classification asserts that a future parser, generator, renderer, or transformation has been implemented.
