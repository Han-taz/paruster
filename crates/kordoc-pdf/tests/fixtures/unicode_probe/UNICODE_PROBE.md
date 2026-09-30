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
Python. The classic xref subsection uses 20-byte CRLF records. SHA-256 values:

- PDF: `331f40fcf23b600772b4baa04d7118c537832ffb2f2f9b0c6e7fca685635dc93`
- Python recipe: `0456e099df963a05fb7a2bb9f149a3b3f950c805ca2faf3c797986608cea5582`
