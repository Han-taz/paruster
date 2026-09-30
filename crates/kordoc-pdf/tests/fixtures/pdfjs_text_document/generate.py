#!/usr/bin/env python3
"""Build a deterministic CC0 two-page PDF.js text-document probe."""

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
    cmap = b"""/CIDInit /ProcSet findresource begin
12 dict begin
begincmap
/CIDSystemInfo << /Registry (Adobe) /Ordering (UCS) /Supplement 0 >> def
/CMapName /FixtureUnicode-UCS def
/CMapType 2 def
1 begincodespacerange
<0000> <FFFF>
endcodespacerange
4 beginbfchar
<0001> <D55C>
<0002> <AE00>
<0003> <D83EDDEA>
<0004> <FB03>
endbfchar
endcmap
CMapName currentdict /CMap defineresource pop
end
end"""
    page_one = (
        b"BT\n/F1 12 Tf\n1 0 0 1 200 700 Tm\n(second-stream) Tj\n"
        b"1 0 0 1 50 700 Tm\n(first-stream) Tj\n"
        b"1 0 0 1 50 680 Tm\n(continued line) Tj\nET"
    )
    page_two = b"BT\n/F2 18 Tf\n1 0 0 1 120 450 Tm\n<0001000200030004> Tj\nET"
    objects = [
        b"<< /Type /Catalog /Pages 2 0 R >>",
        b"<< /Type /Pages /Kids [3 0 R 4 0 R] /Count 2 >>",
        b"<< /Type /Page /Parent 2 0 R /MediaBox [0 0 612 792] /Resources << /Font << /F1 5 0 R /F2 6 0 R >> >> /Contents 10 0 R >>",
        b"<< /Type /Page /Parent 2 0 R /MediaBox [0 0 612 792] /CropBox [100 200 500 700] /Rotate 90 /Resources << /Font << /F1 5 0 R /F2 6 0 R >> >> /Contents 11 0 R >>",
        b"<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica >>",
        b"<< /Type /Font /Subtype /Type0 /BaseFont /FixtureUnicode /Encoding /Identity-H /DescendantFonts [7 0 R] /ToUnicode 8 0 R >>",
        b"<< /Type /Font /Subtype /CIDFontType2 /BaseFont /FixtureUnicode /CIDSystemInfo << /Registry (Adobe) /Ordering (Identity) /Supplement 0 >> /FontDescriptor 9 0 R /DW 1000 /CIDToGIDMap /Identity >>",
        stream(cmap),
        b"<< /Type /FontDescriptor /FontName /FixtureUnicode /Flags 4 /FontBBox [0 -200 1000 900] /ItalicAngle 0 /Ascent 800 /Descent -200 /CapHeight 700 /StemV 80 >>",
        stream(page_one),
        stream(page_two),
        b"<< /Title (  Raw title  ) /Author ( Author ) /Creator () /Subject () /Keywords (alpha, beta; gamma ) /CreationDate (D:20250930123456Z) /ModDate 42 >>",
    ]

    pdf = bytearray(b"%PDF-1.7\n%\xe2\xe3\xcf\xd3\x00\n")
    offsets = [0]
    for number, body in enumerate(objects, start=1):
        offsets.append(len(pdf))
        pdf.extend(f"{number} 0 obj\n".encode("ascii"))
        pdf.extend(body)
        pdf.extend(b"\nendobj\n")
    xref = len(pdf)
    pdf.extend(f"xref\n0 {len(offsets)}\n".encode("ascii"))
    pdf.extend(b"0000000000 65535 f\r\n")
    for offset in offsets[1:]:
        pdf.extend(f"{offset:010} 00000 n\r\n".encode("ascii"))
    pdf.extend(
        f"trailer\n<< /Size {len(offsets)} /Root 1 0 R /Info 12 0 R >>\nstartxref\n{xref}\n%%EOF\n".encode(
            "ascii"
        )
    )
    return bytes(pdf)


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("--output", type=Path, required=True)
    args = parser.parse_args()
    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_bytes(build_pdf())


if __name__ == "__main__":
    main()
