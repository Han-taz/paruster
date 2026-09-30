# Embedded V8 and PDF.js runtime implementation plan

> **For agentic workers:** Use `superpowers:subagent-driven-development`, test-driven development, and verification-before-completion. Follow the existing GitHub Flow and SSOT/WIKI evidence rules.

**Goal:** Establish a private, feature-gated PDF.js text/metadata runtime in Rust using embedded V8, without a Node.js runtime requirement.

**Architecture:** The user's September 30 instruction explicitly supersedes the earlier pure-Rust PDF semantic-backend decision. Rust embeds V8 and evaluates the pinned PDF.js main/worker ES modules in one isolate; `globalThis.pdfjsWorker` provides the in-process worker handler. Rust owns input validation, execution limits, typed output, downstream source-neutral IR, and core projections. The first checkpoint is an isolated test/runtime spike, not production registry or Python capability registration. Production memory/fatal-failure process isolation and native wheel packaging remain mandatory later gates.

**Tech stack:** Rust 1.97, `v8 = 152.2.0`, PDF.js 4.10.38 matching the migration oracle's package version, serde JSON, and the current PDF synthetic fixtures. PDF.js assets originate from the public upstream package, never the ignored oracle checkout.

## Ownership and file map

Coordinator: root dependency/lockfile, `crates/kordoc-pdf/Cargo.toml`, SSOT design/plan/status/index and WIKI. Runtime worker: `crates/kordoc-pdf/src/v8_runtime/{mod,host,engine}.rs`, colocated tests, private module registration in `src/lib.rs`, authored runtime fixtures. No core/Python/schema changes. Separate review must verify runtime boundaries and supply-chain evidence.

## R0: Freeze assets and dependency

- [ ] Record the exact V8 crate/release pin and six native-target availability; retain source URLs and asset checksums.
- [ ] Download `pdfjs-dist@4.10.38` from the public npm registry, verify the registry SHA-512 integrity before selecting `legacy/build/pdf.mjs`, `legacy/build/pdf.worker.mjs`, LICENSE and necessary notices. Check in exact selected assets under `crates/kordoc-pdf/assets/pdfjs/`, with source/provenance and SHA-256. No network access is required by ordinary builds/tests/runtime.
- [ ] Add the optional `pdfjs-v8` feature and exact optional V8 dependency. Use `serde_json` for a bounded private probe result. Regenerate the workspace lockfile and review all transitive license/version changes; do not weaken cargo-deny or advisory checks.

## R1: Write red runtime regressions

Private seam:

```rust
struct PdfJsProbe { page_count: u32, page_text: Vec<String> }
fn probe_pdf_text(bytes: &[u8]) -> Result<PdfJsProbe, KordocError>;
```

- [ ] Add private tests for a deterministic CC0 one-page Helvetica PDF containing `V8 PDF.js probe`, including exact page count and extracted text. Tests depend only on checked-in assets and the authored recipe.
- [ ] Assert malformed PDF is a typed failure, omitted host APIs (`process`, `require`, `fetch`, browser networking) cannot become I/O capabilities, explicit wall-clock termination works for a nonterminating trusted test script, and output length overflow fails instead of truncating.
- [ ] Run `cargo test -p kordoc-pdf --features pdfjs-v8 --locked`; record genuine RED output before runtime implementation.

## R2: Implement the bounded private runtime

- [ ] Initialize V8 once through `std::sync::Once`; create a fresh isolate/context per probe, not a shared context holding prior document secrets. Set a private heap cap and external-buffer allocator cap; expose a termination handle to a deadline guard.
- [ ] Compile the two checked-in self-contained ESM files. Static imports are rejected. Inject worker namespace as `pdfjsWorker`, and main namespace as the trusted bridge. Add only bounded in-memory host compatibility: structured clone/value serialization, UTF-8 text conversion, DOMException and Promise microtask processing as required by the actual pinned modules.
- [ ] Supply document input as bounded binary bytes, never a file path or URL. Call PDF.js with `useWorkerFetch:false`, `isEvalSupported:false`, `disableFontFace:true`, `isOffscreenCanvasSupported:false`, and `isImageDecoderSupported:false`. No filesystem, network, Node module loader or document script execution is exposed. Resource factories must return embedded assets or explicit unsupported errors.
- [ ] Await the load/text promises using the isolate microtask pump. Bound source bytes, page count, extracted items/text and serialized output, before retained Rust copies. Sanitize JS exceptions into existing error codes. A timeout or output/resource failure is a typed hard failure.
- [ ] Run the RED regressions again to GREEN and record V8/PDF.js versions from actual execution. This proves runtime feasibility only; fatal native OOM containment requires a supervised native worker process before production registration.

## R3: Review, gates, and next seam

- [ ] Run fmt, strict all-feature Clippy, feature-enabled PDF tests, locked workspace tests, warning-free docs, source/artifact forbidden-path scans, audit/deny, docs checker and required hosted gates. Review asset licenses/notice and package size explicitly.
- [ ] Independent SOL review: no host I/O escape, no hidden worker import/fetch, termination and cleanup, binary/JSON allocation boundaries, and no runtime oracle dependency. Record missing raster/CMap/standard-font/process-supervision/native-wheel evidence as pending rather than claiming full support.
- [ ] Push only the feature branch and create a focused draft PR. Preserve the existing private object-reader evidence as historical research; do not continue a separate pure-Rust semantic implementation against the user's V8 decision.
- [ ] Next checkpoint maps PDF.js text/operator/font evidence into the approved source-neutral IR and existing layout/table/quality paths, adds metadata/encryption/option parity, worker-process supervision and six-target installed-wheel tests. Production `parse_pdf` remains unregistered until those gates pass.

## Required primary sources

- [Rusty V8](https://github.com/denoland/rusty_v8)
- [PDF.js](https://github.com/mozilla/pdf.js)
- [PDF.js 4.10.38 release](https://github.com/mozilla/pdf.js/releases/tag/v4.10.38)

The source/binary/asset versions are pinned for reproducibility; later updates require separate security and parity verification.
