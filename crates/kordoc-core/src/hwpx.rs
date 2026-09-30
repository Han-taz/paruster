use kordoc_ir::{
    DocumentMetadata, ErrorCode, FileType, KordocError, ParseOptions, ParseSuccess, ParsedDocument,
};

use crate::detect::{ParseDispatchError, detect_format};
use crate::limits::validate_input_len;
use crate::parse::{
    Parser, ParserRegistry, assemble_success, catch_parser_unwind, dispatch_error,
    parser_panic_error,
};

pub(crate) struct HwpxParser;

impl Parser for HwpxParser {
    fn parse(
        &self,
        bytes: &[u8],
        _file_type: FileType,
        options: &ParseOptions,
    ) -> Result<ParsedDocument, KordocError> {
        kordoc_hancom::parse_hwpx(bytes, options)
    }

    fn parse_metadata(
        &self,
        bytes: &[u8],
        _file_type: FileType,
        options: &ParseOptions,
    ) -> Result<Option<DocumentMetadata>, KordocError> {
        kordoc_hancom::parse_hwpx_metadata(bytes, options).map(Some)
    }
}

fn require_hwpx(bytes: &[u8]) -> Result<(), ParseDispatchError> {
    if bytes.is_empty() {
        return Err(ParseDispatchError {
            file_type: FileType::Unknown,
            code: ErrorCode::EmptyInput,
            message: "빈 버퍼이거나 유효하지 않은 입력입니다.".into(),
        });
    }
    validate_input_len(bytes.len()).map_err(ParseDispatchError::from)?;
    let file_type = detect_format(bytes).map_err(ParseDispatchError::from)?;
    if file_type != FileType::Hwpx {
        return Err(ParseDispatchError {
            file_type,
            code: ErrorCode::UnsupportedFormat,
            message: "Input is not an HWPX document".into(),
        });
    }
    Ok(())
}

/// Strictly detect HWPX before parsing; project Markdown and pages in the core.
pub fn parse_hwpx_with_options(
    bytes: &[u8],
    options: &ParseOptions,
) -> Result<ParseSuccess, ParseDispatchError> {
    require_hwpx(bytes)?;
    ParserRegistry::built_in()
        .parse(bytes, FileType::Hwpx, options)
        .and_then(|parsed| assemble_success(FileType::Hwpx, parsed, options))
}

/// Use the specialized metadata-only adapter after strict input preflight.
pub fn parse_hwpx_metadata(
    bytes: &[u8],
    options: &ParseOptions,
) -> Result<DocumentMetadata, ParseDispatchError> {
    require_hwpx(bytes)?;
    ParserRegistry::built_in()
        .parse_metadata(bytes, FileType::Hwpx, options)?
        .ok_or_else(|| {
            dispatch_error(
                FileType::Hwpx,
                KordocError::new(
                    ErrorCode::ParseError,
                    "HWPX metadata adapter is unavailable",
                ),
                options,
            )
        })
}

/// Validate structural HWPX content under the same preflight and sanitized panic boundary.
pub fn validate_hwpx(
    bytes: &[u8],
    password: Option<&str>,
) -> Result<kordoc_hancom::ValidateResult, ParseDispatchError> {
    require_hwpx(bytes)?;
    let options = ParseOptions {
        password: password.map(str::to_owned),
        ..ParseOptions::default()
    };
    match catch_parser_unwind(|| kordoc_hancom::validate_hwpx(bytes, password)) {
        Ok(result) => result.map_err(|error| dispatch_error(FileType::Hwpx, error, &options)),
        Err(_) => Err(parser_panic_error(FileType::Hwpx)),
    }
}
