#!/usr/bin/env python3
"""Generate the CC0 PDF.js text probe fixture using only the Python standard library."""
import argparse
from pathlib import Path

parser = argparse.ArgumentParser(description=__doc__)
parser.add_argument("output", nargs="?", type=Path, default=Path(__file__).with_name("one_page_helvetica.pdf"))
OUT = parser.parse_args().output

# Objects are intentionally fixed. Offsets are calculated so output bytes are repeatable.
objects = [
    b"<< /Type /Catalog /Pages 2 0 R >>",
    b"<< /Type /Pages /Kids [3 0 R] /Count 1 >>",
    b"<< /Type /Page /Parent 2 0 R /MediaBox [0 0 612 792] /Resources << /Font << /F1 4 0 R >> >> /Contents 5 0 R >>",
    b"<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica >>",
    b"<< /Length 53 >>\nstream\nBT /F1 12 Tf 72 720 Td (V8 PDF.js probe) Tj ET\nendstream".replace(b"/Length 53", b"/Length " + str(len(b"BT /F1 12 Tf 72 720 Td (V8 PDF.js probe) Tj ET\n")).encode()),
]

pdf = bytearray(b"%PDF-1.4\n%\xe2\xe3\xcf\xd3\n")
offsets = [0]
for index, body in enumerate(objects, start=1):
    offsets.append(len(pdf))
    pdf.extend(f"{index} 0 obj\n".encode())
    pdf.extend(body)
    pdf.extend(b"\nendobj\n")
xref_offset = len(pdf)
pdf.extend(f"xref\n0 {len(objects) + 1}\n".encode())
pdf.extend(b"0000000000 65535 f \n")
for offset in offsets[1:]:
    pdf.extend(f"{offset:010d} 00000 n \n".encode())
pdf.extend(f"trailer\n<< /Size {len(objects) + 1} /Root 1 0 R >>\nstartxref\n{xref_offset}\n%%EOF\n".encode())
OUT.write_bytes(pdf)
print(f"wrote {OUT} ({len(pdf)} bytes)")
