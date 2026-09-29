# Python API contract

`contracts/public-api.json` classifies every named value and type exported by the reviewed `src/index.ts` barrel. `contracts/oracle-public-exports.json` contains the exact value/type export snapshot and source evidence. These snapshots freeze compatibility scope only; they do not claim that the mapped functions or models are implemented.

## Primary parsing API

```text
parse(input, options=None) -> ParseResult        # raises KordocError on failure
try_parse(input, options=None) -> TryParseResult # serializable success/failure
input := path-like | bytes | bytearray | memoryview | BinaryIO
```

`parse` is the foundation API shell and returns an unsupported-format failure until parsers land. Successful results expose immutable Python models for the recursive IR. `try_parse` returns a serializable success/failure form for batch and protocol consumers. Typed failures use `KordocError` subclasses and stable codes. CPU-bound native work releases the GIL.

The foundation accepts only `options=None`. A non-`None` value raises `NotImplementedError` until parser option behavior is implemented; options are never silently discarded. `try_parse` converts stable document and input-normalization failures, including `FILE_NOT_FOUND` and `OUTPUT_TOO_LARGE`, into its flat result model. Invalid Python input types, text-returning streams, and unsupported options are programmer errors and may raise `TypeError` or `NotImplementedError`.

The TypeScript value exports retain a single explicit disposition and Python mapping in `public-api.json`. `VERSION`, `detectFormat`, and `parse` are `foundation`; format-specific parsers map to snake_case Python names and remain `planned`. Python-only `try_parse`, `TryParseResult`, and the typed exception hierarchy live in the manifest's separate `python_only_entries` inventory. Other library behavior also remains `planned`; classification as `planned` is not evidence of runtime availability.

All 112 TypeScript type exports have an explicit category and mapping in `ir-schema.json`: 111 serializable input/model/result/enum/union types and the `OcrProvider` callable adapter. The schema records exact fields, requiredness, enums, recursive references, and serialization adapters. `Uint8Array`/`ArrayBuffer` payloads map to Python `bytes`, with JSON arrays of byte integers at serialization boundaries. TypeScript `onProgress`, `OcrProvider`, `ParseOptions.ocr`, and `ExtractRegionOptions.filter` map to Python callables. `filePath` remains internal and is not a public Python option.

The Node CLI and Node package entry machinery are classified as `removed-node-surface` in `public-api.json`. That disposition applies only to those product surfaces; it does not remove the remaining document library behaviors, which stay planned for the Python port.

## Python input normalization

The Python boundary accepts path-like values, `bytes`, `bytearray`, `memoryview`, and binary file-like objects. Path inputs are opened and normalized before entering the Rust core. File-like inputs are read as bytes without taking ownership of the caller's stream. Normalization must not silently reinterpret text strings as document bytes. `options` is optional and defaults to the documented library behavior.

Memory lengths are checked before copying. Paths are `stat`-checked and then read through a bounded reader to close the size-change race; file-like reads are bounded to the configured input cap plus one byte and begin at the caller's current position. The caller's stream remains open. Only immutable, already-bounded bytes cross the native boundary.

`TryParseResult` is frozen and slotted. `to_dict()` emits camelCase protocol keys, omits unavailable optional values instead of emitting `null`, and preserves the success/failure discriminator. The base `KordocError` and one explicit subclass per code in `contracts/errors.json` expose the stable code and safe message. Native detector failures use exactly two `ValueError` arguments `(code, message)` before the Python facade maps them; unrelated `ValueError` shapes are not intercepted.

## Compatibility changes

The compatibility manifest describes what is frozen, what is only planned, and the narrow removed Node surfaces. Shared API names, error codes, and wire schemas require coordinator approval before change. No API classification asserts that a future parser, generator, renderer, or transformation has been implemented.
