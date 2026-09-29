# Workspace architecture

This page records the current foundation implementation. It describes the
working interfaces and boundaries, not completed document-processing parity.

## Components and data flow

The Rust workspace is split into three crates. `kordoc-ir` defines the shared
serializable wire DTOs, including document structures, file types, parse result
envelopes, and error codes. `kordoc-core` owns bounded format detection,
container preflight, and the current parse-dispatch boundary. It does not yet
contain real document parsers. `kordoc-python` is the PyO3 extension that
translates calls and wire results between Rust and Python.

The `python/kordoc` Python facade normalizes supported caller inputs—bytes-like
values, filesystem paths, and binary streams—into bounded bytes before calling
the native extension. The extension releases the GIL while Rust detection or
dispatch runs. Core results are serialized through the shared IR types and
translated into the facade's immutable Python result and typed-error surface.

```text
Python caller
  -> Python facade: normalize bytes / filesystem paths / binary streams
  -> PyO3 extension: detach core operation from the GIL
  -> kordoc-core: detect and dispatch within input/container bounds
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

The separate `fuzz/` directory is a standalone `fuzz/` workspace. Its
`detect_format` and `zip_preflight` targets exercise bounded safety properties;
the fuzz-only core API is behind a non-default feature. The repository's CI
workflows define formatting, lint, tests, package, security, and fuzz checks.
Hosted workflow results and required-check enforcement are tracked separately
in the [migration status](../migration/status.md).

## Migration oracle boundary

The ignored `kordoc/` oracle is for local migration research only. It is not a
runtime, build, package, or CI dependency, and its files are not part of the
repository deliverables. Production code and tests must work when that local
directory is absent.
