"""Deterministic inputs used by the detector-helper oracle parity test."""

from __future__ import annotations

from base64 import b64decode
from io import BytesIO
from struct import pack_into
from zipfile import ZIP_STORED, ZipFile, ZipInfo


def build_input(case: dict[str, object]) -> bytes:
    """Build one manifest input without depending on optional packages."""
    if "hex" in case:
        return bytes.fromhex(str(case["hex"]))
    if "cfb_stream" in case:
        return _minimal_cfb(str(case["cfb_stream"]))
    entries = case.get("zip_entries")
    if isinstance(entries, list):
        output = BytesIO()
        with ZipFile(output, "w", compression=ZIP_STORED) as archive:
            for entry in entries:
                if not isinstance(entry, dict):
                    raise TypeError("zip_entries must contain objects")
                info = ZipInfo(str(entry["name"]), date_time=(1980, 1, 1, 0, 0, 0))
                info.compress_type = ZIP_STORED
                archive.writestr(info, str(entry.get("text", "")).encode("utf-8"))
        return output.getvalue()
    if "base64" in case:
        return b64decode(str(case["base64"]), validate=True)
    raise ValueError(f"input case has no supported recipe: {case!r}")


def _minimal_cfb(stream_name: str) -> bytes:
    """Build a tiny synthetic CFB directory containing one empty named stream."""
    if stream_name not in {"Workbook", "Book", "FileHeader"}:
        raise ValueError(f"unsupported synthetic CFB stream: {stream_name!r}")

    end_of_chain = 0xFFFFFFFE
    free_sector = 0xFFFFFFFF
    fat_sector = 0xFFFFFFFD
    data = bytearray(1536)  # header, one directory sector, one FAT sector
    data[:8] = bytes.fromhex("d0cf11e0a1b11ae1")
    pack_into("<HHHH", data, 24, 0x003E, 3, 0xFFFE, 9)
    pack_into("<H", data, 32, 6)
    pack_into("<I", data, 44, 1)  # one FAT sector
    pack_into("<I", data, 48, 0)  # directory starts at sector 0
    pack_into("<I", data, 56, 4096)
    pack_into("<II", data, 60, end_of_chain, 0)
    pack_into("<II", data, 68, end_of_chain, 0)
    pack_into("<109I", data, 76, 1, *([free_sector] * 108))

    def write_entry(index: int, name: str, kind: int) -> None:
        offset = 512 + index * 128
        encoded = (name + "\0").encode("utf-16le")
        data[offset : offset + len(encoded)] = encoded
        pack_into("<H", data, offset + 64, len(encoded))
        data[offset + 66] = kind
        pack_into("<III", data, offset + 68, free_sector, free_sector, free_sector)
        pack_into("<I", data, offset + 116, end_of_chain)

    write_entry(0, "Root Entry", 5)
    write_entry(1, stream_name, 2)
    pack_into("<I", data, 512 + 76, 1)  # root's child is the stream entry
    pack_into("<II", data, 1024, end_of_chain, fat_sector)
    for index in range(2, 128):
        pack_into("<I", data, 1024 + index * 4, free_sector)
    return bytes(data)
