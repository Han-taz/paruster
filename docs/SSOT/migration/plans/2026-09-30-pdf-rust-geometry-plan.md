# Private PDF fractional CropBox geometry

Status: Preparation only; no Rust geometry helper or integration test has been
implemented. Root review of this plan and the authored captures is required
before the RED/GREEN implementation begins.

## Goal and boundary

Project the already bounded private PDF.js page/text DTO into private Rust page
frames and base text positions, preserving the local PDF oracle's unrotated
user-space coordinates. This is a geometry primitive only. It does not register
a parser, change the worker, add an IR/public page type, or implement full text
layout parity.

The source evidence is the ignored local oracle, read only:

- `kordoc/src/pdf/parser.ts:189-194`: calls default `page.getTextContent()`;
  interprets `page.view` as `[x1,y1,x2,y2]`, including the CropBox origin; and
  computes unrotated width and height as `x2-x1` and `y2-y1`.
- `kordoc/src/pdf/text-line.ts:166-181`: rounds the raw text transform's `e`
  and `f` values with JavaScript `Math.round` before any CropBox translation.
- `kordoc/src/pdf/parser.ts:213-240`: filters against the original page
  coordinates and, only after item normalization and annotation matching,
  subtracts `x1,y1` from item positions.
- `kordoc/src/pdf/parser.ts:241-244`: applies the matching translation matrix to
  the operator list when the CropBox origin is nonzero.

Therefore, for the first horizontal-text geometry slice:

```text
page_width  = x2 - x1
page_height = y2 - y1
text_x      = JS_Math_round(transform[4]) - x1
text_y      = JS_Math_round(transform[5]) - y1
```

Do not round after subtracting the CropBox origin. `/Rotate 90` remains a
separate page field; the frame stays `page_width=100`, `page_height=200` for
the fixture below. The oracle uses page rotation for later OCR/vector branch
decisions, but does not use it to rotate or swap these coordinates. Text-item
orientation is separate: later `normalizeItems` stages infer vertical text from
the item transform and can adjust its horizontal position. This first slice
does not claim that broader behavior.

JavaScript rounding must be reproduced exactly. In particular, ties round
toward positive infinity, values in `[-0.5,0)` become negative zero, and
`floor(x + 0.5)` is incorrect for `0.49999999999999994`: the addition rounds to
`1` before `floor`, while JavaScript `Math.round` returns `0`. The pinned Rusty
V8 capture in `tests/fixtures/pdfjs_geometry/rounding-vectors.json` records the
input/output IEEE-754 bit patterns and an explicit `Object.is(result, -0)`
flag. Any proposed Rust rounding helper must pass those vectors before it is
used for coordinates.

## Authored fixture and frozen evidence

The new dedicated CC0 fixture is
`crates/kordoc-pdf/tests/fixtures/pdfjs_geometry/fractional_cropbox.pdf`.
Its stdlib-only `generate.py` defaults to exact-byte verification and uses
`--write` only for deliberate regeneration. The PDF has a one-page MediaBox
`[0,0,200,300]`, CropBox `[10.25,20.75,110.25,220.75]`, `/Rotate 90`, and one
Helvetica text item at transform translation `(11.5,21.5)`. Thus JavaScript
rounds the translation to `(12,22)`, then the CropBox shift yields `(1.75,1.25)`.
The 0.25/0.75 fractional origins make round-before-shift differ from
shift-before-round; the 90-degree page rotation catches accidental axis
rotation or dimension swapping.

Pinned artifact details (all values must be preserved if this preparation is
accepted):

- PDF: 749 bytes, SHA-256
  `d3d9d26a445c99234439c112a5822d8bc34f2b6e0b88ac9c78a7520ce14b253e`.
- Recipe: 2,249 bytes, SHA-256
  `2640fec5a5966f6f3edcf6ddfa722a6713da4bdf62dcb035014e260a0feaee45`.
- Actual kind-3/4 pinned V8 worker response: 413 bytes, SHA-256
  `459107dc4d729bae5504ca298455a5e9207016b137562700b8406afbc9629aa9`.
  It records view box `[10.25,20.75,110.25,220.75]`, rotation `90`, and the
  extracted item's transform `[10,0,0,10,11.5,21.5]`.
- Oracle public-output projection: 549 bytes, SHA-256
  `579336a388156c85af42b9762fb5cd6c39f8275404b35ef0bfc7491b6746f883`.
  It records the paragraph bbox `(x=1.75,y=1.25,width=47,height=10)` and
  Markdown `positive tie`.
- Rusty V8 edge-vector capture: 2,597 bytes, SHA-256
  `ea9db6d3d549b4e3d5aab317fe46b2cb4b03e529b9b6721c37339b73fdc1dea7`.

