from __future__ import annotations

import json
import zipfile
from io import BytesIO
from pathlib import Path

import kordoc
import pytest

PNG_BYTES = bytes.fromhex(
    "89504e470d0a1a0a0000000d49484452000000010000000108060000001f15c489"
    "0000000b49444154789c636000020000050001a5f645400000000049454e44ae426082"
)
PDF_BYTES = b"%PDF-1.7\n"


def _section(text: str, *, image: bool = False) -> bytes:
    image_xml = (
        '<hp:pic><hp:imgRect binaryItemIDRef="pixel.png"/></hp:pic>' if image else ""
    )
    return (
        '<?xml version="1.0" encoding="UTF-8"?>'
        '<hs:sec xmlns:hs="http://www.hancom.co.kr/hwpml/2011/section" '
        'xmlns:hp="http://www.hancom.co.kr/hwpml/2011/paragraph">'
        f"<hp:p><hp:run><hp:t>{text}</hp:t>{image_xml}</hp:run></hp:p>"
        "</hs:sec>"
    ).encode()


def _write_member(archive: zipfile.ZipFile, name: str, data: bytes) -> None:
    info = zipfile.ZipInfo(name, date_time=(1980, 1, 1, 0, 0, 0))
    info.compress_type = zipfile.ZIP_STORED
    info.create_system = 3
    info.external_attr = 0o600 << 16
    archive.writestr(info, data)


def _hwpx_bytes(
    sections: tuple[bytes, ...] | None = None, *, image: bool = True
) -> bytes:
    if sections is None:
        sections = (_section("First section", image=image), _section("Second section"))
    output = BytesIO()
    with zipfile.ZipFile(output, "w") as archive:
        _write_member(archive, "mimetype", b"application/hwp+zip")
        _write_member(
            archive,
            "META-INF/container.xml",
            b'<?xml version="1.0" encoding="UTF-8"?>'
            b'<ocf:container xmlns:ocf="urn:oasis:names:tc:opendocument:xmlns:container">'
            b'<ocf:rootfiles><ocf:rootfile full-path="Contents/content.hpf" '
            b'media-type="application/hwp+zip"/></ocf:rootfiles></ocf:container>',
        )
        item_refs = "".join(
            f'<opf:item id="section{index}" href="section{index}.xml" '
            'media-type="application/xml"/>'
            for index in range(len(sections))
        )
        spine_refs = "".join(
            f'<opf:itemref idref="section{index}"/>' for index in range(len(sections))
        )
        if image:
            item_refs += (
                '<opf:item id="pixel" href="BinData/pixel.png" media-type="image/png"/>'
            )
        content = (
            '<?xml version="1.0" encoding="UTF-8"?>'
            '<opf:package xmlns:opf="http://www.idpf.org/2007/opf" '
            'xmlns:dc="http://purl.org/dc/elements/1.1/">'
            "<opf:metadata><dc:title>API fixture</dc:title>"
            "<dc:creator>paruster tests</dc:creator></opf:metadata>"
            f"<opf:manifest>{item_refs}</opf:manifest>"
            f"<opf:spine>{spine_refs}</opf:spine></opf:package>"
        ).encode()
        _write_member(archive, "Contents/content.hpf", content)
        header = (
            '<?xml version="1.0" encoding="UTF-8"?>'
            '<hh:head xmlns:hh="http://www.hancom.co.kr/hwpml/2011/head" '
            f'secCnt="{len(sections)}"/>'
        ).encode()
        _write_member(archive, "Contents/header.xml", header)
        for index, section in enumerate(sections):
            _write_member(archive, f"Contents/section{index}.xml", section)
        if image:
            _write_member(archive, "BinData/pixel.png", PNG_BYTES)
    return output.getvalue()


def test_fixture_is_deterministic_with_first_stored_mimetype() -> None:
    data = _hwpx_bytes()
    assert data == _hwpx_bytes()
    with zipfile.ZipFile(BytesIO(data)) as archive:
        first = archive.infolist()[0]
        assert first.filename == "mimetype"
        assert first.compress_type == zipfile.ZIP_STORED
        assert archive.read(first) == b"application/hwp+zip"


