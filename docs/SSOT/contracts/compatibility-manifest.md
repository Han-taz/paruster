# Compatibility manifest

The machine-readable manifests are the reviewable inventory of the TypeScript-to-Python boundary:

- `contracts/oracle-public-exports.json` records every named value and type export from `src/index.ts`, the SHA-256 of that exact file, and repeatable extraction and review commands.
- `contracts/public-api.json` gives every captured value and type export one disposition and one target mapping. It also records excluded input surfaces such as `filePath` and the explicitly removed Node CLI/package entry machinery.
- `contracts/ir-schema.json` fixes the public JSON wire fields and nested IR/result shapes. `contracts/errors.json` and `contracts/mcp-tools.json`/`contracts/mcp-protocol.json` freeze shared error and MCP boundaries.

The value and type export sets in the Python API manifest must equal the oracle snapshot exactly; duplicate source names are invalid. Mapped Python names must be unique. Every entry names its disposition, mapping, and reason. Dispositions are `foundation`, `planned`, `internal`, or `removed-node-surface`. `foundation` means only that the foundation API shell is present; `planned` does not assert runtime behavior. At this stage `parse` is the sole foundation function and returns unsupported until parser implementations land. All format parsers and remaining library behavior are planned. `filePath` is internal, and only the Node CLI and Node package-entry machinery are removed.

The oracle directory is a local, read-only research source. Captured JSON is committed evidence and tests validate it without opening the oracle. No new package, runtime, test, or CI dependency may point into `kordoc/`.

Updating an inventory or changing a disposition is a shared-contract change and requires coordinator approval. The snapshot's source digest and extraction evidence must be updated whenever the reviewed source changes.
