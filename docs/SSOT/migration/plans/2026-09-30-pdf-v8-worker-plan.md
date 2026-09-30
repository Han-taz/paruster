# Private PDF.js native worker supervision

Status: Private worker implementation is independently reviewed locally; protected hosted gates are pending under the approved embedded-V8 runtime plan; no production registration.

The next checkpoint moves the existing bounded V8 probe into a one-document
Rust executable and supervises it from Rust. No public parser, API/IR/error/MCP
schema or six-platform packaging claim is introduced. The ignored migration
oracle remains absent from runtime/build/tests. Node is not used.

## Ownership

- Coordinator: PDF Cargo target/feature declarations, private module wiring,
  documentation, artifact policy and hosted publication.
- Runtime Luna: private `src/worker_protocol.rs`, Unicode fixture/probe tests; fixed binary request
  and response framing with checked inclusive length caps and exact EOF.
- Hancom Luna after its option handoff: `src/worker_supervisor.rs`, `tests/worker_supervisor.rs` and a
  separate `tests/support/worker_child.rs` hostile-child fixture; lifecycle, deadline, bounded concurrent I/O and cleanup.
- SOL: native `src/bin/pdfjs_worker.rs`, integration join and independent safety
  review. Keep the existing private in-process runtime modules unchanged unless
  a demonstrated extraction regression requires a reviewed bounded fix.

## Frozen private boundary

Use a fixed magic/version/kind/u32-length header, a binary PDF request capped at
32 MiB and a JSON probe/error response capped at 4 MiB. Check lengths before
allocation and read exact payload/EOF; reject truncation, extra frames, wrong
magic/version/kind and oversized output. Error wire data uses only the existing
stable error inventory and sanitized messages. No path, URL, environment
lookup, dynamic import or filesystem request crosses the protocol.

Production resolution must eventually use a fixed executable packaged adjacent
to the native extension, never PATH or an environment override. This private
checkpoint accepts an explicit trusted executable Path internally so tests can
exercise the actual Cargo-built binary before wheel placement is established.
One fresh process handles one request and exits. A monotonic overall deadline
covers spawn, input, output and exit. Parent readers/writers must not wait for a
malicious blocked child before deadline handling. Timeout, stdout overflow,
truncation and nonzero/signal exits require kill and wait, plus joined I/O
threads. Bound concurrent private workers. Stderr is discarded or bounded and
never mixed with framed stdout.

## Required evidence

Record genuine RED before implementation. Protocol tests cover inclusive caps,
N+1, malformed headers, EOF/trailing frames and UTF-8 JSON bytes. Supervisor
tests cover success/error frames, sleeper timeout with reaping, exit/signal,
output flooding, blocked I/O and bounded concurrency. Run the real binary on
the authored CC0 PDF and compare exact page text/count to the in-process result.
Keep test-child controls outside the production worker; no unsafe public CLI
or environment-based backdoor may be added for tests.

Use fresh scoped feature tests, normal locked workspace tests, all-feature
strict Clippy/fmt, docs, audit/deny, source/artifact checks and hosted gates.
No suppressed lint, weakened cap, fixture/scoring change or API promotion.

## Remaining production gates

A separate wheel checkpoint must prove target-correct executable inclusion and
mode, development/sdist builds and six installed-wheel executions. OS-specific
worker memory containment and calibrated peak RSS remain required: a separate
process protects the parent from native fatal failure but does not itself stop
machine-wide memory pressure. CMap/standard-font factories, Unicode decoding,
metadata/operator-to-IR/layout, encryption/options and full-result parity remain
pending. This checkpoint must not advertise complete PDF support.


## Private implementation observations

The protocol receiver bounds page strings before deserializing a 201st item,
validates page-count/list equality, and handles JSON-escaped diagnostics within
a 512-character retained cap. Supervisor regression review caught consuming a
response Option before exit status was available; both values are now retained
until both are ready. The blocked-stdin test fills the pipe with 1 MiB, and the
concurrency test waits for both children to start before probing a third slot.
A separate helper abort/flood/sleep mode never appears in the production worker.

The additional CC0 ToUnicode fixture maps Hangul and an astral character to
exact `한글🧪` through actual V8 execution, UTF-8 framing and the supervised
binary. It passes with the existing host shim; no host capability or font/CMap
factory was added.


Incoming response decoding must avoid serde internally-tagged enum buffering:
use a direct response struct/map visitor so page and diagnostic budgets are
applied while decoding each field, including when status comes last. The writer
keeps the fixed status/result/error JSON shape. Escaped quotes/backslashes and
Unicode diagnostics must roundtrip through a bounded string visitor; requiring
borrow-only strings would reject valid JSON produced by the worker.