def test_format_specific_parse_accepts_bytes_path_and_positioned_binary_stream(
    tmp_path: Path,
) -> None:
    data = _hwpx_bytes()
    path = tmp_path / "api-fixture.hwpx"
    path.write_bytes(data)
    positioned = BytesIO(b"ignored-prefix" + data)
    positioned.seek(len(b"ignored-prefix"))

    byte_result = kordoc.parse_hwpx(data)
    path_result = kordoc.parse_hwpx(path)
    stream_result = kordoc.parse_hwpx(positioned)
    bytearray_result = kordoc.parse_hwpx(bytearray(data))
    memoryview_result = kordoc.parse_hwpx(memoryview(data))

    assert isinstance(byte_result, kordoc.TryParseResult)
    assert byte_result.success is True
    assert isinstance(byte_result.document, kordoc.Document)
    assert (
        byte_result.to_dict()
        == path_result.to_dict()
        == stream_result.to_dict()
        == bytearray_result.to_dict()
        == memoryview_result.to_dict()
    )


def test_hwpx_result_uses_full_camel_case_wire_and_immutable_document() -> None:
    result = kordoc.parse_hwpx(_hwpx_bytes(image=False))
    document = result.document
    assert document is not None
    wire = result.to_dict()
    assert wire["fileType"] == "hwpx"
    assert wire["pageCount"] == 2
    assert wire["metadata"]["pageMode"] == "section"
    assert wire["metadata"]["pageCount"] == 2
    assert wire["pages"] == [
        {"pageNumber": 1, "markdown": "First section"},
        {"pageNumber": 2, "markdown": "Second section"},
    ]
    assert "file_type" not in wire
    assert "page_count" not in wire
    assert document.page_count == 2
    with pytest.raises(AttributeError):
        document.markdown = "changed"  # type: ignore[misc]


def test_hwpx_page_selection_keeps_source_page_count() -> None:
    result = kordoc.parse_hwpx(_hwpx_bytes(image=False), options={"pages": [2]})
    document = result.document
    assert document is not None
    assert document.file_type == "hwpx"
    assert document.page_count == 2
    assert document.metadata is not None
    assert document.metadata["pageCount"] == 2
    assert document.markdown == "Second section"
    assert document.pages is not None
    assert document.pages == ({"pageNumber": 2, "markdown": "Second section"},)


def test_hwpx_images_preserve_bytes_and_json_wire_uses_integer_arrays() -> None:
    result = kordoc.parse_hwpx(_hwpx_bytes())
    document = result.document
    assert document is not None
    assert document.images is not None
    assert document.images[0]["data"] == PNG_BYTES
    image_block = next(block for block in document.blocks if block["type"] == "image")
    assert image_block["imageData"]["data"] == PNG_BYTES
    wire = result.to_dict()
    assert wire["images"][0]["data"] == list(PNG_BYTES)
    assert image_block["imageData"]["mimeType"] == "image/png"
    assert json.loads(json.dumps(wire)) == wire


@pytest.mark.parametrize(
    "parse_api", [kordoc.parse, kordoc.try_parse, kordoc.parse_hwpx]
)
def test_hwpx_images_false_drops_image_bytes_after_projection(parse_api) -> None:
    data = _hwpx_bytes()

    default = parse_api(data)
    with_images = parse_api(data, options={"images": True})
    without_images = parse_api(data, options={"images": False})

    assert with_images.to_dict() == default.to_dict()
    default_document = default.document
    without_images_document = without_images.document
    assert default_document is not None
    assert without_images_document is not None
    assert default_document.markdown == without_images_document.markdown
    assert default_document.pages == without_images_document.pages
    assert default_document.images
    assert without_images_document.images is None
    image_block = next(
        block for block in without_images_document.blocks if block["type"] == "image"
    )
    assert "imageData" not in image_block


def test_hwpx_tables_false_preserves_table_topology() -> None:
    fixture = (
        Path(__file__).parents[2]
        / "tests/golden/document/hwpx/fixtures/nested_table.hwpx"
    )

    default = kordoc.parse_hwpx(fixture)
    tables_false = kordoc.parse_hwpx(fixture, options={"tables": False})

    default_document = default.document
    tables_false_document = tables_false.document
    assert default_document is not None
    assert tables_false_document is not None
    assert any(block["type"] == "table" for block in default_document.blocks)
    assert tables_false_document.blocks == default_document.blocks
    assert tables_false_document.markdown == default_document.markdown


