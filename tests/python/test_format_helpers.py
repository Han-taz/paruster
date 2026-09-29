from __future__ import annotations

from collections.abc import Callable
from io import BytesIO
from pathlib import Path
from typing import Any

import kordoc
import pytest


@pytest.mark.parametrize(
    ("name", "payload", "expected"),
    (
        ("is_zip_file", b"PK\x03\x04", True),
        ("is_hwpx_file", b"PK\x03\x04not-a-valid-archive", True),
        ("is_old_hwp_file", b"\xd0\xcf\x11\xe0", True),
        ("is_pdf_file", b"%PDF", True),
        ("is_zip_file", b"PK\x05\x06", False),
        ("is_hwpx_file", b"PK\x05\x06", False),
        ("is_old_hwp_file", b"\xd0\xcf\x11", False),
        ("is_pdf_file", b"%PD", False),
    ),
)
def test_magic_predicates_preserve_legacy_four_byte_semantics(
    name: str, payload: bytes, expected: bool
) -> None:
    predicate: Callable[[Any], bool] = getattr(kordoc, name)
    assert predicate(payload) is expected
    assert predicate(bytearray(payload)) is expected
    assert predicate(memoryview(payload)) is expected
    assert predicate(BytesIO(payload)) is expected


def test_magic_predicates_accept_paths(tmp_path: Path) -> None:
    path = tmp_path / "sample.bin"
    path.write_bytes(b"%PDF-extra")

    assert kordoc.is_pdf_file(path)
    assert not kordoc.is_zip_file(path)


def test_malformed_container_refinement_returns_unknown() -> None:
    assert kordoc.detect_zip_format(b"PK\x03\x04not-a-valid-archive") == "unknown"
    assert kordoc.detect_ole2_format(b"\xd0\xcf\x11\xe0not-a-valid-cfb") == "unknown"


def test_container_refinement_preserves_specific_format_detection() -> None:
    assert kordoc.detect_zip_format(_minimal_zip("word/document.xml")) == "docx"
    assert kordoc.detect_zip_format(_minimal_zip("xl/workbook.xml")) == "xlsx"
    assert kordoc.detect_zip_format(_minimal_zip("ppt/presentation.xml")) == "pptx"
    assert kordoc.detect_zip_format(_minimal_zip("Contents/section0.xml")) == "hwpx"


def _minimal_zip(name: str) -> bytes:
    import io
    import zipfile

    output = io.BytesIO()
    with zipfile.ZipFile(output, "w") as archive:
        archive.writestr(name, b"")
    return output.getvalue()
