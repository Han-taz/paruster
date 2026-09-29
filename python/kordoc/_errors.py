from __future__ import annotations

from typing import ClassVar


class KordocError(Exception):
    """Base class for stable errors returned by kordoc operations."""

    code: ClassVar[str]

    def __init__(self, message: str) -> None:
        self.message = message
        super().__init__(self.code, message)


class EmptyInputError(KordocError):
    code = "EMPTY_INPUT"


class UnsupportedFormatError(KordocError):
    code = "UNSUPPORTED_FORMAT"


class EncryptedError(KordocError):
    code = "ENCRYPTED"


class DrmProtectedError(KordocError):
    code = "DRM_PROTECTED"


class CorruptedError(KordocError):
    code = "CORRUPTED"


class DecompressionBombError(KordocError):
    code = "DECOMPRESSION_BOMB"


class ZipBombError(KordocError):
    code = "ZIP_BOMB"


class ImageBasedPdfError(KordocError):
    code = "IMAGE_BASED_PDF"


class NoSectionsError(KordocError):
    code = "NO_SECTIONS"


class ParseError(KordocError):
    code = "PARSE_ERROR"


class MissingDependencyError(KordocError):
    code = "MISSING_DEPENDENCY"


class OutputTooLargeError(KordocError):
    code = "OUTPUT_TOO_LARGE"


class InputFileNotFoundError(KordocError):
    code = "FILE_NOT_FOUND"


_ERROR_TYPES: dict[str, type[KordocError]] = {
    error_type.code: error_type
    for error_type in (
        EmptyInputError,
        UnsupportedFormatError,
        EncryptedError,
        DrmProtectedError,
        CorruptedError,
        DecompressionBombError,
        ZipBombError,
        ImageBasedPdfError,
        NoSectionsError,
        ParseError,
        MissingDependencyError,
        OutputTooLargeError,
        InputFileNotFoundError,
    )
}


def typed_error_from_native(error: BaseException) -> KordocError | None:
    args = error.args
    if (
        not isinstance(error, ValueError)
        or len(args) != 2
        or not all(isinstance(arg, str) for arg in args)
    ):
        return None
    code, message = args
    error_type = _ERROR_TYPES.get(code)
    return error_type(message) if error_type is not None else None
