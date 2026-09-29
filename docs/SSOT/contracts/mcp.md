# MCP contract

`contracts/mcp-tools.json` freezes the ordered public inventory of 17 MCP tools. `contracts/mcp-protocol.json` captures each tool's input JSON Schema, output content envelope, output and file behavior, shared resource limits, transport rules, and security constraints. The source digests identify the migration-oracle files reviewed for this snapshot; the extraction and review commands are recorded with the evidence.

All captured schemas, envelopes, security limits, and stdio behavior are compatibility requirements for the Python MCP adapter. The frozen inventory reports every tool's runtime status as `planned`; listing a name for the future server does not mean its handler or underlying operation is implemented.

## Resource and transport requirements

- Document input is capped at 500 MiB; metadata-only extraction is capped at 50 MiB.
- A 200,000-character response cap applies only to `parse_document`, `parse_chunks`, and `redact_document`. Rendering returns no more than eight selected page images; table extraction returns no more than eight crop images.
- The server uses stdio JSON-RPC framing. Standard output is reserved for protocol messages; diagnostics and console output go to standard error. Importing the server module must not seize stdio, and server startup is idempotent.
- Input paths are canonicalized before extension and configured-root checks. Output paths use extension allowlists, reject a symlink at the leaf, resolve the nearest existing ancestor, and are rechecked before a no-follow file open. Root confinement applies when an access root is configured.
- Error envelopes mark failures with `isError: true`. Error text uses the shared classifier and sanitizer behavior; validation errors may retain the offending path as specified by source behavior.

Changes to a tool name, schema field, enum, default, bound, success content type, error envelope, output rule, limit, path rule, or transport behavior require coordinator review and an update to these machine-readable contracts.
