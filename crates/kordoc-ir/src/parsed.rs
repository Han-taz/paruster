use serde::{Deserialize, Serialize};

use crate::{
    DocumentMetadata, DocumentQualitySummary, ExtractedImage, IrBlock, OutlineItem, PageQuality,
    ParseWarning,
};

/// Numeric values accepted by the `pages` option, retained until a parser can apply format-aware
/// range and rounding rules against the known document length.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PageNumber(f64);

impl PageNumber {
    pub fn new(value: f64) -> Option<Self> {
        value.is_finite().then_some(Self(value))
    }

    pub fn get(self) -> f64 {
        self.0
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum PageSelection {
    Numbers(Vec<PageNumber>),
    Range(String),
}

/// JSON-supported OCR modes. Python OCR providers are adapted at the Python boundary.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OcrOption {
    Boolean(bool),
    Force,
}

/// Shared options used by Rust format parsers. `Option` preserves omission separately from false.
///
/// Python-only callbacks are translated at the facade boundary. Node-only `filePath` is omitted.
/// This deliberately does not implement `Debug` because it may contain a document password.
#[derive(Clone, Default, PartialEq)]
#[allow(dead_code)]
pub struct ParseOptions {
    pub pages: Option<PageSelection>,
    pub ocr: Option<OcrOption>,
    pub remove_header_footer: Option<bool>,
    pub script_tags: Option<bool>,
    pub plain: Option<bool>,
    pub html_tables: Option<bool>,
    pub keep_trailing_empty_cols: Option<bool>,
    pub classify_tables: Option<bool>,
    pub keep_empty_paragraphs: Option<bool>,
    pub include_field_placeholders: Option<bool>,
    pub password: Option<String>,
    pub formula_ocr: Option<bool>,
    pub dedupe_running_headers: Option<bool>,
    pub inline_images: Option<bool>,
    pub images: Option<bool>,
    pub tables: Option<bool>,
}

fn no_null<'de, D, T>(deserializer: D) -> Result<Option<T>, D::Error>
where
    D: serde::Deserializer<'de>,
    T: Deserialize<'de>,
{
    T::deserialize(deserializer).map(Some)
}

fn is_none<T>(value: &Option<T>) -> bool {
    value.is_none()
}

/// Source-neutral parse output returned by format parsers before core projection.
///
/// This is an internal Rust DTO, not part of the public JSON IR contract. In particular it has no
/// Markdown field: the coordinator-owned core layer creates all Markdown projections.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ParsedDocument {
    pub blocks: Vec<IrBlock>,
    #[serde(default, deserialize_with = "no_null", skip_serializing_if = "is_none")]
    pub page_count: Option<u32>,
    #[serde(default, deserialize_with = "no_null", skip_serializing_if = "is_none")]
    pub metadata: Option<DocumentMetadata>,
    #[serde(default, deserialize_with = "no_null", skip_serializing_if = "is_none")]
    pub outline: Option<Vec<OutlineItem>>,
    #[serde(default, deserialize_with = "no_null", skip_serializing_if = "is_none")]
    pub warnings: Option<Vec<ParseWarning>>,
    #[serde(default, deserialize_with = "no_null", skip_serializing_if = "is_none")]
    pub images: Option<Vec<ExtractedImage>>,
    #[serde(default, deserialize_with = "no_null", skip_serializing_if = "is_none")]
    pub is_image_based: Option<bool>,
    #[serde(default, deserialize_with = "no_null", skip_serializing_if = "is_none")]
    pub page_quality: Option<Vec<PageQuality>>,
    #[serde(default, deserialize_with = "no_null", skip_serializing_if = "is_none")]
    pub quality_summary: Option<DocumentQualitySummary>,
    #[serde(default, deserialize_with = "no_null", skip_serializing_if = "is_none")]
    pub page_evidence: Option<Vec<PageEvidence>>,
}

/// A page that exists in the source document, including pages with no emitted blocks.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PageEvidence {
    pub page_number: u32,
}
