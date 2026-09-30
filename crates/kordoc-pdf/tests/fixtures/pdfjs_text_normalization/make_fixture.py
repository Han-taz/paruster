#!/usr/bin/env python3
"""Generate the original CC0 text-normalization capture PDF."""

from __future__ import annotations

import argparse
from pathlib import Path


def stream(data: bytes) -> bytes:
    return (
        b"<< /Length "
        + str(len(data)).encode()
        + b" >>\nstream\n"
        + data
        + b"\nendstream"
    )


def build_pdf() -> bytes:
    content = b"""BT
/F1 10 Tf
1 0 0 1 50.5 100.5 Tm
(\240Alpha\240) Tj
1 0 0 1 70 90 Tm
( ) Tj
/F3 12 Tf
1 0 0 1 10 30 Tm
(bottom) Tj
/F2 10 Tf
1 0 0 1 50.5 100.5 Tm
(tie) Tj
/F1 12 Tf
0 12 -12 0 150 120 Tm
(Vert) Tj
/F1 0.4 Tf
1 0 0 1 180 60 Tm
(hidden) Tj
/F1 10 Tf
0 Tz
1 0 0 1 20 80 Tm
(zero-width) Tj
100 Tz
ET"""
    objects = [
        b"<< /Type /Catalog /Pages 2 0 R >>",
        b"<< /Type /Pages /Kids [3 0 R] /Count 1 >>",
        (
            b"<< /Type /Page /Parent 2 0 R /MediaBox [0 0 300 400] "
            b"/Resources << /Font << /F1 4 0 R /F2 5 0 R /F3 6 0 R >> >> /Contents 7 0 R >>"
        ),
        b"<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica /Encoding /WinAnsiEncoding >>",
        b"<< /Type /Font /Subtype /Type1 /BaseFont /Courier /Encoding /WinAnsiEncoding >>",
        b"<< /Type /Font /Subtype /Type1 /BaseFont /Times-Roman /Encoding /WinAnsiEncoding >>",
        stream(content),
    ]
    pdf = bytearray(b"%PDF-1.7\n%\xe2\xe3\xcf\xd3\x00\n")
    offsets = [0]
    for number, body in enumerate(objects, start=1):
        offsets.append(len(pdf))
        pdf.extend(f"{number} 0 obj\n".encode())
        pdf.extend(body + b"\nendobj\n")
    xref = len(pdf)
    pdf.extend(f"xref\n0 {len(offsets)}\n".encode())
    pdf.extend(b"0000000000 65535 f\r\n")
    for offset in offsets[1:]:
        pdf.extend(f"{offset:010} 00000 n\r\n".encode())
    pdf.extend(
        f"trailer\n<< /Size {len(offsets)} /Root 1 0 R >>\n"
        f"startxref\n{xref}\n%%EOF\n".encode()
    )
    return bytes(pdf)


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("--write", action="store_true")
    args = parser.parse_args()
    directory = Path(__file__).parent
    path = directory / "base_items.pdf"
    expected = build_pdf()
    if args.write:
        path.write_bytes(expected)
    elif not path.is_file() or path.read_bytes() != expected:
        raise SystemExit("base_items.pdf mismatch; use --write to regenerate")
    if {item.name for item in directory.glob("*.pdf")} != {path.name}:
        raise SystemExit("unexpected or missing PDF fixture")


if __name__ == "__main__":
    main()
