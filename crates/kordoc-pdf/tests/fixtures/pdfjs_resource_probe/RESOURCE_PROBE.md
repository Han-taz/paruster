# PDF.js resource-factory probe

This deterministic one-page PDF is an authored feature probe. Its Type0 font
uses the built-in `UniKS-UCS2-H` encoding with an authored ToUnicode map for
`한글`; its unembedded Type1 Helvetica text requires PDF.js standard font data.
The probe exists to prove the private CMap and standard-font host callbacks
return embedded bytes. Text extraction alone does not establish that both
resources were loaded, so its integration test checks callback counts too.

The complete probe is generated with Python's standard library only:

```sh
python3 generate_resource_probe.py
```

The generated `resource_probe.pdf` is 1,644 bytes with SHA-256
`62601a0563488196397e88773eadfd7e2259c56fa33777a797ce958045a72679`.

The PDF and recipe are dedicated to the public domain under CC0 1.0. The
PDF.js CMap and standard font payloads used by the runtime are separate
upstream assets and retain their upstream notices.
