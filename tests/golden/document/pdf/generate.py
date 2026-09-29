#!/usr/bin/env python3
"""Generate deterministic, original CC0-1.0 PDF fixtures (stdlib only)."""

# SPDX-License-Identifier: CC0-1.0

from __future__ import annotations

import argparse
from pathlib import Path

ROOT = Path(__file__).parent


def pdf(objects: list[tuple[int, bytes]], trailer: bytes = b"<< /Root 1 0 R >>") -> bytes:
    out = bytearray(b"%PDF-1.7\n%\x00\xe2\xe3\xcf\xd3\n")
    offsets: dict[int, int] = {}
    for number, body in objects:
        offsets[number] = len(out)
        out.extend(f"{number} 0 obj\n".encode())
        out.extend(body)
        out.extend(b"\nendobj\n")
    xref_offset = len(out)
    size = max(offsets, default=0) + 1
    out.extend(f"xref\n0 {size}\n0000000000 65535 f \n".encode())
    for number in range(1, size):
        offset = offsets.get(number)
        out.extend((f"{offset:010} 00000 n \n" if offset is not None else "0000000000 00000 f \n").encode())
    trailer_fields = trailer.removeprefix(b"<<").removesuffix(b">>").strip().splitlines()
    out.extend(b"trailer\n<<\n")
    for field in trailer_fields:
        out.extend(field.strip() + b"\n")
    out.extend(f">>\nstartxref\n{xref_offset}\n%%EOF\n".encode())
    return bytes(out)


