# Workspace architecture

This page records the current foundation and P7 branch implementation. It describes the
working interfaces and boundaries, not completed document-processing parity.

## Components and data flow

The Rust workspace is split into three crates. `kordoc-ir` defines shared wire
DTOs plus the Rust-only `ParsedDocument` and `ParseOptions` handoff for future
format crates. `kordoc-core` owns bounded format detection, container preflight, a
private injectable registry, panic containment, result assembly, and P7's
shared Markdown/page/chunk projections, table policy, and one Markdown
table-unit reader. Format
crates depend on IR only; core-side adapters call them, preventing a Cargo
cycle. The production registry remains empty until a real parser passes parity.
`kordoc-python` validates owned options and translates calls and wire results.

The `python/kordoc` Python facade normalizes supported caller inputs—bytes-like
values, filesystem paths, and binary streams—into bounded bytes before calling
the native extension. The extension releases the GIL while Rust detection,
dispatch, or default projection runs. A caller-supplied page renderer instead
runs synchronously with Python attached to the GIL and receives deeply
immutable page blocks. Core results are serialized through the shared IR types and
translated into the facade's immutable Python result and typed-error surface.

```text
Python caller
  -> Python facade: normalize bytes / filesystem paths / binary streams
  -> PyO3 extension: detach core operation from the GIL
  -> kordoc-core: validate, detect, dispatch, contain panics, project IR/result
  -> format adapter: return source-neutral ParsedDocument (future)
  -> kordoc-ir wire DTOs: serialize the result envelope
  -> Python facade result or typed error
```

## Security boundaries

The shared maximum input size is 524,288,000 bytes. The core checks ZIP
central-directory structure, single-disk metadata, archive entry count,
declared uncompressed size, and local data extents before opening the archive
library. These checks bound detector work; they do not establish that the
document can be parsed.

The Python facade enforces the same input-size limit for in-memory inputs and
bounded reads from paths and binary streams. Path handling opens files in
binary mode. Text streams and reads that exceed the bound are rejected before
the native call.

Parser callbacks run behind an unwind boundary. Panic payloads and configured
passwords do not cross public errors, and the detected file type is retained.
Options are converted while the GIL is held; only owned bytes and options enter
detached native work.

P7 projections accept untrusted recursive IR. Python and native input preflight
enforce a 256 MiB JSON budget and 256 raw nesting levels before the core's 64
logical block/table levels; native result serialization uses a capped writer.
The core caps table columns/cells at 200/2,000,000, projected page spans and
chunk counts at 100,000 each, raw HTML table units at 8 MiB, and projected
Markdown/chunk UTF-8 output at 256 MiB. Excess work returns a typed error
without partial truncation. Table classification is opt-in, independent from
default Markdown. Legacy layout flattening is HWP3/HWP5-only by internal
parser policy; HWPX and generic IR preserve table topology. The single
`kordoc_core::markdown_units` reader is owned by core for future P8/P10/P12
consumers. See [normalization](../components/normalization.md).

The separate `fuzz/` directory is a standalone `fuzz/` workspace. Its
`detect_format`, `zip_preflight`, `markdown_units`, and `projections` targets
exercise bounded safety properties; the detector/container fuzz-only core API
is behind a non-default feature, while the projection targets use the public
source-neutral APIs. The repository's CI workflows define formatting, lint,
tests, package, security, and fuzz checks.
Hosted workflow results and required-check enforcement are tracked separately
in the [migration status](../migration/status.md).

## Migration oracle boundary

The ignored `kordoc/` oracle is for local migration research only. It is not a
runtime, build, package, or CI dependency, and its files are not part of the
repository deliverables. Production code and tests must work when that local
directory is absent.
