# PDF.js text probe fixture

`one_page_helvetica.pdf` is an authored, deterministic fixture dedicated to the private PDF.js runtime probe. It contains one page and one standard Type 1 Helvetica text run: `V8 PDF.js probe`.

The fixture and its generator are dedicated to the public domain under [CC0 1.0](https://creativecommons.org/publicdomain/zero/1.0/). No migration-oracle data was used.

Regenerate it with Python 3 and only the standard library:

```sh
python3 crates/kordoc-pdf/tests/fixtures/pdfjs_probe/generate_fixture.py
```

The generator fixes object order and content, computes stream length and xref byte offsets, and writes no timestamps or host-dependent metadata. Expected SHA-256: `b290aaa8fa5b388a5f3893bba6fc9e8d73f5350af681d95269b986f9dcdef451`.
