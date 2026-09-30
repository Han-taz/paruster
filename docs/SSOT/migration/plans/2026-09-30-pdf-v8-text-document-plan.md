# Bounded PDF.js page-text and metadata document

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add a bounded, private transfer of ordered PDF.js text items and the current Info-dictionary metadata into Rust, without changing the existing worker probe or public parser contracts.

**Architecture:** PDF.js remains the semantic decoder inside the existing isolated V8 runtime. It streams default text-content chunks and projects only bounded scalar fields into a new private Rust DTO; Rust owns metadata normalization and any later layout/IR lowering. The process worker keeps existing KPDF v1 kinds 1/2 unchanged and adds separate kinds 3/4 for this DTO.

**Tech Stack:** Rust 1.97, rusty_v8 152.2.0, pinned pdfjs-dist 4.10.38, serde/serde_json, existing KPDF worker framing, deterministic stdlib-only PDF fixture generator.

---

## Status and boundary

This is a private extraction substrate. It does not implement or register a public PDF parser, expose a new Python or MCP API, change IR/error contracts, or claim full legacy PDF parity. It does not return Markdown or inferred paragraphs, headings, columns, tables, links, images, OCR, or warnings. Existing `PdfJsProbe { page_count, page_text }` and its kind-1/kind-2 frames remain byte-for-byte stable.

The transferred raw text fields match what the current oracle’s layout entry point consumes: string, width, height, six-element transform, and PDF.js font name. Page coordinates remain in PDF user space with the original `page.view` CropBox origin. Rust can apply the established rounding, origin shift, and layout policies later.

The current parser also uses PDF.js operator lists, common font objects, and annotations for glyph recovery, spacing repair, hidden/occluded text, graphics, tables, images, and links. Those are not represented here. This checkpoint must report the text-only scope and keep broader corpus parity pending until a separately bounded evidence DTO covers those inputs.

## Frozen PDF.js behavior

- For normal non-XFA pages, call `page.streamTextContent()` with no arguments and consume it in chunks. Pinned PDF.js 4.10.38 `PDFPageProxy.getTextContent()` around lines 17045–17073 aggregates `streamTextContent(params)` items in order, so this preserves the ordinary text-item sequence without collecting the entire page first. Retain default normalization/bidi behavior and do not include marked-content records. `getTextContent()` has a separate XFA branch; XFA equivalence is explicitly outside this checkpoint.
- Preserve page order, each chunk’s item order, exact `str`, `width`, `height`, `transform`, and `fontName`. Do not trim, normalize, join, reorder, or round item values in the V8 bridge.
- Preserve `page.view` exactly, including the CropBox origin. Copy `page.rotate` as the original integer multiple of 90 degrees. Do not apply viewport rotation or origin translation in V8.
- Read only fixed Info fields from `doc.getMetadata().info`: `Title`, `Author`, `Creator`, `Subject`, `Keywords`, `CreationDate`, and `ModDate`. Do not enumerate the Info object or call XMP `metadata.getAll()`.
- Leave metadata strings raw in the DTO, including empty strings. A missing or non-string value for one of the seven known Info fields becomes `null`; do not enumerate unknown fields. The later Rust parser maps trimmed Title/Author/Creator/Subject, splits Keywords on comma or semicolon, and parses PDF `D:` creation/modification dates. Full parse owns `pageMode=layout`; metadata-only owns only page count, matching the existing oracle behavior.
- Treat only retrieval/parsing errors from `doc.getMetadata()` as best-effort: return seven null fields and continue text extraction. Never catch a resource denial, allocation-cap failure, input/output cap, or deadline as a metadata fallback; those remain sticky fatal errors, with configured budget failures mapped to `OutputTooLarge`. A page or text-stream failure is fatal for this DTO because it has no warning/partial-page representation; map it to a typed parse failure instead of silently dropping that page.
- Continue the runtime’s existing no-Node/no-network/no-filesystem host boundary, embedded-resource allowlist, 32 MiB input cap, 10-second watchdog, 192 MiB V8 heap cap, and 128 MiB external-buffer cap. Keep `useSystemFonts:false`, `useWorkerFetch:false`, `isEvalSupported:false`, and built-in CMap/font factories.

