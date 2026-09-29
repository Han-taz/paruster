# Compatibility manifest

The machine-readable manifests are the reviewable inventory of the TypeScript-to-Python boundary:

- `contracts/oracle-public-exports.json` records every named value and type export from `src/index.ts`, the SHA-256 of that exact file, and repeatable extraction and review commands.
- `contracts/public-api.json` gives every captured value and type export one disposition and one target mapping. It also records excluded input surfaces such as `filePath` and the explicitly removed Node CLI/package entry machinery.
- `contracts/ir-schema.json` classifies all 112 exported type names and fixes the serializable public API input/result/model fields, recursive IR, enums, requiredness, and non-JSON adapter mappings. `contracts/errors.json` and `contracts/mcp-tools.json`/`contracts/mcp-protocol.json` freeze shared error and MCP boundaries.

The value and type export sets in the Python API manifest must equal the oracle snapshot exactly; duplicate source names are invalid. Python-owned additions are recorded separately in `python_only_entries` so they cannot corrupt oracle export-set equality. Mapped Python names must be unique. Every entry names its disposition, mapping, and reason. Dispositions are `foundation`, `planned`, `internal`, or `removed-node-surface`. `foundation` means only that the named foundation behavior is present; `planned` does not assert runtime behavior. At this stage version metadata, bounded `detect_format`, and the `parse` shell are foundation mappings. Python additionally owns `native_version`, `try_parse`, its result model, and typed exceptions. `parse` still returns unsupported until parser implementations land. All format-specific parsers and remaining library behavior are planned. `filePath` is internal, and only the Node CLI and Node package-entry machinery are removed.

The oracle directory is a local, read-only research source. Captured JSON is committed evidence and tests validate it without opening the oracle. No new package, runtime, test, or CI dependency may point into `kordoc/`.

The Python `detect_format` translation is intentionally more precise than the oracle's coarse synchronous `detectFormat`: it performs the oracle's separate container refinements in one bounded call and returns the refined HWPX/XLSX/DOCX/PPTX type. A generic ZIP or OLE container remains `unknown`. This approved translation changes detection precision, not the frozen `FileType` protocol strings, and does not imply that the detected parser is implemented.

Updating an inventory or changing a disposition is a shared-contract change and requires coordinator approval. The snapshot's source digest and extraction evidence must be updated whenever the reviewed source changes.
