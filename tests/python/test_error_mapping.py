from __future__ import annotations

import json
from pathlib import Path

import kordoc
from kordoc._errors import typed_error_from_native


ROOT = Path(__file__).resolve().parents[2]


def test_every_frozen_error_code_has_a_typed_exception() -> None:
    codes = json.loads((ROOT / "contracts/errors.json").read_text(encoding="utf-8"))[
        "codes"
    ]
    for entry in codes:
        code = entry["code"]
        error = typed_error_from_native(ValueError(code, "safe message"))
        assert type(error).__name__ == _class_name(code)
        assert isinstance(error, kordoc.KordocError)
        assert error.code == code
        assert error.message == "safe message"


def test_native_error_translation_requires_exact_two_string_arguments() -> None:
    assert typed_error_from_native(ValueError("EMPTY_INPUT")) is None
    assert typed_error_from_native(ValueError("EMPTY_INPUT", 42)) is None
    assert typed_error_from_native(ValueError("NOT_A_CODE", "message")) is None
    assert typed_error_from_native(TypeError("EMPTY_INPUT", "message")) is None


def test_native_error_argument_shape_survives_translation() -> None:
    native = ValueError("EMPTY_INPUT", "original message")
    assert native.args == ("EMPTY_INPUT", "original message")
    translated = typed_error_from_native(native)
    assert translated is not None
    assert translated.code == native.args[0]
    assert translated.message == native.args[1]


def _class_name(code: str) -> str:
    return {
        "EMPTY_INPUT": "EmptyInputError",
        "UNSUPPORTED_FORMAT": "UnsupportedFormatError",
        "ENCRYPTED": "EncryptedError",
        "DRM_PROTECTED": "DrmProtectedError",
        "CORRUPTED": "CorruptedError",
        "DECOMPRESSION_BOMB": "DecompressionBombError",
        "ZIP_BOMB": "ZipBombError",
        "IMAGE_BASED_PDF": "ImageBasedPdfError",
        "NO_SECTIONS": "NoSectionsError",
        "PARSE_ERROR": "ParseError",
        "MISSING_DEPENDENCY": "MissingDependencyError",
        "OUTPUT_TOO_LARGE": "OutputTooLargeError",
        "FILE_NOT_FOUND": "InputFileNotFoundError",
    }[code]