## Private DTO and bounds

Add `crates/kordoc-pdf/src/v8_runtime/text_document.rs` with strict serde DTOs. The serialized kind-4 success result has this shape:

```json
{
  "page_count": 1,
  "metadata": {
    "title": "...",
    "author": "...",
    "creator": "...",
    "subject": "...",
    "keywords": "...",
    "creation_date": "...",
    "modified_date": "..."
  },
  "pages": [
    {
      "page_number": 1,
      "view_box": [0.0, 0.0, 612.0, 792.0],
      "rotation": 0,
      "items": [
        {
          "text": "...",
          "width": 18.0,
          "height": 12.0,
          "transform": [12.0, 0.0, 0.0, 12.0, 40.0, 740.0],
          "font_name": "g_d0_f1"
        }
      ]
    }
  ]
}
```

Rust signatures:

```rust
pub(crate) struct PdfJsTextDocument {
    pub(crate) page_count: u32,
    pub(crate) metadata: PdfJsMetadata,
    pub(crate) pages: Vec<PdfJsPage>,
}

pub(crate) struct PdfJsMetadata {
    pub(crate) title: Option<String>,
    pub(crate) author: Option<String>,
    pub(crate) creator: Option<String>,
    pub(crate) subject: Option<String>,
    pub(crate) keywords: Option<String>,
    pub(crate) creation_date: Option<String>,
    pub(crate) modified_date: Option<String>,
}

pub(crate) struct PdfJsPage {
    pub(crate) page_number: u32,
    pub(crate) view_box: [f64; 4],
    pub(crate) rotation: i32,
    pub(crate) items: Vec<PdfJsTextItem>,
}

pub(crate) struct PdfJsTextItem {
    pub(crate) text: String,
    pub(crate) width: f64,
    pub(crate) height: f64,
    pub(crate) transform: [f64; 6],
    pub(crate) font_name: String,
}

pub(crate) struct TextDocumentLimits {
    pub(crate) max_pages: usize,
    pub(crate) max_items: usize,
    pub(crate) max_item_text_bytes: usize,
    pub(crate) max_text_bytes: usize,
    pub(crate) max_font_name_bytes: usize,
    pub(crate) max_font_names_bytes: usize,
    pub(crate) max_metadata_value_bytes: usize,
    pub(crate) max_metadata_bytes: usize,
    pub(crate) max_response_bytes: usize,
}

pub(crate) fn extract_text_document(
    bytes: &[u8],
) -> Result<PdfJsTextDocument, KordocError>;

#[cfg(test)]
pub(crate) fn test_extract_text_document_with_limits(
    bytes: &[u8],
    limits: TextDocumentLimits,
) -> Result<PdfJsTextDocument, KordocError>;
```

Apply all limits before retaining a projected record or growing a Rust collection:

| Field | Inclusive cap | Overflow behavior |
| --- | ---: | --- |
| PDF input | 32 MiB | `OutputTooLarge` |
| Pages | 200 | `OutputTooLarge`; no truncation |
| Text items | 100,000 total across all pages | `OutputTooLarge`; no truncation |
| One text string | 64 KiB UTF-8 | `OutputTooLarge` |
| All text strings | 2 MiB UTF-8 | `OutputTooLarge` |
| One font name | 128 UTF-8 bytes | `OutputTooLarge` |
| All font names | 512 KiB UTF-8 | `OutputTooLarge` |
| One metadata value | 4 KiB UTF-8 | `OutputTooLarge` |
| All seven metadata values | 16 KiB UTF-8 | `OutputTooLarge` |
| Serialized kind-4 response | 4 MiB | `OutputTooLarge` |
| V8 execution | existing 10-second deadline | typed runtime failure |

