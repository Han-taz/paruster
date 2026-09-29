use serde::{Deserialize, Serialize};
use std::fmt;
use thiserror::Error;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ErrorCode {
    #[serde(rename = "EMPTY_INPUT")]
    EmptyInput,
    #[serde(rename = "UNSUPPORTED_FORMAT")]
    UnsupportedFormat,
    #[serde(rename = "ENCRYPTED")]
    Encrypted,
    #[serde(rename = "DRM_PROTECTED")]
    DrmProtected,
    #[serde(rename = "CORRUPTED")]
    Corrupted,
    #[serde(rename = "DECOMPRESSION_BOMB")]
    DecompressionBomb,
    #[serde(rename = "ZIP_BOMB")]
    ZipBomb,
    #[serde(rename = "IMAGE_BASED_PDF")]
    ImageBasedPdf,
    #[serde(rename = "NO_SECTIONS")]
    NoSections,
    #[serde(rename = "PARSE_ERROR")]
    ParseError,
    #[serde(rename = "MISSING_DEPENDENCY")]
    MissingDependency,
    #[serde(rename = "OUTPUT_TOO_LARGE")]
    OutputTooLarge,
    #[serde(rename = "FILE_NOT_FOUND")]
    FileNotFound,
}

impl ErrorCode {
    pub const ALL: [Self; 13] = [
        Self::EmptyInput,
        Self::UnsupportedFormat,
        Self::Encrypted,
        Self::DrmProtected,
        Self::Corrupted,
        Self::DecompressionBomb,
        Self::ZipBomb,
        Self::ImageBasedPdf,
        Self::NoSections,
        Self::ParseError,
        Self::MissingDependency,
        Self::OutputTooLarge,
        Self::FileNotFound,
    ];

    pub const fn as_str(self) -> &'static str {
        match self {
            Self::EmptyInput => "EMPTY_INPUT",
            Self::UnsupportedFormat => "UNSUPPORTED_FORMAT",
            Self::Encrypted => "ENCRYPTED",
            Self::DrmProtected => "DRM_PROTECTED",
            Self::Corrupted => "CORRUPTED",
            Self::DecompressionBomb => "DECOMPRESSION_BOMB",
            Self::ZipBomb => "ZIP_BOMB",
            Self::ImageBasedPdf => "IMAGE_BASED_PDF",
            Self::NoSections => "NO_SECTIONS",
            Self::ParseError => "PARSE_ERROR",
            Self::MissingDependency => "MISSING_DEPENDENCY",
            Self::OutputTooLarge => "OUTPUT_TOO_LARGE",
            Self::FileNotFound => "FILE_NOT_FOUND",
        }
    }
}

impl fmt::Display for ErrorCode {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.as_str())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Error)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[error("{code}: {message}")]
pub struct KordocError {
    pub code: ErrorCode,
    pub message: String,
}

impl KordocError {
    pub fn new(code: ErrorCode, message: impl Into<String>) -> Self {
        Self {
            code,
            message: message.into(),
        }
    }
}
