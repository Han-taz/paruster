# Private PDF.js embedded resource factories

Status: coordinator-approved next private slice; depends on PR #27's worker
wheel foundation. No public parser, IR or worker-wire contract change.

## Ownership and boundary

Luna B owns `crates/kordoc-pdf/src/v8_runtime/{mod,engine,host,resources}.rs`,
feature tests and new authored PDF probe inputs/generator with provenance. SOL
reviews callbacks, budgets and real resource usage. The coordinator owns SSOT,
WIKI, license manifest and worker-wheel notice inventory updates/publication.
No worker edits the existing worker supervisor or public/shared contracts.

## Implementation and validation

First add authored one-page PDFs that require a built-in CID CMap and an
unembedded standard font, then record real resource-callback RED evidence.
Extraction success alone is insufficient because PDF.js can fall back.

Embed only existing SHA-pinned CMap/font bytes using an exact compile-time
allowlist: 168 `.bcmap` entries and 14 standard-font binary entries. Implement
custom PDF.js `CMapReaderFactory` and `StandardFontDataFactory` via a narrow
Rust callback returning V8 allocator-backed `Uint8Array`. Require exact kind,
ASCII basename <=128 bytes and exact inventory; reject paths, URLs and unknown
entries. Set `useWorkerFetch:false` and `useSystemFonts:false`, retaining no
filesystem/network/Node/fetch/document-script host APIs. Resource URL options are null and factories return only embedded bytes;
the existing module identifier is a trusted `pdfjs://` pseudo-URL, never I/O.

Charge before every returned allocation: inclusive 192 KiB per item, 512
requests and 8 MiB cumulative served bytes per document, in addition to the
existing 128 MiB allocator cap, heap/input/output limits and 10s watchdog.
Counters record successful/denied requests. A recorded callback rejection or
budget exhaustion must be returned as a typed failure even when PDF.js would
otherwise fall back. No captured status string may hide a denied resource.

GREEN requires exact document text and >0 CMap/font callbacks from the intended
fixtures, installed subprocess protocol parity, inclusive/overflow boundary
regressions, unknown/path/URL rejection and initialized-context no-I/O checks.
Verify allowlist names/bytes against the existing provenance. Preserve prior
fixtures, expectations, worker framing and resource/security gates.

Before packaging the resource-embedded binary, add the existing CMap, Foxit
and Liberation notices to the worker's strict license manifest and inventory.
All six installed-worker targets and protected CI remain required. Ordinary
PEP 517/sdist worker assembly, OS memory/process-tree containment, PDF layout,
IR/options/metadata/encryption/corpus qualification remain separate work.

## Local implementation evidence

The authored CID/Helvetica PDF first loses its CJK text in the unchanged
baseline. The bounded custom factories restore exact `한글V8 resource probe`,
and actual callbacks for both CMaps and standard fonts are asserted. Each of
182 embedded payloads matches its exact source path, byte count and SHA-256 in
the unchanged 188-file asset provenance. A Python-only offline test regenerates
the 1,644-byte PDF recipe in a temporary directory and matches its frozen hash.

Root verification passes 179 PDF feature test executions (including reused
private modules), default locked workspace tests, all-target/all-feature strict
workspace Clippy and warning-free rustdoc. Twenty notice-helper tests validate
the 36-entry bundle and each missing resource notice. Independent review is
CLEAN. A fresh macOS arm64 release-worker wheel passes all three installed probes,
345 isolated Python tests after integrating the published H0 fixtures and architecture/notice/artifact audit. Hosted
six-target checks remain required before protected merge; public PDF semantics remain pending.