On the V8 side, reject a JavaScript string by cheap `String.length` before any UTF-8 scan; then measure exact UTF-8 bytes for per-field and aggregate caps. Count text/font/metadata bytes before retaining each projected record. Serialize only each bounded projected item, account its exact UTF-8 JSON bytes plus document/page framing before appending, and assemble the final response from those bounded fragments; do not stringify PDF.js-owned item/style/metadata objects or expose `TextContent.styles` wholesale. Measure final V8 string `utf8_length` and reject before copying it into Rust if it exceeds 4 MiB. The Rust decoder precharges shared document-wide counts and aggregate byte totals before retaining decoded strings, uses fallible reserve for each collection, ignores attacker-controlled `size_hint`, and immediately rejects an over-limit N+1 sequence using a rejecting seed, without traversing its nested payload. Escaped JSON strings may transiently use serde_json's bounded input-frame scratch; the 4 MiB response frame is the upper bound for that transient parse buffer. Resource/output/allocation errors are checked after async extraction settles and override any caught best-effort metadata error.

On the Rust side, use a shared-budget `DeserializeSeed` for nested `pages`, `items`, and strings; reject unknown and duplicate fields. Check each sequence count before fallible reserve/push and ignore attacker-controlled `size_hint`. Precharge shared item/text/font/metadata counters before retaining decoded strings; the already-capped 4 MiB frame bounds serde_json's transient scratch for escaped strings and map keys. Validate nonempty document page count, `pages.len()==page_count`, one-based monotonically ordered page numbers, finite view/width/height/transform scalars, `rotation % 90 == 0`, and font-name aggregate limits. Reject malformed or over-budget data; never clip or silently omit text.

The result uses the existing stable error inventory. Invalid geometry or malformed JSON maps to `ParseError`; any count/byte/frame limit maps to `OutputTooLarge`. Do not add error variants.

## Ownership

| Owner | Files | Responsibility |
| --- | --- | --- |
| Runtime owner (Luna B) | `crates/kordoc-pdf/src/v8_runtime/{mod.rs,engine.rs,text_document.rs}` | V8 streaming projection, DTO, validation hook and focused in-process tests |
| Protocol/binary owner (SOL, after HWPML handoff) | `crates/kordoc-pdf/src/worker_protocol.rs`, `src/worker_supervisor.rs`, `src/bin/pdfjs_worker.rs` | New kind-3/kind-4 request/response path, exact EOF, supervised subprocess tests; must preserve kind 1/2 |
| Fixture owner (runtime owner) | `crates/kordoc-pdf/tests/fixtures/pdfjs_text_document/{generate.py,document.pdf,README.md}` | Deterministic authored input and provenance; no oracle-derived files |
| Coordinator | `.github/workflows/pdf-worker-wheels.yml`, `tests/support/pdf_worker_wheel.py`, `tests/support/test_pdf_worker_wheel.py`, SSOT/WIKI | Extend installed-worker smoke to kinds 3/4 and six-target evidence after protocol join |
| Later Rust PDF parser/layout owner | `crates/kordoc-pdf/src/{parser.rs,document.rs,geometry.rs,layout.rs,lines.rs}` | Normalize raw metadata and later consume bounded items into layout/IR; not part of this DTO implementation |

The coordinator approved this plan for runtime/DTO implementation. Protocol and binary work proceeds under SOL’s separate ownership once the DTO entry points are available. Do not edit `kordoc-ir`, `kordoc-core`, Python APIs, contracts, assets/license inventory, existing fixtures, or worker-wheel ownership files in this checkpoint.

## Tasks

### Task 1: Author the deterministic two-page fixture and RED extraction assertions

**Files:**

