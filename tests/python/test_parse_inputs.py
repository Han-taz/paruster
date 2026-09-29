from __future__ import annotations

from io import BytesIO
from pathlib import Path
from typing import BinaryIO

import pytest

import kordoc
from kordoc import _native
from kordoc._api import _normalize_input
from kordoc._models import TryParseResult


PDF = b"%PDF-1.7\n"


def test_native_byte_functions_keep_a_camel_case_failure_dict() -> None:
    assert _native.detect_format_bytes(PDF) == "pdf"
    assert _native.try_parse_bytes(b"") == {
        "success": False,
        "fileType": "unknown",
        "error": "빈 버퍼이거나 유효하지 않은 입력입니다.",
        "code": "EMPTY_INPUT",
    }


def test_native_zip_security_error_has_exact_two_string_arguments() -> None:
    with pytest.raises(ValueError) as caught:
        _native.detect_format_bytes(b"PK\x03\x04")
    assert len(caught.value.args) == 2
    assert caught.value.args[0] == "ZIP_BOMB"
    assert isinstance(caught.value.args[0], str)
    assert isinstance(caught.value.args[1], str)


def test_detect_format_accepts_memory_and_stream_inputs() -> None:
    assert kordoc.detect_format(PDF) == "pdf"
    assert kordoc.detect_format(bytearray(PDF)) == "pdf"
    assert kordoc.detect_format(memoryview(PDF)) == "pdf"
    assert kordoc.detect_format(BytesIO(PDF)) == "pdf"


def test_detect_format_accepts_paths(tmp_path: Path) -> None:
    path = tmp_path / "sample.pdf"
    path.write_bytes(PDF)
    assert kordoc.detect_format(path) == "pdf"
    assert kordoc.detect_format(str(path)) == "pdf"


def test_detect_and_parse_map_missing_paths_to_typed_file_not_found(
    tmp_path: Path,
) -> None:
    missing = tmp_path / "missing.pdf"
    with pytest.raises(kordoc.InputFileNotFoundError) as detected:
        kordoc.detect_format(missing)
    assert detected.value.code == "FILE_NOT_FOUND"
    with pytest.raises(kordoc.InputFileNotFoundError) as parsed:
        kordoc.parse(missing)
    assert parsed.value.code == "FILE_NOT_FOUND"


def test_try_parse_is_serializable_for_empty_input() -> None:
    result = kordoc.try_parse(b"")
    assert result.to_dict() == {
        "success": False,
        "fileType": "unknown",
        "error": "빈 버퍼이거나 유효하지 않은 입력입니다.",
        "code": "EMPTY_INPUT",
    }


def test_try_parse_result_is_frozen_slotted_and_supports_both_shapes() -> None:
    failure = TryParseResult.from_dict(
        {
            "success": False,
            "fileType": "unknown",
            "error": "empty",
            "code": "EMPTY_INPUT",
        }
    )
    assert failure.to_dict() == {
        "success": False,
        "fileType": "unknown",
        "error": "empty",
        "code": "EMPTY_INPUT",
    }
    assert not hasattr(failure, "__dict__")
    with pytest.raises(AttributeError):
        failure.success = True  # type: ignore[misc]

    success = TryParseResult.from_dict(
        {"success": True, "fileType": "pdf", "markdown": "", "blocks": []}
    )
    assert success.to_dict() == {
        "success": True,
        "fileType": "pdf",
        "markdown": "",
        "blocks": [],
    }

    invalid_values = [
        {"success": True, "fileType": "pdf", "blocks": []},
        {"success": True, "fileType": "pdf", "markdown": ""},
        {"success": False, "fileType": "unknown", "code": "EMPTY_INPUT"},
        {"success": False, "fileType": "unknown", "error": "empty", "pageCount": None},
        {
            "success": True,
            "fileType": "pdf",
            "markdown": "",
            "blocks": [],
            "error": "mixed",
        },
        {
            "success": False,
            "fileType": "unknown",
            "error": "empty",
            "markdown": "mixed",
        },
    ]
    for value in invalid_values:
        with pytest.raises((TypeError, ValueError)):
            TryParseResult.from_dict(value)


def test_try_parse_result_covers_success_projection_and_serializes_nested_bytes() -> (
    None
):
    result = TryParseResult.from_dict(
        {
            "success": True,
            "fileType": "pdf",
            "pageCount": 2,
            "isImageBased": False,
            "markdown": "body",
            "blocks": [
                {
                    "type": "image",
                    "imageData": {"data": b"\x00\xff", "mimeType": "image/png"},
                }
            ],
            "metadata": {"title": "doc"},
            "outline": [{"level": 1, "text": "heading"}],
            "warnings": [],
            "images": [
                {"filename": "a.png", "data": b"\x00\xff", "mimeType": "image/png"}
            ],
            "pages": [{"pageNumber": 1, "markdown": "body"}],
            "pageQuality": [],
            "qualitySummary": {"totalPages": 2},
        }
    )
    data = result.to_dict()
    assert data["pageCount"] == 2
    assert data["isImageBased"] is False
    assert data["blocks"][0]["imageData"]["data"] == [0, 255]
    assert data["images"][0]["data"] == [0, 255]
    import json

    assert json.loads(json.dumps(data)) == data


