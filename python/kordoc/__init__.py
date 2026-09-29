from ._api import detect_format, parse, try_parse
from ._errors import (
    CorruptedError,
    DecompressionBombError,
    DrmProtectedError,
    EmptyInputError,
    EncryptedError,
    ImageBasedPdfError,
    InputFileNotFoundError,
    KordocError,
    MissingDependencyError,
    NoSectionsError,
    OutputTooLargeError,
    ParseError,
    UnsupportedFormatError,
    ZipBombError,
)
from ._models import TryParseResult
from ._native import native_version

__version__ = "0.1.0"
__all__ = [
    "CorruptedError",
    "DecompressionBombError",
    "DrmProtectedError",
    "EmptyInputError",
    "EncryptedError",
    "ImageBasedPdfError",
    "InputFileNotFoundError",
    "KordocError",
    "MissingDependencyError",
    "NoSectionsError",
    "OutputTooLargeError",
    "ParseError",
    "TryParseResult",
    "UnsupportedFormatError",
    "ZipBombError",
    "__version__",
    "detect_format",
    "native_version",
    "parse",
    "try_parse",
]