- Create `crates/kordoc-pdf/tests/fixtures/pdfjs_text_document/generate.py` (stdlib only).
- Create `crates/kordoc-pdf/tests/fixtures/pdfjs_text_document/document.pdf`.
- Create `crates/kordoc-pdf/tests/fixtures/pdfjs_text_document/README.md` with CC0 1.0 authorship, exact command, byte count, and SHA-256.
- Create `crates/kordoc-pdf/tests/pdfjs_text_document.rs` behind `#![cfg(feature = "pdfjs-v8")]`.
- Read only: pinned `assets/pdfjs/legacy/build/pdf.mjs`/`pdf.worker.mjs`, existing `pdfjs_unicode.rs`, `pdfjs_resources.rs`, and `tests/pdfjs_assets.rs` for fixture conventions.

- [ ] **Step 1: Generate a CC0 fixture without consulting or writing the oracle.**

The recipe must construct valid PDF object/xref bytes using the Python standard library. Page 1 has two lines and text runs whose content-stream order differs from their x-coordinate order. Page 2 has a nonzero CropBox origin and a 90-degree page rotation. A ToUnicode CMap includes Korean, an astral character, and U+FB03, whose pinned default normalization is frozen as `ffi`; an initialized-V8 comparison asserts `disableNormalization:true` differs. The Info dictionary contains whitespace-padded Title/Author/Creator/Subject, comma-and-semicolon Keywords, and D: CreationDate/ModDate. Include one blank page if practical to test page evidence, otherwise keep both pages text-bearing. Re-run generation twice in separate temporary directories and require identical byte output.

- [ ] **Step 2: Add failing extraction tests before the DTO implementation.**

The feature integration must import the private runtime by path as existing tests do. Add tests for exact page count/order, exact PDF.js item sequence/text (including Korean/astral UTF-8 bytes), raw CropBox-origin `view_box`, unrounded six-number transforms, rotation, and the seven raw Info fields. Include an empty-string Info field and a non-string Info field to prove empty values are preserved while non-strings become null. The initial compile/test must fail because `PdfJsTextDocument`/`extract_text_document` do not exist; save this failure as genuine RED evidence in the PR description, not as a committed generated log.

- [ ] **Step 3: Run the RED fixture checks and verify deterministic bytes.**

Run:

```bash
python3 crates/kordoc-pdf/tests/fixtures/pdfjs_text_document/generate.py --output /tmp/pdfjs-text-document-a.pdf
python3 crates/kordoc-pdf/tests/fixtures/pdfjs_text_document/generate.py --output /tmp/pdfjs-text-document-b.pdf
cmp /tmp/pdfjs-text-document-a.pdf /tmp/pdfjs-text-document-b.pdf
cargo +1.97.0 test -p kordoc-pdf --features pdfjs-v8 --test pdfjs_text_document --locked
```

Expected: deterministic recipe comparison passes; Rust test compilation fails on the intentionally absent DTO/extraction entry point. Do not weaken the assertions to make the baseline compile.

### Task 2: Implement the bounded in-process DTO extraction

**Files:**

- Create `crates/kordoc-pdf/src/v8_runtime/text_document.rs` for serde DTOs, limits and validation.
- Modify `crates/kordoc-pdf/src/v8_runtime/{mod.rs,engine.rs}` only for extraction hook and V8 projection.
- Modify `crates/kordoc-pdf/tests/pdfjs_text_document.rs` only for exact runtime and boundary evidence.

- [ ] **Step 1: Define strict private DTO types and test validation failures.**

Derive serialization for Rust worker output and deserialization for parent input. Every DTO struct denies unknown fields. Implement nested page/item/string visitors, finite-number checks and page/order/aggregate validation before allocations grow past configured limits. Add unit tests for page count mismatch, wrong page number, non-finite geometry, transform length other than six, invalid rotation, maximum font-name field/aggregate, and inclusive text/page/item/metadata caps. A low metadata-byte limit against the nonempty authored Info fixture must return `OutputTooLarge`, not seven nulls; this proves metadata best-effort cannot swallow cap denials.

- [ ] **Step 2: Add the bounded runtime function using default PDF.js extraction semantics.**

