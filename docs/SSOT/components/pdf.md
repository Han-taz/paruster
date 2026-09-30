# PDF runtime and historical substrate

## Current runtime direction

The user reconfirmed that PDF.js itself remains JavaScript running inside V8;
Rust owns its execution management and the rest of the port. This is the
explicit PDF exception to a pure Rust semantic implementation, not a Node.js
runtime dependency.

The September 30 user decision requires PDF.js embedded in V8 for semantic extraction. The active [runtime plan](../migration/plans/2026-09-30-pdf-v8-runtime-plan.md) pins `v8 = 152.2.0` and upstream `pdfjs-dist = 4.10.38`, matching the oracle version without copying oracle assets. The private `pdfjs-v8` feature is a feasibility probe; default builds expose no PDF parser. Rust owns binary input, execution/output limits, typed results, and downstream IR/layout policy. Production PDF registration requires OS-calibrated worker containment, full-result parity, resource factories, and six-target installed-wheel evidence. The [private worker checkpoint](../migration/plans/2026-09-30-pdf-v8-worker-plan.md) now adds process supervision without claiming those remaining gates. The existing borrowed-source reader remains private historical research and is not developed as a competing semantic backend.

## Private runtime evidence

The `pdfjs-v8` feature now executes the checked-in main/worker modules in a fresh isolate and returns exact page count and text for the authored one-page Helvetica fixture. The executing V8 engine reports `15.2.124.1-rusty`; the bridge checks PDF.js `4.10.38` at runtime. The scoped suite has 8 passing executions: extraction, prefixed corrupt input, initialized-context host-I/O absence, inclusive serialized-output boundary, actual watchdog termination, custom allocator unit boundary, actual JS ArrayBuffer over-cap rejection/release, and V8 version.

Private feasibility limits are 32 MiB binary input, 200 pages, 4 MiB serialized UTF-8 output, 192 MiB V8 old-generation heap, 128 MiB V8-owned ArrayBuffer backing stores and a 10-second watchdog. The custom allocator charges input before copying and rejects further buffers before allocation. The actual over-cap JS probe produces `RangeError`, records rejection, maps to existing `OUTPUT_TOO_LARGE`, and releases all charged bytes when its isolate drops. Heap/native allocations outside that allocator and fatal OOM are not contained by this in-process spike; these values are not a production acceptance/RSS guarantee.

Default builds and the public registry still expose no PDF parser. The narrow ReadableStream, URL and structuredClone shims prove this fixture only; complete transfer/Web API semantics, text decoding, CMap/standard-font factories, metadata/operator evidence and complex documents remain pending; the private process checkpoint below covers the current probe only. The checked-in upstream source/resources total 5,106,488 bytes across 188 provenance-pinned files, with all applicable licenses included and a default-feature offline integrity test. Licensed resources are ready for future factories; their presence is not evidence those factories are implemented.

