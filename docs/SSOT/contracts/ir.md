# Intermediate representation contract

`contracts/ir-schema.json` is the normative, machine-readable JSON wire contract for `ParseResult`, recursive blocks and tables, page data, metadata, quality data, warnings, and stable error codes. The schema records the reviewed TypeScript source digests. It is an inert compatibility snapshot: consumers and tests do not read or import the local migration oracle.

## Wire representation

- JSON property names preserve the existing camelCase spelling (`pageNumber`, `captionBlocks`, `pageQuality`, `qualitySummary`). `IRBlock.type` is the discriminant; it is not renamed to `kind`.
- A field in `required` must be present. Other declared fields are optional and are omitted when unavailable; omission is distinct from an explicit `null`. Objects reject undeclared keys.
- `IRBlock.children`, table-cell `blocks`, and `IRTable.captionBlocks` recursively contain `IRBlock` values. `cells` is a two-dimensional array of `IRCell`; recursive tables and nested blocks retain source order.
- `ParseResult` is a discriminated union. Success requires `success: true`, `fileType`, `markdown`, and `blocks`. Failure requires `success: false`, `fileType`, and `error`; `code`, `pageCount`, and `isImageBased` remain optional. The shared result base fields `fileType`, `pageCount`, and `isImageBased` are present on both variants where supplied.
- Warning `code` uses the exact 17-value `WarningCode` enum. Failure `code` uses the exact stable error code inventory in `contracts/errors.json`. Warnings remain separate from failures.
- `ImageData.data`, `ExtractedImage.data`, and binary patch/fill outputs originate as TypeScript byte arrays/buffers. Python models expose binary payloads as `bytes`; JSON serialization represents those bytes as arrays of integers from 0 through 255. The wire schema retains the byte-array representation.
- Recursive JSON roundtrip must preserve block children, table cell blocks, caption blocks, spans, images, page projections, metadata, outline, quality summaries, warnings, and error values without changing key casing or omitting required fields.

`IRSpan` and table-classification details are included as supporting definitions because public `IRBlock`/`IRTable` values can contain them, even though not every supporting type is separately exported from the package barrel.

`ParseOptions` includes the serializable option fields in the schema. The oracle-only `filePath` option is internal and excluded from Python's public options; `onProgress` and the OCR callback are adapted to Python callables. Callback functions are not JSON values. Option omission uses the documented default behavior and is not equivalent to `false`.

Any change to a public field, key spelling, required/optional status, enum, binary translation, recursive edge, result discriminator, warning, or error value requires coordinator review and an update to the machine-readable schema and this page.
