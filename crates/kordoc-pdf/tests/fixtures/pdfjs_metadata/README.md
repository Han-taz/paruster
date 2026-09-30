# PDF metadata normalization fixtures

These three deterministic, one-page PDFs are original synthetic inputs authored
for private metadata-parity tests and dedicated to the public domain under CC0
1.0 Universal (see `LICENSE.txt`). `generate.py` uses only Python's standard
library. Verify all checked-in input bytes and the complete PDF inventory with:

```sh
python3 crates/kordoc-pdf/tests/fixtures/pdfjs_metadata/generate.py
```

Regenerate the declared inputs deliberately with `--write`:

```sh
python3 crates/kordoc-pdf/tests/fixtures/pdfjs_metadata/generate.py --write
```

The inputs cover trimmed and empty Info strings, repeated/mixed keyword
delimiters, delimiter-only keywords, partial date matching, ignored timezone
suffixes, invalid calendar components, and missing or non-string Info values.
All inputs successfully pass both oracle entry points.

## Frozen oracle captures

`oracle-captures.jsonl` contains the metadata projection from each successful
full parse and the separate metadata-only result for each input. It is a fixed
offline expected-data artifact, not an executable oracle dependency. The
capture source was the local kordoc 4.16.3 checkout at
`bb71f7fb0bf51dd456d27505a8c04772df182144`; the exact
`src/pdf/parser.ts` SHA-256 was
`2eb7018d17bf9bb3d39b7d2cb21145fe812a5257a239331aa13d44da9776bf70`.
That checkout resolves `pdfjs-dist` 4.10.38. Captures were taken with Node
22.22.0 using a temporary copy of the oracle checkout, `npm ci
--ignore-scripts --no-audit --no-fund`, and `node --import tsx
/tmp/pdf-rust-metadata-capture.mjs`; the research-only capture script had
SHA-256 `b4ae1f432a334cb307cdebf97140f3b7131722047fae9b29ed1cc945d8fe6fa1`.
For full parsing, the call used `{ ocr: false }`; both calls completed
successfully for all three inputs before their metadata was selected. The
temporary copy and script are outside the repository and are not distributed.
No oracle source files are copied into this fixture directory.

The expected behavior includes `Keywords: (; ,;;,)` becoming an empty array,
`D:2025120X` becoming `2025-12-01T00:00:00`, and `D:20251399` becoming
`2025-13-99T00:00:00`. These reflect the oracle's permissive source behavior.

## SHA-256

- `mixed.pdf` (810 bytes): `aa843ac436da42c84b938bd59a544cd11842b97fd5edf7be73180abbbb645145`
- `delimiters.pdf` (676 bytes): `0b729cf3a66503eaefb161d2b41fb64f5dd86d69a0c4a6b85efba09efa7acdea`
- `dates.pdf` (739 bytes): `208287a8ac86bb4191e2c67aa0d4803f7ede12770826047072f0328ec15721e2`
- `generate.py` (2,985 bytes): `f4c19e1cbf63c80059329c7501ee58e51e56f15e902ff9d00217be1a20f3be8d`
- `oracle-captures.jsonl` (2,703 bytes): `d79d30dc6b189bbfcdb4e05fbb8bec54de8c9740298c46ee1034ce7838413145`
