# Unicode PDF.js probe fixture

`unicode_probe.pdf` is an original, deterministic one-page PDF fixture dedicated
to testing PDF.js Unicode text extraction. It contains a Type 0 font using
`/Identity-H`, a CIDFontType2 descendant, and an embedded `/ToUnicode` CMap. The
content stream maps CIDs 1, 2, and 3 to `한`, `글`, and the astral character
`🧪` (U+1F9EA). Expected extracted page text is exactly `한글🧪`.

The fixture and recipe are released under CC0 1.0 Universal. No external fonts,
network, Node.js, or migration-oracle files are used. The PDF has no embedded
font program: this probe verifies `/ToUnicode` text mapping for one small
synthetic document, not general font rendering, CMap coverage, or full PDF
Unicode parity.

Rebuild using only the Python standard library:

```sh
python3 make_unicode_probe.py
```

The Rust integration test independently reconstructs the same bytes and
compares them with the checked-in fixture, so normal tests do not invoke
Python. SHA-256 values:

- PDF: `e2dbd29cee44ce50c03fab2385141252e566fe80e59c72e2f2187b53affc2f86`
- Python recipe: `63576c817a8fba1f68d8664f0cba1a9d958e21c8a5062142a408c89e3b9214ac`