Implement `extract_text_document(bytes)` using the existing isolated V8 setup and resources. Check the same 32 MiB input boundary. Load `getDocument` with the current host restrictions and run `doc.getMetadata()` plus a page loop capped at 200. If metadata retrieval throws or one of the fixed Info fields is non-string, use null for those fields and continue. Preserve empty string values. Do not catch V8 allocator rejection, output overflow, a sticky resource denial, or deadline as metadata fallback; check sticky runtime failures after async extraction settles. For each page, copy exactly four `page.view` scalars and `page.rotate`; consume `page.streamTextContent()` with no options and retain each chunk’s items in order. For each item project only `str`, `width`, `height`, `transform`, and `fontName`; reject wrong scalar types, non-finite geometry, invalid transform length, or budget overflow before appending. Select only the seven named `info` fields and do not enumerate the source object or inspect XMP. Any page/text-stream exception fails the document rather than silently omitting a page. Always call `task.destroy()` in a finalizer.

- [ ] **Step 3: Verify the focused runtime RED tests are now GREEN.**

Run:

```bash
cargo +1.97.0 test -p kordoc-pdf --features pdfjs-v8 --test pdfjs_text_document --locked -- --nocapture
```

Expected: the authored fixture returns the exact text item sequence and coordinates; no item text is joined or reordered. Metadata matches raw Info values, while all cap/invalid-value cases return typed errors.

- [ ] **Step 4: Verify streamed output against the default `getTextContent()` contract.**

Add a test-only V8 hook that reads both `page.getTextContent()` and the DTO’s no-argument `page.streamTextContent()` projection from the same non-XFA authored page, then compares each item’s `str`, `width`, `height`, `transform`, and `fontName` in order. The deterministic fixture also pins expected default-normalized values. Separately call `getTextContent({disableNormalization:true})` in that initialized V8 context and assert the ligature’s text differs, so the fixture detects an accidental option change. XFA input remains outside the equivalence assertion. Keep the test independent of host fonts and network; CMap/standard-font request counts can be observed with private stats where useful.

### Task 3: Add the version-preserving kind-3/kind-4 private worker path

**Files:**

- Protocol owner: `crates/kordoc-pdf/src/worker_protocol.rs`.
- Protocol owner: `crates/kordoc-pdf/src/worker_supervisor.rs` and `src/bin/pdfjs_worker.rs`.
- Protocol owner: `crates/kordoc-pdf/tests/pdfjs_worker.rs` or a new `tests/pdfjs_text_worker.rs`.
- Runtime owner supplies only the DTO type and `extract_text_document` signature from Task 2.

- [ ] **Step 1: Freeze old kind-1/kind-2 behavior with byte-exact regression assertions.**

Before adding new variants, assert the current request header/payload and a known success/error response frame’s header and JSON payload bytes exactly. Run existing `worker_protocol`, `pdfjs_unicode`, and `pdfjs_worker` tests unchanged.

- [ ] **Step 2: Add request kind 3 and response kind 4 without changing version 1.**

Keep the 10-byte KPDF header and magic/version unchanged. Add separate read/write functions for kind 3 (one raw PDF payload, same 32 MiB request cap) and kind 4 (success DTO or existing sanitized stable error, same 4 MiB response cap). Add a worker input dispatcher that accepts exactly old kind 1 or new kind 3 and selects the paired response kind. Preserve current worker behavior byte-for-byte for every kind-1 request, including malformed framing: it returns the old kind-2 sanitized protocol error frame. Unknown kinds also follow the existing kind-2 framing-error behavior. Once the valid six-byte magic/version/kind-3 prefix is recognized, even a truncated length header returns the paired kind-4 error frame; malformed payload/EOF and parse/runtime failures also use kind 4. Shorter prefixes, invalid magic/version and unknown kinds retain kind-2 errors. Keep `read_request`, `write_request`, `read_response`, `write_response` semantics and payload structs for kinds 1/2 unchanged. Both request forms require exact EOF. Parent DTO input uses bounded visitors before page/item vectors can exceed limits.

- [ ] **Step 3: Add a sibling supervisor entry point and real binary tests.**

