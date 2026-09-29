# Synthetic golden fixtures

These fixtures are small, deterministic, and independently generated. The
foundation golden currently covers format detection only; it does not claim
document parsing or full oracle parity.

`fixtures/minimal.pdf` is a one-page blank PDF 1.4 assembled from literal
objects with the Python standard library and no external inputs. Its manifest
entry records the CC0-1.0 dedication, generator, SHA-256, and detection
contract it covers.

Do not copy a government corpus or a file from the ignored local `kordoc/`
oracle into this directory without recording redistribution rights and the
fixture checksum. Tests consume only committed manifest data and fixture
bytes; they never read the oracle.