The raw DTO response came from a fresh build at the PDF text-document candidate
commit `7cebe6543c07dedae6d0b88e495bda8863cd10a4`, using the pinned
`v8 = 152.2.0` crate and PDF.js 4.10.38. The engine reports
`15.2.124.1-rusty`. The research-only Python framing script used SHA-256
`373e398cc8b4aa0d97c9b4d2c906d7b708b3ddf5940d98f0106ad84092c62cba`; it sent
one KPDF v1 kind-3 request to the built worker and saved the kind-4 JSON
payload. The oracle public projection was captured from local kordoc 4.16.3 at
`bb71f7fb0bf51dd456d27505a8c04772df182144`, `parser.ts` SHA-256
`2eb7018d17bf9bb3d39b7d2cb21145fe812a5257a239331aa13d44da9776bf70`, with
PDF.js 4.10.38; the research-only Node/tsx capture script SHA-256 is
`1c03304a95a60db7940f43dfecec03d56d2b2f0b0d3f7fe5a36690e2d981a2fe`.

The oracle JSON is only a focused projection of `parsePdfDocument` fields
`metadata`, `markdown`, `blocks`, and `pages`; it is not the full
`InternalParseResult` and is not a corpus-wide parity claim. The raw DTO capture
is from PDF.js in the native V8 worker, not from the migration oracle. The
negative half-tie cases are separate synthetic JavaScript inputs in the pinned
V8 vector capture because the fixture tests CropBox coordinates, not the
PDF.js behavior for out-of-CropBox text.

No oracle source, Node runtime, or runtime oracle import is included. Keep the
new fixture and capture bytes outside the older PDF.js and Unicode fixture
directories. The repository's effective Git attributes must preserve the PDF
and the pinned JSON bytes on Windows checkouts; coordinate that attribute edit
with its owner rather than changing shared configuration in this checkpoint.

## Proposed RED/GREEN implementation after review

Proposed source file ownership is limited to a new private
`crates/kordoc-pdf/src/geometry.rs`, a narrow crate-module declaration owned
with the coordinator, and a new focused `crates/kordoc-pdf/tests/pdfjs_geometry.rs`.
The V8 DTO, worker framing, supervisor, public IR, and `kordoc-ir` remain
unchanged.

The private API can be kept small:

```rust
pub(crate) struct PdfPageFrame {
    pub(crate) page_number: u32,
    pub(crate) origin_x: f64,
    pub(crate) origin_y: f64,
    pub(crate) width: f64,
    pub(crate) height: f64,
    pub(crate) rotation: i32,
}

pub(crate) struct PdfTextPosition {
    pub(crate) x: f64,
    pub(crate) y: f64,
}

pub(crate) fn page_frame(page: &PdfJsPage) -> Result<PdfPageFrame, KordocError>;
pub(crate) fn base_text_position(
    frame: &PdfPageFrame,
    item: &PdfJsTextItem,
) -> Result<PdfTextPosition, KordocError>;
```

Keep the coordinate projection intentionally small: preserve raw item order,
text, dimensions and transform; project only base `(x,y)` for the authored
horizontal item. Do not yet sort or normalize text, apply vertical-text
position corrections, filter hidden/outside text, resolve annotations, shift
operator lists, build `BoundingBox`/`IrBlock`, or call a parser registry.

Tests to add before implementation:

1. Reconstruct the authored PDF from its recipe in a clean temporary directory
   and compare exact bytes/hash; assert the captured worker DTO and oracle
   public-output captures have their pinned hashes and expected fields.
2. Exercise the actual kind-3 V8 extraction of the authored PDF and verify the
   raw page view, rotation, item transform and item order against the captured
   DTO.
3. With a wished-for page-frame/position helper, assert positive dimensions,
   retained rotation, and `(1.75,1.25)`. This test must fail before the helper
   exists and pass after its minimal implementation.
4. Feed synthetic finite DTO positions covering every captured V8 rounding
   vector, including negative zero and `0.49999999999999994`; compare rounded
   bit patterns/signs before translation. Add a fractional origin case that
   proves the rounding precedes subtraction.
5. Reject nonfinite or non-positive view boxes, nonfinite width/height
   differences, and nonfinite derived coordinates as typed `ParseError`.

## Bounds and error behavior

The incoming DTO is already limited to 200 pages, 100,000 items, 64 KiB per
text item, 2 MiB aggregate text, and 4 MiB serialized response. Its existing
decoder validates page-view and transform values are finite and rotation is a
multiple of 90. The private frame constructor must additionally require
`x2 > x1`, `y2 > y1`, finite positive dimensions, and finite translated
coordinates. Malformed geometry is `ParseError`; do not clamp coordinates or
turn failure into absent geometry. This primitive performs no input-sized
allocation, so it needs no new output/vector budget. Do not invent a page-size
magnitude limit without separate compatibility evidence and coordinator
approval; check arithmetic overflow directly.

## Explicit exclusions

No IR/schema/API/error inventory change; page layout and block formation;
full `normalizeItems` parity; font and width-based layout; hidden-text policy;
operator-list or link-annotation geometry; OCR or graphics; source ordering and
deduplication policy; page-range/parser registration; changes to existing
fixtures or their expected outputs; dependency or limit changes; packaging or
wheel claims. Those remain later independently approved checkpoints.