Add `supervise_text_document_worker(executable, bytes, timeout)` beside the existing API. Keep path resolution, deadlines, child reaping, I/O thread joining, concurrency slots and sanitized errors shared. Test a kind-3/kind-4 rich response from the actual Cargo-built worker, malformed/truncated/wrong kind/extra frame, timeout and nonzero exit. Assert old kind 1/2 tests still pass byte-for-byte.

- [ ] **Step 4: Run the protocol/runtime focused suite.**

Run:

```bash
cargo +1.97.0 test -p kordoc-pdf --features pdfjs-worker-tests --locked --test pdfjs_text_document
cargo +1.97.0 test -p kordoc-pdf --features pdfjs-worker-tests --locked --test pdfjs_worker
cargo +1.97.0 test -p kordoc-pdf --features pdfjs-worker-tests --locked --test worker_supervisor
```

Expected: new rich frames succeed through the installed Rust child; all old kind 1/2 wire bytes and behavior are unchanged; malformed frames fail closed; request/response `N` boundary succeeds and `N+1` fails before growth.

### Task 4: Extend installed-wheel evidence and run protected gates

**Files:**

- Coordinator: `tests/support/pdf_worker_wheel.py`, its colocated unit tests, `.github/workflows/pdf-worker-wheels.yml`.
- Runtime/protocol owners provide the fixture and kind-3/kind-4 smoke invocation; do not edit wheel files without reassigned ownership.

- [ ] **Step 1: Extend the bounded frame helper with an explicit text-document smoke mode.**

The helper sends the authored PDF as kind 3, reads exactly one kind-4 response under 4 MiB, checks page count, raw text item order/UTF-8/font names, page views/rotation and metadata fields. Validate no trailing frame and verify the executable path resolves to installed `kordoc/_bin/pdfjs-worker[.exe]`. Do not import the ignored oracle.

- [ ] **Step 2: Run kind 1/2 and kind 3/4 smoke on every existing wheel matrix target.**

Keep the existing six targets, architecture checks, license manifest/inventory, manylinux 2.28 container checks, Python versions, locked builds, and protected gates intact. The new rich fixture must run through the cleanly installed worker for Linux x86_64/aarch64, Windows x86_64/ARM64, and macOS x86_64/ARM64. Windows ARM64 remains tested at its supported configured CPython version; do not claim untested CPython 3.10 availability there.

- [ ] **Step 3: Run local quality checks before coordinator review.**

Run:

```bash
cargo +1.97.0 fmt --all -- --check
cargo +1.97.0 clippy --workspace --all-targets --all-features --locked -- -D warnings
cargo +1.97.0 test --workspace --locked
cargo +1.97.0 test -p kordoc-pdf --features pdfjs-worker-tests --locked
cargo +1.97.0 doc --workspace --no-deps --locked
python3 -m unittest discover -s tests/support -p 'test_pdf_worker_wheel.py'
```

Expected: all locked tests and strict Clippy pass; docs are warning-free; wheel helper tests pass. Then the coordinator runs the existing six-target hosted gate, security/artifact scans, and installed worker smoke. A kind-3/kind-4 local success is not itself evidence that every wheel is portable.

## Open scope after this checkpoint

- Operator-list evidence and `page.commonObjs` are required for oracle glyph-name restoration, spacing/synthetic-space repair, hidden/occluded text, table lines, vector/image regions, and image extraction.
- `page.getAnnotations()` is required for link annotations.
- Rust PDF geometry/layout/table modules remain planned; this DTO does not produce `ParsedDocument` or public IR.
- Page selection, metadata-only operation, encrypted PDFs/password behavior, warnings/partial pages, OCR, source-provenance disclosure, six-wheel release support, and complete oracle parity remain separate gates.
- `useSystemFonts:false` is mandatory in the isolated runtime and differs from the oracle’s `useSystemFonts:true`; static embedded resources do not establish parity for host-substituted fonts.
- The local migration oracle may inform later parity research only. No oracle source, fixture, output, path, package or runtime dependency may enter this plan’s implementation, tests, build, or wheel.
