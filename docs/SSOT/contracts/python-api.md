# Python API contract

`contracts/public-api.json` classifies every named value and type exported by the reviewed `src/index.ts` barrel. `contracts/oracle-public-exports.json` contains the exact value/type export snapshot and source evidence. These snapshots freeze compatibility scope only; they do not claim that the mapped functions or models are implemented.

## Primary parsing API

```text
parse(input, options=None) -> ParseResult        # raises KordocError on failure
try_parse(input, options=None) -> TryParseResult # serializable success/failure
input := path-like | bytes | bytearray | memoryview | BinaryIO
```

`parse` is the foundation API shell and returns an unsupported-format failure until parsers land. Successful results expose immutable Python models for the recursive IR. `try_parse` returns a serializable success/failure form for batch and protocol consumers. Typed failures use `KordocError` subclasses and stable codes. CPU-bound native work releases the GIL.

The TypeScript value exports retain a single explicit disposition and Python mapping in `public-api.json`. `parse` is `foundation`; its format-specific siblings map to snake_case Python names and remain `planned`. Other library behavior also remains `planned`; classification as `planned` is not evidence of runtime availability.

Type exports each map to a Python model, enum, union, or callable adaptation. The IR wire types are specified in `ir.md` and `ir-schema.json`. `Uint8Array`/`ArrayBuffer` payloads map to Python `bytes`, with JSON arrays of byte integers at serialization boundaries. TypeScript `onProgress` and `OcrProvider` functions map to Python callables. `filePath` remains internal and is not a public Python option.

The Node CLI and Node package entry machinery are classified as `removed-node-surface` in `public-api.json`. That disposition applies only to those product surfaces; it does not remove the remaining document library behaviors, which stay planned for the Python port.

## Python input normalization

The Python boundary accepts path-like values, `bytes`, `bytearray`, `memoryview`, and binary file-like objects. Path inputs are opened and normalized before entering the Rust core. File-like inputs are read as bytes without taking ownership of the caller's stream. Normalization must not silently reinterpret text strings as document bytes. `options` is optional and defaults to the documented library behavior.

## Compatibility changes

The compatibility manifest describes what is frozen, what is only planned, and the narrow removed Node surfaces. Shared API names, error codes, and wire schemas require coordinator approval before change. No API classification asserts that a future parser, generator, renderer, or transformation has been implemented.