def test_generic_parse_and_try_parse_dispatch_to_hwpx() -> None:
    data = _hwpx_bytes(image=False)
    parsed = kordoc.parse(data)
    attempted = kordoc.try_parse(data)
    assert parsed.success is True
    assert attempted.success is True
    assert parsed.file_type == attempted.file_type == "hwpx"
    assert parsed.document is not None
    assert attempted.document is not None
    assert parsed.document.to_dict() == attempted.document.to_dict()


def test_validate_hwpx_counts_files_but_not_directory_entries() -> None:
    data = _hwpx_bytes()
    archive_buffer = BytesIO(data)
    with zipfile.ZipFile(archive_buffer, "a") as archive:
        for name in ("META-INF/", "Contents/", "BinData/"):
            info = zipfile.ZipInfo(name, date_time=(1980, 1, 1, 0, 0, 0))
            info.external_attr = (0o40700 << 16) | 0x10
            archive.writestr(info, b"")
    data_with_directories = archive_buffer.getvalue()
    with zipfile.ZipFile(BytesIO(data_with_directories)) as archive:
        assert len(archive.infolist()) == 10
        assert sum(not item.is_dir() for item in archive.infolist()) == 7

    result = kordoc.validate_hwpx(data_with_directories)

    assert isinstance(result, kordoc.ValidateResult)
    assert result.ok is True
    assert result.issues == ()
    assert result.entry_count == 7
    assert result.to_dict() == {"ok": True, "issues": [], "entryCount": 7}


@pytest.mark.parametrize(
    ("data", "error_type", "code"),
    [
        (b"", kordoc.EmptyInputError, "EMPTY_INPUT"),
        (PDF_BYTES, kordoc.UnsupportedFormatError, "UNSUPPORTED_FORMAT"),
    ],
)
def test_validate_hwpx_returns_typed_empty_and_wrong_format_errors(
    data: bytes, error_type: type[Exception], code: str
) -> None:
    with pytest.raises(error_type) as caught:
        kordoc.validate_hwpx(data)
    assert caught.value.code == code


def test_format_specific_parse_returns_typed_empty_malformed_and_wrong_format_errors() -> (
    None
):
    with pytest.raises(kordoc.EmptyInputError) as empty:
        kordoc.parse_hwpx(b"")
    assert empty.value.code == "EMPTY_INPUT"

    with pytest.raises(kordoc.UnsupportedFormatError) as wrong_format:
        kordoc.parse_hwpx(PDF_BYTES)
    assert wrong_format.value.code == "UNSUPPORTED_FORMAT"

    malformed = _hwpx_bytes((b"<hs:sec><hp:p>",), image=False)
    with pytest.raises(kordoc.NoSectionsError) as no_sections:
        kordoc.parse_hwpx(malformed)
    assert no_sections.value.code == "NO_SECTIONS"


@pytest.mark.parametrize(
    ("options", "error_type"),
    [
        ("invalid", TypeError),
        ({"unknown_option": True}, ValueError),
        ({"images": 1}, TypeError),
        ({"pages": [True]}, TypeError),
        ({"on_progress": lambda *_: None}, NotImplementedError),
    ],
)
def test_hwpx_parse_uses_strict_option_normalization(
    options: object, error_type: type[Exception]
) -> None:
    with pytest.raises(error_type):
        kordoc.parse_hwpx(_hwpx_bytes(image=False), options=options)


def test_parse_and_validator_reject_invalid_or_overlong_passwords() -> None:
    data = _hwpx_bytes(image=False)
    overlong = "p" * 65_537
    with pytest.raises(TypeError):
        kordoc.parse_hwpx(data, options={"password": 7})
    with pytest.raises(ValueError):
        kordoc.parse_hwpx(data, options={"password": overlong})
    with pytest.raises(TypeError):
        kordoc.validate_hwpx(data, password=7)
    with pytest.raises(ValueError):
        kordoc.validate_hwpx(data, password=overlong)
