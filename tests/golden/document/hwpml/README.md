# HWPML H0 fixtures and oracle captures

These eight small XML inputs are authored test material, dedicated to the public
domain under [CC0 1.0](https://creativecommons.org/publicdomain/zero/1.0/).
They contain no copied oracle fixture or implementation code.
`crates/kordoc-hancom/tests/support/hwpml/generate.py` is the deterministic
source of their exact bytes. From the repository root, verify them offline with
`python3 crates/kordoc-hancom/tests/support/hwpml/generate.py`; regenerate only
with `python3 crates/kordoc-hancom/tests/support/hwpml/generate.py --write`.
The default command fails on changed bytes, missing inputs, or extra XML files.

| Fixture | Bytes | SHA-256 | Focus |
| --- | ---: | --- | --- |
| `fixtures/normal_metadata_styles.xml` | 1467 | `7c4bde519ae9ffbf7e65b2330f82e6838fc6d430c55c7cb84a2d84a12188f715` | UTF-8 BOM, default and prefixed namespace, metadata, heading levels, multiple sections, CHAR concatenation, `&nbsp;`, ignored header/footer/autonumber/picture/shape text, inline footnote text |
| `fixtures/empty_body.xml` | 133 | `b19c14c589cfecae3f7adfc1723a8286f6a6488775e3a297ba475349b408f92f` | Empty body with metadata |
| `fixtures/empty_sections.xml` | 146 | `c65386caf4be41ee33859e631c71ba67be0c664ded192a6f8f79b87842c97151` | Empty sections and ignored header content |
| `fixtures/nested_table.xml` | 949 | `3255695c5b3a0a0cd677687d9e27fa9248f30118a7ebf0be6afc19287e86c308` | Nested table, row/column spans, empty third column, and trailing-column option |
| `fixtures/malformed_partial.xml` | 223 | `326aa488832d9979de62ac896db29d8b958763803375dca597ffe99b8a3449ea` | Recoverable undefined entity after ordinary document text |
| `fixtures/malformed_unclosed.xml` | 178 | `907d2491125acd4076ec954559d77f35832387237f3ae9638836448b4e4deb42` | Unclosed tags at EOF |
| `fixtures/dtd_external_entity.xml` | 221 | `6055036457910c3d11acd3b6f142c61fd4c34ae03a20a152bf833e5791017a92` | DTD plus escaped `&amp;local;` text; literal-text control only |
| `fixtures/dtd_external_entity_reference.xml` | 259 | `4463607f952b8a62b79fd498d03ea49e8f4685975e96cb040ee84ba4fd8134e8` | DTD declares an entity at a harmless nonexistent file URI and references it as `&probe;` |

## Legacy result capture

`crates/kordoc-hancom/tests/support/hwpml/hwpml-oracle-results.jsonl` stores
the complete JSON result returned by the legacy public `parse(input, options)`
for every case. Each record includes exact options, input byte count and SHA-256.
Twelve fixture captures cover default parsing, page selection, both trailing
empty-column settings, malformed XML, and the entity-reference observation.
Together, the capture has 13 records. One generated in-memory case is
52,428,801 bytes consisting of the ASCII prefix `<?xml version="1.0"?><HWPML/>` followed
by U+0020 bytes. No oversized input is checked in. Oracle returns
`DECOMPRESSION_BOMB` for that input.

The one-time capture used the ignored local migration oracle at commit
`bb71f7fb0bf51dd456d27505a8c04772df182144`. Oracle source identities:

| Source | SHA-256 |
| --- | --- |
| `src/hwpml/parser.ts` | `e7a6dc1e2c2d6c0656c68044175259b29ac69b53a002745f221b6d5e3a2d8412` |
| `src/shared/xml.ts` | `f77f52f609cb9125cf8c6fd5a341959e5688ed8fe6656ae40f40963a62de2eec` |
| `src/table/builder.ts` | `062aa444cef210dfeb674d52a8de01158612ca89cfbd747c8077de63941da021` |
| `src/utils.ts` | `b5e59e707826cd7aad771de63a09d1d397f0c58a5f7eebb9abd2c0cd534d54ca` |

The capture was run against a disposable copy of the read-only oracle source
using Node `v22.22.0`, npm `10.9.4`, oracle package `4.16.3`, `tsx` `4.23.13`,
and `@xmldom/xmldom` `0.9.12`. The helper was kept only under ignored
`build/oracle-capture/hwpml-capture.mjs`; it is not tracked or used by tests.
The one-time command shape from the repository root was:

```sh
node --import tsx build/oracle-capture/hwpml-capture.mjs \
  --oracle-root /path/to/disposable-oracle-copy \
  --fixture-root tests/golden/document/hwpml/fixtures \
  --output crates/kordoc-hancom/tests/support/hwpml/hwpml-oracle-results.jsonl
```

The disposable copy contained only oracle `src/`, `package.json`, and
`package-lock.json`; setup used `npm ci --omit=optional --ignore-scripts
--no-audit --no-fund`, then installed the host's esbuild binary with
`npm install --no-save --ignore-scripts --no-audit --no-fund
@esbuild/darwin-arm64@0.28.2`. No dependency or oracle source was copied into
this repository.

The local capture helper required an explicit oracle path and was never invoked
by Rust tests, Python packaging, runtime code, or CI. Oracle dependencies are
not included in this repository. The final JSONL SHA-256 is
`aba0dd587753d2c898340be9fe10e4dfd79a535c9f1356c4c83f0ede250551c6`; the
generator SHA-256 is
`13a40938e58cacac0f3194a3813ecac187450afc1a68b2c9e36b652115e60cfe`; the
ignored local helper SHA-256 is
`9e6c4d41f9717738308905b7a280a3a3a4940440bebf8bd2f54428a220b6972e`.

The semantic captures show metadata and headings, section-based pages, nested
and merged tables, preservation/removal of the trailing empty column according
to the option, empty-body success, and partial output with a `MALFORMED_XML`
warning. The escaped DTD fixture is only a literal-text control. For the
separate actual entity reference, the oracle returns literal `&probe;` plus an
`entity not found` `MALFORMED_XML` warning. These observations do not establish
a general external-entity security guarantee. The unclosed XML fixture returns
a failed public `PARSE_ERROR`; it is separate from recoverable malformed
content. The in-memory size case returns a stable size-limit error. These are
H0 inputs and captures only; they do not claim Rust parity, parser completion,
or promotion into protected scoring. No protected manifest or scoring entry
is changed here.
