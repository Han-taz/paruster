# Fractional CropBox coordinate probe

`fractional_cropbox.pdf` is an original, deterministic one-page PDF authored
for the private Rust geometry checkpoint and dedicated to the public domain
under CC0 1.0 Universal (see `LICENSE.txt`). It uses a fractional, nonzero
CropBox origin `[10.25,20.75]`, page `/Rotate 90`, and one Helvetica text item
whose transform translation is `(11.5,21.5)`. The oracle's block bbox is
`(x=1.75,y=1.25,width=47,height=10)`: it rounds text coordinates first and
then subtracts the CropBox origin; rotation does not swap page dimensions.

Regenerate the declared PDF deliberately with `--write`:

```sh
python3 crates/kordoc-pdf/tests/fixtures/pdfjs_geometry/generate.py --write
```

By default, the recipe verifies exact bytes and the complete PDF inventory.
It uses only Python's standard library. The captures below are fixed research
evidence; tests must not execute the local oracle or the capture scripts.

## Captures

`worker-response.json` is the actual KPDF kind-4 response to the fixture's
kind-3 request from the pinned native PDF.js/V8 worker. Its page DTO contains
view box `[10.25,20.75,110.25,220.75]`, rotation `90`, and item transform
`[10,0,0,10,11.5,21.5]`.

`oracle-public-output.json` is a focused projection of the oracle's successful
`parsePdfDocument` result: metadata, Markdown, blocks, and pages. It does not
contain the full `InternalParseResult` and is not evidence of general PDF
parity. The raw DTO capture above is from native PDF.js in V8, not the oracle.

`rounding-vectors.json` records JavaScript `Math.round` results evaluated by
Rusty V8 crate `152.2.0` (runtime version `15.2.124.1-rusty`). Every vector
includes the input and output IEEE-754 bit pattern and an explicit
`Object.is(rounded,-0)` flag. It includes the adjacent float below `0.5`, where
`floor(x + 0.5)` incorrectly yields `1`, and negative ties yielding `-0`.

The local oracle was kordoc 4.16.3 at commit
`bb71f7fb0bf51dd456d27505a8c04772df182144`, PDF.js 4.10.38, with parser source
SHA-256 `2eb7018d17bf9bb3d39b7d2cb21145fe812a5257a239331aa13d44da9776bf70`.
The V8 worker was freshly built at PDF text-document candidate commit
`7cebe6543c07dedae6d0b88e495bda8863cd10a4`, with PDF.js 4.10.38 and
`v8 = 152.2.0`. Research capture scripts are external to the repository and
are not required to regenerate the PDF or run tests.

## SHA-256 pins

- PDF, 749 bytes: `d3d9d26a445c99234439c112a5822d8bc34f2b6e0b88ac9c78a7520ce14b253e`
- `generate.py`, 2,249 bytes: `2640fec5a5966f6f3edcf6ddfa722a6713da4bdf62dcb035014e260a0feaee45`
- worker response, 413 bytes: `459107dc4d729bae5504ca298455a5e9207016b137562700b8406afbc9629aa9`
- oracle public output, 549 bytes: `579336a388156c85af42b9762fb5cd6c39f8275404b35ef0bfc7491b6746f883`
- V8 rounding vectors, 2,597 bytes: `ea9db6d3d549b4e3d5aab317fe46b2cb4b03e529b9b6721c37339b73fdc1dea7`

This directory is new and separate from the older PDF.js and Unicode fixtures;
no existing PDF fixture is modified.