def write_fixtures() -> None:
    catalog = b"<< /Type /Catalog /Pages 2 0 R >>"
    pages = b"<< /Type /Pages /Kids [3 0 R] /Count 1 >>"
    page = b"<< /Type /Page /Parent 2 0 R /MediaBox [0 0 612 792] >>"
    (ROOT / "classic_xref.pdf").write_bytes(pdf([(1, catalog), (2, pages), (3, page)]))

    # A plain, unfiltered xref stream. The reader need only honor /W and /Index;
    # entries describe object 0 (free) and objects 1-3 (normal).
    stream_start = bytearray(b"%PDF-1.5\n%\x00\xe2\xe3\xcf\xd3\n")
    xref_objects: list[tuple[int, bytes]] = []
    xoff: dict[int, int] = {}
    for number, body in [(1, catalog), (2, pages), (3, page)]:
        xoff[number] = len(stream_start)
        stream_start.extend(f"{number} 0 obj\n".encode() + body + b"\nendobj\n")
    xref_id = 4
    xoff[xref_id] = len(stream_start)
    entries = bytearray([0, 0, 0, 0, 0, 255, 255])
    for number in range(1, 5):
        entries.extend([1])
        entries.extend(xoff[number].to_bytes(4, "big"))
        entries.extend([0, 0])
    xref_body = (b"<< /Type /XRef /Size 5 /Root 1 0 R /W [1 4 2] /Length "
                 + str(len(entries)).encode() + b" >>\nstream\n" + entries
                 + b"\nendstream")
    stream_start.extend(f"{xref_id} 0 obj\n".encode() + xref_body + b"\nendobj\n")
    stream_start.extend(f"startxref\n{xoff[xref_id]}\n%%EOF\n".encode())
    (ROOT / "xref_stream.pdf").write_bytes(stream_start)

    # A real type-2 xref path for two objects inside an unfiltered ObjStm.
    first_object = b"<< /Label (inside) >> "
    second_object = b"[5 0 R]"
    embedded = first_object + second_object
    header = f"5 0 6 {len(first_object)} ".encode()
    body = header + embedded
    objstm = b"<< /Type /ObjStm /N 2 /First " + str(len(header)).encode() + b" /Length " + str(len(body)).encode() + b" >>\nstream\n" + body + b"\nendstream"
    data = bytearray(b"%PDF-1.7\n")
    offsets: dict[int, int] = {}
    for number, content in [(1, catalog), (2, pages), (3, page), (4, objstm)]:
        offsets[number] = len(data)
        data.extend(f"{number} 0 obj\n".encode() + content + b"\nendobj\n")
    xref_id = 7
    offsets[xref_id] = len(data)
    entries = bytearray([0, 0, 0, 0, 0, 255, 255])
    for number in range(1, 8):
        if number in offsets:
            entries.extend([1])
            entries.extend(offsets[number].to_bytes(4, "big"))
            entries.extend([0, 0])
        elif number in (5, 6):
            entries.extend([2, 0, 0, 0, 4, 0, number - 5])
        else:
            entries.extend([0, 0, 0, 0, 0, 0, 0])
    xref = (b"<< /Type /XRef /Size 8 /Root 1 0 R /W [1 4 2] /Length "
            + str(len(entries)).encode() + b" >>\nstream\n" + entries + b"\nendstream")
    data.extend(f"{xref_id} 0 obj\n".encode() + xref + b"\nendobj\n")
    data.extend(f"startxref\n{offsets[xref_id]}\n%%EOF\n".encode())
    (ROOT / "object_stream.pdf").write_bytes(data)

    # A second revision replaces object 3 and points /Prev at revision one.
    base = pdf([(1, catalog), (2, pages), (3, page)])
    revision = bytearray(base)
    changed_page = b"<< /Type /Page /Parent 2 0 R /MediaBox [0 0 300 400] >>"
    changed_offset = len(revision)
    revision.extend(b"3 0 obj\n" + changed_page + b"\nendobj\n")
    xref_offset = len(revision)
    revision.extend(f"xref\n3 1\n{changed_offset:010} 00000 n \n".encode())
    previous_xref = int(base.split(b"startxref\n")[-1].splitlines()[0])
    revision.extend(f"trailer\n<<\n/Size 4\n/Root 1 0 R\n/Prev {previous_xref}\n>>\nstartxref\n{xref_offset}\n%%EOF\n".encode())
    (ROOT / "incremental_revision.pdf").write_bytes(revision)

    generation_mismatch = pdf([(1, catalog), (2, pages), (3, page)]).replace(b"3 0 obj", b"3 1 obj")
    generation_mismatch = generation_mismatch.replace(b"00000 n \ntrailer", b"00001 n \ntrailer")
    (ROOT / "generation_mismatch.pdf").write_bytes(generation_mismatch)
    cyclic = b"<< /Loop 1 0 R >>"
    (ROOT / "reference_cycle.pdf").write_bytes(pdf([(1, cyclic)]))
    mixed = b"<< /Length 6 /Filter [/ASCIIHexDecode /ASCII85Decode] >>\nstream\n4142>\nendstream"
    (ROOT / "mixed_filters.pdf").write_bytes(pdf([(1, mixed)]))
    ascii_hex = b"<< /Length 6 /Filter /ASCIIHexDecode >>\nstream\n4142>\nendstream"
    (ROOT / "ascii_hex.pdf").write_bytes(pdf([(1, ascii_hex)]))
    malformed = b"<< /Length 999 >>\nstream\nshort\nendstream"
    (ROOT / "malformed_length.pdf").write_bytes(pdf([(1, malformed)]))
    encrypted = pdf([(1, catalog)], b"<< /Root 1 0 R /Encrypt 2 0 R >>")
    encrypted = encrypted.replace(b"trailer\n<<\n/Root 1 0 R /Encrypt 2 0 R\n>>", b"2 0 obj\n<< /Filter /Standard /V 1 /R 2 /O <00> /U <00> /P -4 >>\nendobj\ntrailer\n<<\n/Root 1 0 R\n/Encrypt 2 0 R\n>>")
    (ROOT / "encrypted_trailer.pdf").write_bytes(encrypted)


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("--seed", type=int, default=20260930)
    parser.parse_args()
    write_fixtures()


if __name__ == "__main__":
    main()
