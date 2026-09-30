#!/usr/bin/env python3
"""Rebuild the deterministic resource-factory PDF probe using Python stdlib."""

import hashlib
from pathlib import Path

ROOT = Path(__file__).parent
DESTINATION = ROOT / "resource_probe.pdf"


def stream(data: bytes) -> bytes:
    return (
        b"<< /Length "
        + str(len(data)).encode("ascii")
        + b" >>\nstream\n"
        + data
        + b"\nendstream"
    )


def build_pdf() -> bytes:
    to_unicode = b"""/CIDInit /ProcSet findresource begin
12 dict begin
begincmap
/CIDSystemInfo << /Registry (Adobe) /Ordering (UCS) /Supplement 0 >> def
/CMapName /ResourceProbe-UCS def
/CMapType 2 def
1 begincodespacerange
<0000> <FFFF>
endcodespacerange
2 beginbfchar
<D55C> <D55C>
<AE00> <AE00>
endbfchar
endcmap
CMapName currentdict /CMap defineresource pop
end
end"""
    content = b"BT\n/FK 18 Tf\n72 720 Td\n<D55CAE00> Tj\n/FH 14 Tf\n0 -28 Td\n(V8 resource probe) Tj\nET"
    objects = [
        b"<< /Type /Catalog /Pages 2 0 R >>",
        b"<< /Type /Pages /Kids [3 0 R] /Count 1 >>",
        b"<< /Type /Page /Parent 2 0 R /MediaBox [0 0 612 792] /Resources << /Font << /FK 4 0 R /FH 8 0 R >> >> /Contents 9 0 R >>",
        b"<< /Type /Font /Subtype /Type0 /BaseFont /FixtureKorean /Encoding /UniKS-UCS2-H /DescendantFonts [5 0 R] /ToUnicode 6 0 R >>",
        b"<< /Type /Font /Subtype /CIDFontType2 /BaseFont /FixtureKorean /CIDSystemInfo << /Registry (Adobe) /Ordering (Korea1) /Supplement 2 >> /FontDescriptor 7 0 R /DW 1000 /CIDToGIDMap /Identity >>",
        stream(to_unicode),
        b"<< /Type /FontDescriptor /FontName /FixtureKorean /Flags 4 /FontBBox [0 -200 1000 900] /ItalicAngle 0 /Ascent 800 /Descent -200 /CapHeight 700 /StemV 80 >>",
        b"<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica >>",
        stream(content),
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
        f"trailer\n<< /Size {len(offsets)} /Root 1 0 R >>\nstartxref\n{xref}\n%%EOF\n".encode(
            "ascii"
        )
    )
    return bytes(pdf)


if __name__ == "__main__":
    result = build_pdf()
    DESTINATION.write_bytes(result)
    print(f"wrote {DESTINATION} ({len(result)} bytes)")
    print(f"sha256 {hashlib.sha256(result).hexdigest()}")
