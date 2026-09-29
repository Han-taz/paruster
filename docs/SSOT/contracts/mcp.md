# MCP contract

`contracts/mcp-tools.json` freezes the ordered public inventory of 17 MCP tools. `contracts/mcp-protocol.json` captures each tool's input JSON Schema, output content envelope, output and file behavior, shared resource limits, transport rules, and security constraints. The source digests identify the migration-oracle files reviewed for this snapshot; the extraction and review commands are recorded with the evidence.

All captured schemas, envelopes, security limits, and stdio behavior are compatibility requirements for the Python MCP adapter. The frozen inventory reports every tool's runtime status as `planned`; listing a name for the future server does not mean its handler or underlying operation is implemented.

## Resource and transport requirements

- Document input is capped at 500 MiB; metadata-only extraction is capped at 50 MiB.
- `detect_format` first reads a 512-byte header. It reads the full file, subject to the 500 MiB limit, only when the header indicates ZIP or OLE data that needs container inspection; other detected formats do not trigger that full-file read.
- A 200,000-character response cap applies only to `parse_document`, `parse_chunks`, and `redact_document`. Rendering returns no more than eight selected page images; table extraction returns no more than eight crop images.
- The server uses stdio JSON-RPC framing. Standard output is reserved for protocol messages; diagnostics and console output go to standard error. Importing the server module must not seize stdio, and server startup is idempotent.
- Input paths are canonicalized before extension and configured-root checks. Output paths use extension allowlists, reject a symlink at the leaf, resolve the nearest existing ancestor, and are rechecked before a no-follow file open. Root confinement applies when an access root is configured.
- Error envelopes mark failures with `isError: true`. Error text uses the shared classifier and sanitizer behavior; validation errors may retain the offending path as specified by source behavior.

`contracts/mcp-protocol.json` records the per-tool input extensions, output extension maps, conditional `output_path`/`output_dir` requirements, generated manifests, and content-dependent response behavior. For example, `fill_form` selects output extensions from `output_format` and returns a preview when no output path is supplied; `redact_document` requires a path unless `dry_run` is true and chooses the output extension from detected format; `render_document` requires a file path for SVG, HTML, and PDF, while raster output can be returned as image content. `patch_document` documents the same-extension expectation but its handler only validates that the output is either `.hwpx` or `.hwp`; it does not enforce a match to the input extension. Generation assets resolve beneath `image_dir`, remain under the configured `KORDOC_ROOT` when set, reject symlink escapes, and are opened without following a final symlink. The reviewed source digest for this behavior is included in the protocol evidence.

The generation schema also freezes preset-sensitive defaults: an omitted cover date resolves to today's date at render time; `gaejosik` and `ministry` enable covers and tables of contents by default; the `press` preset suppresses both; page numbers default on for `gaejosik`, `ministry`, `report`, `plan`, and `bangchim`; and role-based font overrides apply to all four roles for the report-family presets, while other presets apply the body role only. Other preset defaults and supported field descriptions are captured in `generation_behavior` and the `generate_document` schema.

Changes to a tool name, schema field, enum, default, bound, success content type, error envelope, output rule, limit, path rule, or transport behavior require coordinator review and an update to these machine-readable contracts.
