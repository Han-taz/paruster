from __future__ import annotations

import os
from pathlib import Path
from typing import Any, BinaryIO

from . import _native
from ._errors import KordocError, OutputTooLargeError, typed_error_from_native
from ._models import TryParseResult

MAX_INPUT_BYTES = _native.max_input_bytes()
EMPTY_INPUT_MESSAGE = "빈 버퍼이거나 유효하지 않은 입력입니다."


def _file_not_found() -> KordocError:
    error = typed_error_from_native(
        ValueError("FILE_NOT_FOUND", "Input file was not found or could not be opened")
    )
    assert error is not None
    return error


def _normalize_input(value: Any, *, max_bytes: int | None = None) -> bytes:
    if max_bytes is None:
        max_bytes = MAX_INPUT_BYTES
    if isinstance(value, bytes):
        if len(value) > max_bytes:
            raise OutputTooLargeError("Input exceeds the configured byte limit")
        return value
    if isinstance(value, bytearray):
        if len(value) > max_bytes:
            raise OutputTooLargeError("Input exceeds the configured byte limit")
        return bytes(value)
    if isinstance(value, memoryview):
        if value.nbytes > max_bytes:
            raise OutputTooLargeError("Input exceeds the configured byte limit")
        return value.tobytes()

    if isinstance(value, (str, os.PathLike)):
        try:
            path = Path(os.fsdecode(os.fspath(value)))
            if path.stat().st_size > max_bytes:
                raise OutputTooLargeError("Input exceeds the configured byte limit")
            with path.open("rb") as file:
                return _bounded_read(file, max_bytes)
        except FileNotFoundError as error:
            raise _file_not_found() from error
        except OSError as error:
            raise _file_not_found() from error

    read = getattr(value, "read", None)
    if callable(read):
        return _bounded_read(value, max_bytes)
    raise TypeError(
        "input must be a path, bytes-like object, or binary file-like object"
    )


def _bounded_read(file: BinaryIO, max_bytes: int) -> bytes:
    content = bytearray()
    while len(content) <= max_bytes:
        requested = max_bytes + 1 - len(content)
        chunk = file.read(requested)
        if isinstance(chunk, str):
            raise TypeError("file-like input must return bytes, not text")
        if not isinstance(chunk, (bytes, bytearray, memoryview)):
            raise TypeError("file-like input read() must return bytes")
        chunk_size = chunk.nbytes if isinstance(chunk, memoryview) else len(chunk)
        if chunk_size > requested:
            raise OutputTooLargeError("Input exceeds the configured byte limit")
        if not chunk_size:
            break
        content.extend(bytes(chunk))
    if len(content) > max_bytes:
        raise OutputTooLargeError("Input exceeds the configured byte limit")
    return bytes(content)


def detect_format(value: Any) -> str:
    data = _normalize_input(value)
    try:
        return _native.detect_format_bytes(data)
    except ValueError as error:
        translated = typed_error_from_native(error)
        if translated is None:
            raise
        raise translated from error


def try_parse(value: Any, options: object | None = None) -> TryParseResult:
    if options is not None:
        raise NotImplementedError("Parse options are not implemented yet")
    try:
        data = _normalize_input(value)
    except KordocError as error:
        return TryParseResult.from_dict(
            {
                "success": False,
                "fileType": "unknown",
                "error": error.message,
                "code": error.code,
            }
        )
    return TryParseResult.from_dict(_native.try_parse_bytes(data))


def parse(value: Any, options: object | None = None) -> TryParseResult:
    """Parse input; the foundation currently has no successful parser yet.

    Once core parsing is implemented, success will be returned as the same
    immutable wire envelope until the full ``ParseResult`` object API lands.
    """
    result = try_parse(value, options=options)
    if result.success:
        return result
    error = (
        typed_error_from_native(ValueError(result.code, result.error))
        if result.code
        else None
    )
    if error is None:
        from ._errors import ParseError

        error = ParseError(result.error or EMPTY_INPUT_MESSAGE)
    raise error
