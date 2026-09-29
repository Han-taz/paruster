//! Shared wire types for the kordoc document-processing workspace.

mod document;
mod error;
mod parsed;

pub use document::{
    BoundingBox, ChunkGranularity, ChunkOptions, ClassifyContext, DocChunk, DocChunkTable,
    DocChunkType, DocumentMetadata, DocumentQualitySummary, ExtractedImage, FileType, ImageData,
    InlineStyle, IrBlock, IrBlockType, IrCell, IrSpan, IrTable, ListType, OcrReason, OutlineItem,
    PageMarkdown, PageMode, PageQuality, ParseFailure, ParseResult, ParseSuccess, ParseWarning,
    TableClassificationKind, TableClassificationReason, TableClassificationSummary,
    TableRepresentation, WarningCode,
};
pub use error::{ErrorCode, KordocError};
pub use parsed::{
    OcrOption, PageEvidence, PageNumber, PageSelection, ParseOptions, ParsedDocument,
};
