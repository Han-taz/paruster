//! Shared wire types for the kordoc document-processing workspace.

mod document;
mod error;

pub use document::{
    BoundingBox, DocumentMetadata, DocumentQualitySummary, ExtractedImage, FileType, ImageData,
    InlineStyle, IrBlock, IrBlockType, IrCell, IrSpan, IrTable, ListType, OcrReason, OutlineItem,
    PageMarkdown, PageMode, PageQuality, ParseFailure, ParseResult, ParseSuccess, ParseWarning,
    TableClassificationKind, TableClassificationReason, TableClassificationSummary, WarningCode,
};
pub use error::{ErrorCode, KordocError};
