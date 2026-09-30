# PDF.js text-document probe fixture

`document.pdf` is an authored two-page PDF, dedicated to this repository and
released under CC0 1.0 Universal. It contains reversed stream-vs-horizontal
text order on page one, a nonzero CropBox origin and 90-degree rotation on page
two, a small ToUnicode CMap with Hangul and an astral character, and selected
Info dictionary fields for raw metadata projection tests.

Regenerate with Python's standard library only:

```bash
python3 crates/kordoc-pdf/tests/fixtures/pdfjs_text_document/generate.py \
  --output crates/kordoc-pdf/tests/fixtures/pdfjs_text_document/document.pdf
```

The generated file is 2,256 bytes with SHA-256
`6420221a8fbbfe4386caa18705fe79b3f450ec5893852297461fa793522c4a6b`.
`expected.json` freezes the complete output of the pinned PDF.js 4.10.38
default text stream and fixed Info projection, including normalization of a
ToUnicode U+FB03 ligature to `ffi`; an in-process comparison checks that
`disableNormalization:true` differs. It is 1,764 bytes with SHA-256
`24da296d2ec1b231d55c3171530c79bcb9a5847283428ed8dee469670f331f7e`.
The fixture is an original authored input and the expected result was produced
by the in-process pinned runtime test, not captured from the migration oracle.
No Node, font download, network, or external PDF library is required to
reproduce the PDF.
