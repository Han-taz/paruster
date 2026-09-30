# HWPML nested-table collision captures

These three synthetic XML inputs are dedicated to the public domain under
[CC0 1.0](https://creativecommons.org/publicdomain/zero/1.0/). They were
authored for this investigation and are not copied from the local oracle.
`generate.py` is the deterministic source of their exact bytes:

```sh
python3 tests/golden/document/hwpml-collision/generate.py --write
python3 tests/golden/document/hwpml-collision/generate.py
```

The no-argument verification checks exact bytes, SHA-256 and fixture names. The
generator SHA-256 is
`502caa7d3dab075c05582b3ce1913522e505d0aafd1ac819080fa703d83c1be8`.

| Fixture | Bytes | SHA-256 | Observation target |
| --- | ---: | --- | --- |
| `fixtures/collision_unmatched.xml` | 511 | `27279b6d327574607fcf060f7c4ff929bbc9e4bf33df382336eeacaf1d32d230` | No row-major text/span target remains after nonempty same-coordinate collision. |
| `fixtures/collision_repeated_text_decoy.xml` | 743 | `92f7e789b361091abed261e14ef683a9865b249f79d208d46e23de6af6f77b96` | A duplicate flat text/span cell exercises row-major fallback; an anchored empty final column observes the option. |
| `fixtures/collision_blank_cell.xml` | 503 | `5a120b4cb1aa5517c68c7ecc6dc0689bb294ab4eee2c12ef3a830c40e6c8d92b` | Whitespace-only same-coordinate collision leaves owner text unchanged. |

## Legacy public parse captures

`oracle-results.jsonl` contains the complete unnormalized result of the public
legacy `parse(input, options)` for each exact case/options pair, plus the input
byte count and SHA-256. Its SHA-256 is
`0387de2f63a23795a10dee84feb337f9b3f4f377d89a0edf6256415c77f1564d`.

The pinned results show:

- `collision_unmatched_default` succeeds with one table cell whose flat text is
  `owner\ninner\ncollision`; its `blocks` field is absent. The nested structure
  was therefore omitted by the source's best-effort attachment path.
- `collision_decoy_default` succeeds with the collided source cell at `(0,0)`
  holding flat text `owner\ninner\ncollision`, while `(0,1)`—the unrelated
  duplicate-text cell—receives paragraph `owner` plus the nested 1×1 table
  containing `inner`.
- Explicit false and omission produce the same two-column decoy result. True
  retains the anchored empty third column; the misattachment remains at
  `(0,1)`.
- `collision_blank_cell_default` keeps paragraph `owner` and the nested table
  attached to the original `(0,0)` cell because the blank collision does not
  change its text.

These are observations of this exact oracle revision and input, not a Rust
parity claim or a recommendation to reproduce the wrong-cell fallback. The
current private Rust slice intentionally fails closed when it cannot attach
nested blocks to an exact unchanged source anchor. Any future policy change
requires coordinator review. No protected H0 fixture, result or score is
modified by this capture-only addition.

## Provenance

The read-only local oracle is pinned to commit
`bb71f7fb0bf51dd456d27505a8c04772df182144` (`kordoc` 4.16.3). Relevant source
SHA-256 values from a disposable archive of that commit:

| Oracle source | SHA-256 |
| --- | --- |
| `src/hwpml/parser.ts` | `e7a6dc1e2c2d6c0656c68044175259b29ac69b53a002745f221b6d5e3a2d8412` |
| `src/table/builder.ts` | `062aa444cef210dfeb674d52a8de01158612ca89cfbd747c8077de63941da021` |
| `src/shared/xml.ts` | `f77f52f609cb9125cf8c6fd5a341959e5688ed8fe6656ae40f40963a62de2eec` |
| `src/utils.ts` | `b5e59e707826cd7aad771de63a09d1d397f0c58a5f7eebb9abd2c0cd534d54ca` |

The one-time local capture used Node `v22.22.0`, npm `10.9.4`, `tsx` `4.23.13`,
`@xmldom/xmldom` `0.9.12`, and `esbuild` `0.28.2`. It ran against a disposable
archive under `/tmp`, never the source checkout. The one-time helper was kept
only under ignored `build/oracle-capture/hwpml-collision-capture.mjs`; its
SHA-256 is
`996f3bfd3af53f88c597cdd4b8988ba1604c3b4acfc604adcd1f9b530da89d27`. It is
not tracked, packaged, or used by runtime/tests. No oracle source or JS
dependencies are included in this directory.

The capture command shape was:

```sh
node --import /path/to/disposable-oracle/node_modules/tsx/dist/loader.mjs \
  build/oracle-capture/hwpml-collision-capture.mjs \
  --oracle-root /path/to/disposable-oracle \
  --fixture-root tests/golden/document/hwpml-collision/fixtures \
  --output tests/golden/document/hwpml-collision/oracle-results.jsonl
```

The helper requires an explicit oracle root and output path. Ordinary Rust and
Python tests must only verify fixture bytes and pinned JSONL; they must not run
or import the oracle helper.
