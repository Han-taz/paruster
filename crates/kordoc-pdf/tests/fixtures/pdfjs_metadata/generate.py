#!/usr/bin/env python3
"""Generate deterministic, minimal CC0 PDF Info-metadata probes."""

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


def build_pdf(info: bytes) -> bytes:
    content = b"BT\n/F1 12 Tf\n1 0 0 1 72 720 Tm\n(metadata probe) Tj\nET"
    objects = [
        b"<< /Type /Catalog /Pages 2 0 R >>",
        b"<< /Type /Pages /Kids [3 0 R] /Count 1 >>",
        b"<< /Type /Page /Parent 2 0 R /MediaBox [0 0 612 792] /Resources << /Font << /F1 4 0 R >> >> /Contents 5 0 R >>",
        b"<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica >>",
        stream(content),
        info,
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
        f"trailer\n<< /Size {len(offsets)} /Root 1 0 R /Info 6 0 R >>\nstartxref\n{xref}\n%%EOF\n".encode(
            "ascii"
        )
    )
    return bytes(pdf)


def fixtures() -> dict[str, bytes]:
    return {
        "mixed.pdf": build_pdf(
            b"<< /Title (  Raw title  ) /Author ( Author ) /Creator () /Subject () "
            b"/Keywords (alpha,, beta; alpha ; ;gamma) /CreationDate (D:20250930123456Z) /ModDate 42 >>"
        ),
        "delimiters.pdf": build_pdf(b"<< /Keywords (; ,;;,) >>"),
        "dates.pdf": build_pdf(
            b"<< /Title 17 /CreationDate (prefix D:2025120X suffix) "
            b"/ModDate (xxD:20251399+05'30') >>"
        ),
    }


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument(
        "--write",
        action="store_true",
        help="replace checked-in PDFs (default only verifies exact bytes)",
    )
    args = parser.parse_args()
    output_dir = Path(__file__).parent
    expected = fixtures()
    actual_names = {path.name for path in output_dir.glob("*.pdf")}
    unexpected_names = actual_names - set(expected)
    if unexpected_names or (not args.write and actual_names != set(expected)):
        raise SystemExit(
            f"fixture inventory mismatch: expected {sorted(expected)}, got {sorted(actual_names)}"
        )
    for name, data in expected.items():
        path = output_dir / name
        if args.write:
            path.write_bytes(data)
        elif path.read_bytes() != data:
            raise SystemExit(
                f"fixture bytes differ from generator output: {name}; use --write deliberately"
            )


if __name__ == "__main__":
    main()
