from typing import Literal, TypeAlias, TypedDict

FileType: TypeAlias = Literal[
    "hwpx",
    "hwp",
    "hwp3",
    "hwpml",
    "pdf",
    "xlsx",
    "xls",
    "docx",
    "pptx",
    "image",
    "unknown",
]

class _ParseSuccessRequired(TypedDict):
    success: Literal[True]
    fileType: FileType
    markdown: str
    blocks: list[dict[str, object]]

class ParseSuccessWire(_ParseSuccessRequired, total=False):
    pageCount: int
    isImageBased: bool
    metadata: dict[str, object]
    outline: list[dict[str, object]]
    warnings: list[dict[str, object]]
    images: list[dict[str, object]]
    pages: list[dict[str, object]]
    pageQuality: list[dict[str, object]]
    qualitySummary: dict[str, object]

class _ParseFailureRequired(TypedDict):
    success: Literal[False]
    fileType: FileType
    error: str

class ParseFailureWire(_ParseFailureRequired, total=False):
    code: str
    pageCount: int
    isImageBased: bool

class ParseOptionsWire(TypedDict, total=False):
    pages: list[int | float] | str
    ocr: bool | Literal["force"]
    removeHeaderFooter: bool
    scriptTags: bool
    plain: bool
    htmlTables: bool
    keepTrailingEmptyCols: bool
    classifyTables: bool
    keepEmptyParagraphs: bool
    includeFieldPlaceholders: bool
    password: str
    formulaOcr: bool
    dedupeRunningHeaders: bool
    inlineImages: bool
    images: bool
    tables: bool

def native_version() -> str: ...
def max_input_bytes() -> int: ...
def detect_format_bytes(data: bytes) -> FileType: ...
def detect_zip_format_bytes(data: bytes) -> FileType: ...
def detect_ole2_format_bytes(data: bytes) -> FileType: ...
def is_zip_file_bytes(data: bytes) -> bool: ...
def is_hwpx_file_bytes(data: bytes) -> bool: ...
def is_old_hwp_file_bytes(data: bytes) -> bool: ...
def is_pdf_file_bytes(data: bytes) -> bool: ...
def try_parse_bytes(
    data: bytes, options: ParseOptionsWire | None = ...
) -> ParseSuccessWire | ParseFailureWire: ...
