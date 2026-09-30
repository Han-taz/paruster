#!/usr/bin/env python3
"""Generate a deterministic CC0 PDF for CropBox-coordinate probes."""

from __future__ import annotations

import argparse
from pathlib import Path


def stream(data: bytes) -> bytes:
    return (
        b"<< /Length "
        + str(len(data)).encode("ascii")
        + b" >>\nstream\n"
        + data
        + b"\nendstream"
    )


def build_pdf() -> bytes:
    content = b"BT\n/F1 10 Tf\n1 0 0 1 11.5 21.5 Tm\n(positive tie) Tj\nET"
    objects = [
        b"<< /Type /Catalog /Pages 2 0 R >>",
        b"<< /Type /Pages /Kids [3 0 R] /Count 1 >>",
        (
            b"<< /Type /Page /Parent 2 0 R /MediaBox [0 0 200 300] "
            b"/CropBox [10.25 20.75 110.25 220.75] /Rotate 90 "
            b"/Resources << /Font << /F1 4 0 R >> >> /Contents 5 0 R >>"
        ),
        b"<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica >>",
        stream(content),
        b"<< /Title (Fractional CropBox geometry probe) >>",
    ]
    pdf = bytearray(b"%PDF-1.7\n%\xe2\xe3\xcf\xd3\x00\n")
    offsets = [0]
    for number, body in enumerate(objects, start=1):
        offsets.append(len(pdf))
        pdf.extend(f"{number} 0 obj\n".encode("ascii"))
        pdf.extend(body + b"\nendobj\n")
    xref = len(pdf)
    pdf.extend(f"xref\n0 {len(offsets)}\n".encode("ascii"))
    pdf.extend(b"0000000000 65535 f\r\n")
    for offset in offsets[1:]:
        pdf.extend(f"{offset:010} 00000 n\r\n".encode("ascii"))
    pdf.extend(
        f"trailer\n<< /Size {len(offsets)} /Root 1 0 R /Info 6 0 R >>\n"
        f"startxref\n{xref}\n%%EOF\n".encode("ascii")
    )
    return bytes(pdf)


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("--write", action="store_true")
    args = parser.parse_args()
    directory = Path(__file__).parent
    path = directory / "fractional_cropbox.pdf"
    expected = build_pdf()
    if args.write:
        path.write_bytes(expected)
    elif not path.is_file() or path.read_bytes() != expected:
        raise SystemExit("fractional_cropbox.pdf does not match generator; use --write")
    if {item.name for item in directory.glob("*.pdf")} != {path.name}:
        raise SystemExit("unexpected or missing PDF fixture")


if __name__ == "__main__":
    main()
