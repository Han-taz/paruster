# Pinned PDF worker notice bundle

[`PROVENANCE.json`](PROVENANCE.json) maps 33 pinned upstream notice files to
their source SHA-256 and experimental worker-wheel paths. Thirty-two notice
files are stored here; the existing PDF.js Apache notice is reused from
[`../pdfjs/LICENSE`](../pdfjs/LICENSE). Total staged notice bytes: 221,177.
No oracle source or native executable is stored in this directory.

The bundle covers Rusty V8 152.2.0, its pinned V8 source tree and identified
native dependencies, plus PDF.js 4.10.38. Conservative V8-tree coverage includes
some build/tool notices; this is not a claim that every listed component is
linked, nor a blanket audit of every Rust dependency.

The package staging/audit tool requires the fixed 33-path inventory and exact
hashes, rejects missing or duplicate archive entries, and verifies installed
notices before invoking the worker. Notices are copied only in the separate
worker-wheel workflow. Fresh default Python wheels remain unchanged.

PDF.js provenance is the original published npm tarball's `package/LICENSE`;
its SHA-512 and original selected-file hashes remain recorded in
[`../pdfjs/PROVENANCE.json`](../pdfjs/PROVENANCE.json). Other source URLs identify pinned upstream releases or commits. The coordinator independently fetched
and verified the 33 sources before publication.