Dependency policy passes without changes. `cargo audit --file Cargo.lock` reports the upstream V8 build-time `paste 1.0.15` unmaintained warning [RUSTSEC-2024-0436](https://rustsec.org/advisories/RUSTSEC-2024-0436.html); it reports no listed vulnerability and no ignore is added. Production release must revisit upstream replacement and keep advisory checks intact.

## Private native worker checkpoint

A feature-gated `pdfjs-worker` Rust executable reads exactly one framed binary
PDF request and writes exactly one framed JSON result/error. It reuses the
same pinned V8/PDF.js modules, with no Node runtime or additional host I/O.
The parent supervisor takes only an explicit trusted absolute executable path
at this private seam, so it never searches PATH or a user environment override.
A fixed packaged sibling resolver is a later wheel checkpoint.

Framing uses `KPDF`, version 1, request/response kind and a big-endian u32
payload length. Requests allow at most 32 MiB, responses at most 4 MiB; lengths
are checked before allocation. The receiver requires exact EOF and rejects
wrong headers/kinds/versions, truncation, multiple frames, invalid JSON,
unknown fields and unsupported error codes. Page strings are deserialized
through a 200-element visitor, rejecting the next element before materializing
it, and must match page count. Errors preserve existing codes and bounded
control-cleaned diagnostics. Unicode text remains UTF-8.

Two worker slots bound concurrency; slot acquisition, I/O and process-exit
monitoring share one monotonic deadline. Reader/writer threads use bounded
protocol operations while the supervisor polls exit, kills/waits on timeout or
failure, and joins both threads. Stderr is discarded. One fresh child handles
one document and exits, so document state is not reused. Correctness tests
cover EOF arriving before exit status, actual blocked stdin, open stdout,
nonzero/abort exits, flood/header caps and worker-slot pressure.

The real Cargo-built worker matches direct extraction for the original
Helvetica probe and an authored Type0/Identity-H PDF with embedded ToUnicode
mapping yielding exact `한글🧪`. The Unicode recipe, full byte reconstruction,
provenance and UTF-8 protocol roundtrip are checked in; it requires no external
font or CMap factory. This establishes those two synthetic probes, not general
font/resource support or representative document parity.

Hostile-child controls live in a separate `pdfjs-test-worker` target under the
explicit `pdfjs-worker-tests` feature. Production worker code has no test modes.
CI enables this feature for runtime/supervisor tests. Default builds and wheels
still register no PDF parser and do not bundle either executable.

Process separation prevents a worker native abort from corrupting its parent;
it does not enforce total OS memory usage, process-group/descendant cleanup,
spawn syscall latency or a hard end-to-end wall time independent of scheduling.
Calibrated Linux/macOS resource limits, Windows Job Objects, target-correct
six-wheel executable placement/mode, developer/sdist builds and installed-wheel
supervised tests remain mandatory before production registration. Existing
in-isolate buffer/heap/watchdog caps remain unchanged; no production RSS or
full PDF support claim is made.

## Separate worker-wheel feasibility gate

The [worker-wheel plan](../migration/plans/2026-09-30-pdf-worker-wheel-plan.md)
adds a separate CI workflow that stages the optional native worker into
`kordoc/_bin/pdfjs-worker[.exe]`. Fresh default wheels do not run this staging
step. This establishes a packaging checkpoint, not a public parser or an
ordinary PEP 517/sdist worker build.

Six native targets build the worker and abi3 extension, audit actual
ELF/PE/thin-Mach-O architecture and archive executable mode, then install and
execute the worker against the unchanged Helvetica and Hangul/astral probes.
Linux builds inside the manylinux 2.28 container and repeats the installed
probes in a matching baseline runtime. Windows ARM64 uses available native
CPython 3.12; its CPython 3.10 runtime floor remains unproven. Hosted six-target
success remains pending until the new workflow passes.

The notice-only [provenance manifest](../../../crates/kordoc-pdf/assets/v8-licenses/PROVENANCE.json)
pins 33 upstream notices totaling 221,177 bytes for PDF.js, Rusty V8, pinned V8
and identified native dependencies. Source, staged, wheel and installed bytes
are SHA-256 checked; omitted inventory and duplicate ZIP entries fail closed.
Targeted Git attributes preserve pinned assets and PDF inputs during Windows
checkout. A defensive CRLF restoration is accepted only when it reproduces the
same pinned notice hash; tampered bytes still fail. Both probe inputs and the
existing legacy PDF fixture keep exact checkout bytes.
All upstream sources were independently reverified, including the original
PDF.js npm tarball SHA-512 and `package/LICENSE`. This is a notice bundle for
identified embedded assets, not a blanket audit of every Rust dependency.

A macOS arm64 wheel built after the HWPX warning merge passes archive/artifact
audits and **339** tests in isolated CPython 3.10.19 (320 existing API/contract/
parity cases plus 18 tooling regressions), then extracts exact ASCII and
`한글🧪` through the installed worker. Its worker is 61,976,928 bytes with mode
0755 and declares macOS 11.0, matching this local wheel tag. Other targets
require hosted execution evidence; OS memory containment, fixed production
resolver, resource factories and full IR/layout/option parity remain pending.

## Historical private reader

This page defines the private PDF object-access substrate established by P2a
Task 0. It is not a public parser contract and does not claim PDF, Python, or
MCP capability. Higher-level page, glyph, layout, table, asset, and quality
semantics remain pending.

## Substrate decision

The pinned `lopdf = 0.45.0` candidate is rejected for runtime object loading.
Its document reader eagerly materializes ordinary objects and object streams,
copies stream payloads before caller-controlled charging, applies only
per-stream decompression limits, and runs predictor expansion outside that
limit. Its relevant parser phases are not public hooks. These properties
cannot satisfy pre-allocation, cumulative-byte, object-count, recursion, or
500 MiB no-second-copy gates. The dependency remains pinned only as an audited
candidate until a later change removes it or a separately reviewed use is
approved; the Task 0 runtime path does not call it.

`kordoc-pdf` instead owns a small borrowed-source reader. It locates the final
`startxref` in a bounded tail window, walks classic or xref-stream revision
chains, charges every distinct xref ID including free entries before map
insertion, and parses an indirect object only when requested. All reference
walks share active-ID and depth state. Trailer dictionaries, strings, comments,
and stream boundaries are lexed so data bytes cannot become references or hide
an encryption marker.

## Current supported surface

- classic xref tables, xref streams, incremental `/Prev` revisions, generation
  checks, and type-2 compressed-object lookup;
- borrowed ordinary-object values and charged owned compressed-object values;
- unfiltered and ASCIIHex stream payloads under 32 MiB per-stream and 256 MiB
  cumulative decoded ceilings;
- typed fail-closed rejection for unsupported or multi-filter streams,
  encryption, malformed lengths/offsets/generations, cycles, and budget
  violations;
- the shared `PdfBudget` counters frozen in the implementation plan, including
  source, object/dereference/depth, decoded bytes, operators, glyphs, text, IR,
  and pixels.

Flate/predictor decoding, indirect stream lengths, fonts/CMaps, page trees,
content operators, and all semantic IR lowering are deliberately deferred to
the superseded P2a design. They are not active implementation tasks under the V8 decision. Filtered xref streams therefore fail closed today.

## Security evidence

Deterministic CC0 fixtures and the reproducible per-probe error/RSS table live
in [`tests/golden/document/pdf/README.md`](../../../tests/golden/document/pdf/README.md).
The adversarial suite covers the inclusive source and decoded-byte boundaries,
the million-ID boundary for normal and free classic/xref-stream entries,
object and form recursion, compressed-container cycles and index overflow,
allocation-free oversized token/filter/index/reference inputs, encryption
trailer lexical evasions, stream-payload false references, and the 500 MiB
sparse probe. These are substrate checks only; they do not advance the
real-document parity numerator.

Installed worker probes explicitly use `python -I -X utf8`: isolated mode
ignores environment-based `PYTHONUTF8`, and the explicit flag preserves exact
Hangul/astral status output on Windows. All six targets remain mandatory.

The separate worker-wheel feasibility checkpoint merged in PR
[#27](https://github.com/Han-taz/paruster/pull/27) as
`8d565cc6e4a05ffd16c3686a87f4cc4449b3008f`. All six installed-worker targets
and required CI/security/wheel/fuzz gates passed on final head `e981f6c`.
This remains CI-only staging and does not register a production PDF parser.

## Private embedded CMap and standard-font factories

The merged [resource checkpoint](../migration/plans/2026-09-30-pdf-v8-resources-plan.md)
adds custom PDF.js factories backed by Rust `include_bytes!` assets: 168 CMaps
and 14 standard-font payloads, 1,940,006 bytes total. Exact allowlisted names,
kinds and ASCII basenames <=128 bytes are mandatory. Resource URL options are
null, `useWorkerFetch` and `useSystemFonts` are false, and no I/O host APIs are
introduced. Every callback counts before argument conversion; a cheap UTF-16
length check bounds subsequent UTF-8 scanning and copying. V8's existing
allocator charges the returned `Uint8Array` backing store.

Per-document bounds are inclusive: 192 KiB per item, 512 requests and 8 MiB
cumulative served bytes. The existing heap, allocator, input, output, page and
watchdog limits stay intact. Unsupported requests yield a recorded `PARSE_ERROR`,
resource exhaustion yields `OUTPUT_TOO_LARGE`, and recorded callback denials
override PDF.js fallback success. Private counters remain outside the worker
wire and public IR.

The new CC0 CID/Helvetica input restores previously missing Korean text;
real callback counts prove both resource types are supplied. Boundary, unknown
name/path/URL, huge/non-string argument, strict typed failure, initialized
no-I/O and subprocess-equivalence tests pass. All 182 embedded byte/name/hash
mappings match unchanged upstream provenance. The strict wheel notice inventory
now includes the existing CMap, Foxit and Liberation notices: 36 entries,
229,224 bytes. The separate six-target workflow retains both old probes and
adds the new resource probe. PR [#29](https://github.com/Han-taz/paruster/pull/29) merged as
`f7528a6a40b1df80617c961f2225c40eeb4b7f68` after required CI and all six
installed-worker targets passed on final head `11fbabd`;
full PDF IR/layout/metadata/options/corpus and production containment/assembly
are still separate gates.


## Private ordered text and Info metadata transfer

The [text-document checkpoint](../migration/plans/2026-09-30-pdf-v8-text-document-plan.md)
adds explicit KPDF v1 request/response kinds 3/4 beside unchanged kinds 1/2.
Rust owns the same supervised child lifecycle, framing and strict DTO decoder.
For normal non-XFA pages, default PDF.js text streaming preserves item order,
raw strings, width/height, six transform values, font names, CropBox-origin view
and rotation. Seven fixed Info metadata strings remain raw; missing/non-string
values become null and empty strings stay empty. Metadata retrieval failures
are best-effort, while resource/quota failures remain fatal. XFA, operator-list
and annotation parity are separate work.

Document-wide limits are inclusive: 200 pages, 100,000 items, 64 KiB per text
string and 2 MiB total text, 128 bytes per font name and 512 KiB total names,
4 KiB per Info value and 16 KiB total metadata, and 4 MiB response frames.
Cheap UTF-16 length checks precede exact UTF-8 scans; bounded item fragments
account exact JSON bytes before retention. Shared Rust deserialization budgets
reject excessive counts/bytes, unknown/duplicate fields and invalid geometry;
fallible collection growth ignores input size hints. An extra sequence value
is rejected immediately. Escaped strings/map keys can use transient serde
scratch up to the capped input frame; this is not a zero-allocation guarantee.
Internal sticky quota state and anchored decoder markers keep attacker error
text from changing parse failures into quota failures.

The authored two-page fixture covers reversed stream/x order, nonzero CropBox,
90-degree rotation, Hangul/astral text and default U+FB03 normalization, plus
raw empty and whitespace metadata. Offline recipe/complete-result hashes and
Git autocrlf byte preservation are verified. All six installed worker targets
retain the three old probes and add exact full kind-4 result comparison.
This remains a private unregistered substrate; Rust layout/IR lowering and
production containment/packaging remain pending.


## Private Rust metadata normalization

The [metadata-normalization plan](../migration/plans/2026-09-30-pdf-rust-metadata-plan.md)
is approved for a pure Rust conversion from bounded raw Info fields into the
existing metadata type. Six frozen observations from three authored PDFs cover
full-parse metadata projection and the metadata-only result, including ordered
duplicate keywords, empty delimiter-only arrays, partial unanchored dates and
invalid calendar components. The private normalizer must preserve ECMAScript
trim behavior and omit empty text fields while revalidating 4 KiB per field
and 16 KiB total before fallible allocation. Full mode retains layout page mode;
metadata-only leaves it absent. Ten focused native cases and six captured
metadata projections pass; independent review is clean. Fresh main33 integration passes all four installed-worker probes and 360
Python/helper cases plus ten subtests; hosted gates remain before publication. No geometry, DTO, wire or public parser contract changes are approved.

PR [#32](https://github.com/Han-taz/paruster/pull/32) merged the private text
transfer as `762ddde4050660abaaa815889ed76da21a589612` after final-head
required checks and all six four-probe installed-worker targets passed.

## Private Rust base geometry

The [geometry checkpoint](../migration/plans/2026-09-30-pdf-rust-geometry-plan.md)
projects finite page frames and text positions in Rust. Frames retain unrotated
CropBox dimensions and separate page rotation. Text translations reproduce
V8 Math.round before subtracting the fractional CropBox origin, including
negative zero and the float immediately below 0.5. Positive ordered views,
page numbers, rotation multiples, all transform values and derived arithmetic
are validated; invalid geometry yields the existing ParseError. No input-sized
allocation or new magnitude clamp is introduced. One authored PDF, actual
worker DTO, focused source projection and pinned V8 IEEE vectors establish this
micro-contract. Nine focused cases and 252 scoped feature executions pass with
independent review. Full text normalization, filtering, annotation/operator
handling, IR/layout and public PDF registration remain pending.


PR [#34](https://github.com/Han-taz/paruster/pull/34) merged private Rust
metadata as `bc61d32a8fbf2fdea2e2ace3e36104d3eb1e56c6` after all
required checks and six installed worker targets passed.

## Private Rust base text scalars

The [scalar checkpoint](../migration/plans/2026-09-30-pdf-rust-text-normalization-plan.md)
consumes bounded raw items in Rust, trims ECMAScript whitespace, preserves
one-based source sequence gaps, rounds positions/dimensions, estimates font
size and projects vertical/hidden flags before sorting by y descending, x
ascending and original sequence. Signed zeros compare as equal coordinates.
An in-place unstable sort with the complete unique sequence key preserves
source tie order without stable-sort scratch allocation. All raw numeric
fields and existing 100,000-item, 64 KiB/2 MiB text and 128-byte/512 KiB font
budgets are preflighted before fallible output reservation/copies. Nonfinite
derived metrics reject as ParseError; no clamp or magnitude cap is added.

Rust directly reproduces pinned V8's finite two-argument Math.hypot fast path
with separately rounded scaled operations. Direct Rust f64::hypot changes a
captured font-size boundary from zero to one, so it is unsuitable here. Actual
PDF.js 20-item evidence projects to 15 oracle records; separate synthetic
captures cover trim/NEL/whitespace gaps/ties and V8 IEEE vectors. Thirteen
focused cases and independent review pass. The six native worker builders run
release metadata, geometry and scalar tests plus all four installed probes.
Source rewrites, splitting, deduplication, spacing/operator flags, filtering,
full line/layout/IR and public registration remain pending.

The combined main35 base-text candidate passes 265 scoped executions and
release native 9/10/13 geometry/metadata/scalar tests. Fresh base and optional
macOS ARM worker wheels pass 366 isolated Python/helper cases plus ten subtests;
all four installed probes, artifact inventories and eleven checkout bytepins
pass. The initial unchanged 100ms supervisor startup-marker failure and later
unchanged full-suite success are recorded in the geometry WIKI. No deadline or
quality gate is weakened. Final-head hosted qualification precedes publication.

The private geometry/scalar checkpoint merged in PR [#36](https://github.com/Han-taz/paruster/pull/36)
as `c5be99bd3cd4c0508a6e9237fb64c2e30a12d30a` at 2026-09-30 14:58:13 UTC.
All 42 final-head checks passed, including the five aggregate gates, actual
CodeQL analyses and all six native worker-wheel targets.

The private [text-rewrite checkpoint](../migration/plans/2026-09-30-pdf-text-rewrites-plan.md)
implements one bounded Rust text helper: ECMAScript trim, selective per-character
NFKC only for U+2F00–U+2FD5, literal-space removal for the exact numeric class,
and the rounded-font uppercase-label rule. Twenty text-only source observations
and all 214 independently captured radical mappings pass. The pinned optional
Unicode dependency reuses the existing 0.1.25 lock entry without external version
changes. Nine focused tests and independent review pass; the helper remains
unwired. Splitting/orientation/sorting must be assembled in source order in a
later slice. Full PDF parsing, layout, public registration and containment
qualification remain pending.

Fresh rewrite base and optional-worker wheels each pass 368 Python/helper cases
plus ten subtests; both base artifacts and the worker architecture/notices audit
pass, as do all four installed probes. The existing unchanged 100ms supervisor
startup-marker race reproduced in one coordinator feature run and is under
separate investigation; an independent full feature run passed. Hosted final-head
gates remain before this new checkpoint can be merged.
