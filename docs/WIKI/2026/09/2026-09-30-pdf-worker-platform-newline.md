---
id: 2026-09-30-pdf-worker-platform-newline
date: 2026-09-30
status: locally-verified
component: pdf
issue: null
pr: https://github.com/Han-taz/paruster/pull/27
commit: null
related_ssot: [docs/SSOT/components/pdf.md]
related_decisions: []
---

# Exact native line endings in the UTF-8 helper regression

The [third Windows helper run](https://github.com/Han-taz/paruster/actions/runs/36713134839)
confirms the new isolated-mode output contains the exact UTF-8 Hangul/astral
bytes. The regression fails because Windows `print` terminates with CRLF and
the expected bytes terminate with LF. The test now expects `os.linesep`
explicitly while comparing every byte of the output. No stripping, decoding
replacement or broad normalization is added; the PDF result comparison remains
unchanged. This follows the [UTF-8 correction](2026-09-30-pdf-worker-windows-utf8.md).

Nineteen helper tests and pinned Ruff check/format pass locally. The hosted
six-target worker matrix and existing protected gates must rerun on this head.