def test_try_parse_preserves_detected_type_for_unsupported_parser() -> None:
    result = kordoc.try_parse(PDF)
    assert result.to_dict() == {
        "success": False,
        "fileType": "pdf",
        "error": "Parsing is not implemented for this format",
        "code": "UNSUPPORTED_FORMAT",
    }


def test_try_parse_serializes_missing_path_and_size_failures(
    tmp_path: Path, monkeypatch: pytest.MonkeyPatch
) -> None:
    missing = tmp_path / "missing.pdf"
    assert kordoc.try_parse(missing).to_dict()["code"] == "FILE_NOT_FOUND"

    import kordoc._api as api

    monkeypatch.setattr(api, "MAX_INPUT_BYTES", 4)
    assert kordoc.try_parse(b"12345").to_dict()["code"] == "OUTPUT_TOO_LARGE"

    class OversizedStream:
        def read(self, size: int = -1) -> bytes:
            assert size == 5
            return b"123456"

    assert kordoc.try_parse(OversizedStream()).to_dict()["code"] == "OUTPUT_TOO_LARGE"


def test_path_growth_after_stat_is_still_bounded(
    tmp_path: Path, monkeypatch: pytest.MonkeyPatch
) -> None:
    path = tmp_path / "growing.bin"
    path.write_bytes(b"1234")
    original_open = Path.open

    def grow_before_open(self: Path, *args: object, **kwargs: object):
        if self == path:
            with original_open(self, "wb") as file:
                file.write(b"12345")
        return original_open(self, *args, **kwargs)

    monkeypatch.setattr(Path, "open", grow_before_open)
    with pytest.raises(kordoc.OutputTooLargeError):
        _normalize_input(path, max_bytes=4)


def test_parse_raises_the_matching_typed_error() -> None:
    with pytest.raises(kordoc.EmptyInputError) as caught:
        kordoc.parse(b"")
    assert caught.value.code == "EMPTY_INPUT"
    assert caught.value.message == "빈 버퍼이거나 유효하지 않은 입력입니다."


def test_non_none_options_are_not_silently_ignored() -> None:
    with pytest.raises(NotImplementedError):
        kordoc.try_parse(PDF, options={"pages": "1"})
    with pytest.raises(NotImplementedError):
        kordoc.parse(PDF, options={"pages": "1"})


@pytest.mark.parametrize(
    "value",
    [b"1234", bytearray(b"1234"), memoryview(b"1234"), BytesIO(b"1234")],
)
def test_normalization_accepts_all_binary_inputs_at_the_cap(value: object) -> None:
    assert _normalize_input(value, max_bytes=4) == b"1234"


@pytest.mark.parametrize(
    "value",
    [b"12345", bytearray(b"12345"), memoryview(b"12345"), BytesIO(b"12345")],
)
def test_normalization_rejects_all_binary_inputs_over_the_cap(value: object) -> None:
    with pytest.raises(kordoc.OutputTooLargeError):
        _normalize_input(value, max_bytes=4)


def test_file_like_read_is_bounded_and_stream_is_not_closed() -> None:
    class RecordingStream(BytesIO):
        requested: int | None = None

        def read(self, size: int = -1) -> bytes:
            self.requested = size
            return super().read(size)

    stream: BinaryIO = RecordingStream(b"12345")
    with pytest.raises(kordoc.OutputTooLargeError):
        _normalize_input(stream, max_bytes=4)
    assert isinstance(stream, RecordingStream)
    assert stream.requested == 5
    assert not stream.closed


def test_file_like_input_honors_current_position() -> None:
    source = b"ignore%PDF-1.7\n"
    stream = BytesIO(source)
    stream.seek(6)
    assert _normalize_input(stream, max_bytes=100) == PDF
    assert stream.tell() == len(source)


def test_malicious_file_like_cannot_return_more_than_the_configured_cap() -> None:
    class OversizedReader:
        def read(self, size: int = -1) -> bytes:
            assert size == 5
            return b"x" * (size + 100)

    with pytest.raises(kordoc.OutputTooLargeError):
        _normalize_input(OversizedReader(), max_bytes=4)


def test_text_stream_is_rejected_without_encoding() -> None:
    from io import StringIO

    with pytest.raises(TypeError):
        _normalize_input(StringIO("text"), max_bytes=10)


def test_memoryview_limit_uses_nbytes_not_element_count() -> None:
    view = memoryview(b"1234").cast("I")
    assert len(view) == 1
    assert view.nbytes == 4
    assert _normalize_input(view, max_bytes=4) == b"1234"
    with pytest.raises(kordoc.OutputTooLargeError):
        _normalize_input(view, max_bytes=3)


@pytest.mark.parametrize(
    "native_error", [ValueError("unexpected"), ValueError("NOT_A_CODE", "message")]
)
def test_unknown_native_value_errors_pass_through_unchanged(
    monkeypatch: pytest.MonkeyPatch, native_error: ValueError
) -> None:
    import kordoc._api as api

    def fail(_: bytes) -> str:
        raise native_error

    monkeypatch.setattr(api._native, "detect_format_bytes", fail)
    with pytest.raises(ValueError) as caught:
        kordoc.detect_format(PDF)
    assert caught.value is native_error
