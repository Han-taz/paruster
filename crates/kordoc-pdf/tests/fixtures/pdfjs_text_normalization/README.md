# PDF base text-item normalization capture

`base_items.pdf` is an original deterministic one-page CC0 fixture. Its
stdlib-only recipe verifies exact bytes by default and uses `--write` only for
deliberate regeneration:

```sh
python3 make_fixture.py
python3 make_fixture.py --write
```

`raw-worker-response.json` is the exact kind-4 response payload from the
supervised native PDF.js worker after sending a KPDF v1 kind-3 request for this
PDF. The worker binary was freshly built from paruster commit
`e0fde7db6ed97914ed78483fd2b5f0a58a5d99a3`, which includes the pinned PDF.js
4.10.38 assets and `v8 = 152.2.0`; the build used the existing local Cargo
target cache. Research-only capture script SHA-256:
`fde51d4e1ba8f7d8bbc4d214b68ff7cf2b3ad2dc19f9e3f75b43d0bd05089111`.

`oracle-scalar-projection.json` records `normalizeItems` applied to the raw
worker items converted to the oracle's `{str, transform, width, height,
fontName}` shape. `oracle-synthetic-vectors.json` separately records
synthetic raw items for Unicode trimming, a whitespace-only item at sequence 2
between visible sequence 1 and 3, and equal rounded x/y keys whose original
order must remain stable. These synthetic inputs do not claim to be raw
PDF.js output. The capture used local kordoc commit
`bb71f7fb0bf51dd456d27505a8c04772df182144`, with
`src/pdf/text-line.ts` SHA-256
`034b6a882eb913b723e21a1729e032bc45147e60acdf88aab655dfd415253072`, Node
`v22.22.0`, and `node --experimental-strip-types`. The source was copied byte
for byte to a temporary directory (same source hash); its unrelated runtime
`line-detector.js` import was satisfied by a temporary stub because
`normalizeItems` does not call either imported function. No oracle/runtime
source or stub is included in the repository. Capture script SHA-256s:

- Actual worker-derived oracle records:
  `301f77168a5011e1e790064dc3002bfb1c09425ccc106115327131f880eb258e`.
- Synthetic oracle vectors:
  `4e17d338b38b73077f1c5765b4db113e20773c20d62c793b6b8fa340d045d4d8`.
- Temporary no-op import stub:
  `d04df7200c90d872389779bfaf2504fbcc432d4415e3d39509477193d5455bb2`.

`math-hypot-vectors.json` records actual JavaScript `Math.hypot` and
`Math.round` values from Rusty V8 crate `152.2.0` / V8
`15.2.124.1-rusty`, alongside Rust `f64::hypot` values from rustc `1.97.0`
on `aarch64-apple-darwin`. The V8 capture source was
`v8/src/builtins/math.tq`, SHA-256
`60fbee44d8896b6d1e49eb89ab6cc67c2028048b6ef91e6ec297d64e37bb521c`. One
captured vector demonstrates that this platform's Rust `hypot` can change the
rounded font size compared with pinned V8; the comparison is evidence for
root review, not an approved implementation strategy. Research-only capture
program SHA-256:
`e3440b7b6a30f01801d5de5681e14a363a68d12b022bc9db02426257bb352fe7`.

All capture artifacts are scoped to this authored fixture or synthetic
numeric/item inputs. They do not establish full `normalizeItems`, line layout,
or document-parity behavior. Exact byte pins:

- `base_items.pdf`: 1,125 bytes,
  `f4abb4f30af5582ecaca9c54e73b93f57632d84b0fd464a01b1cf4f22c07bf0f`.
- `make_fixture.py`: 2,532 bytes,
  `9d9a77307876b6804ec8ec599f76356aa0348e8b00e12452b95e1f331f9c58ca`.
- `raw-worker-response.json`: 2,352 bytes,
  `a5d5366766cccb1da67754b759a052bad1045de0472b1983805ef1f5c6fbc2e8`.
- `oracle-scalar-projection.json`: 3,394 bytes,
  `0f9ac629705012f5f065c8d135ceb617c826af4259d95d8813f4ee604bc56752`.
- `oracle-synthetic-vectors.json`: 2,387 bytes,
  `dc1697861512c72e28f625b5b5b5f06d1bf093eb4954f3db1145e45fd666e736`.
- `math-hypot-vectors.json`: 1,934 bytes,
  `cca58018caa7e26846b5450fa4bf503f5fc5873b402fbf6699cdd75005bed2a1`.

The fixture is capture-only preparation. No Rust implementation is included.
