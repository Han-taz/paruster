use serde::de::{Error as DeError, Unexpected, Visitor};
use serde::{Deserialize, Serialize};
use std::fmt;

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

fn deserialize_u32<'de, D>(deserializer: D) -> Result<u32, D::Error>
where
    D: serde::Deserializer<'de>,
{
    struct U32Visitor;

    impl Visitor<'_> for U32Visitor {
        type Value = u32;

        fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
            formatter.write_str("an unsigned 32-bit integer or an integral JSON number in range")
        }

        fn visit_u64<E>(self, value: u64) -> Result<Self::Value, E>
        where
            E: DeError,
        {
            u32::try_from(value).map_err(|_| E::invalid_value(Unexpected::Unsigned(value), &self))
        }

        fn visit_i64<E>(self, value: i64) -> Result<Self::Value, E>
        where
            E: DeError,
        {
            u32::try_from(value).map_err(|_| E::invalid_value(Unexpected::Signed(value), &self))
        }

        fn visit_f64<E>(self, value: f64) -> Result<Self::Value, E>
        where
            E: DeError,
        {
            if value.is_finite() && value.fract() == 0.0 && (0.0..=u32::MAX as f64).contains(&value)
            {
                Ok(value as u32)
            } else {
                Err(E::invalid_value(Unexpected::Float(value), &self))
            }
        }
    }

    deserializer.deserialize_any(U32Visitor)
}

fn deserialize_optional_u32<'de, D>(deserializer: D) -> Result<Option<u32>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    deserialize_u32(deserializer).map(Some)
}

#[derive(Debug)]
struct U32Element(u32);

impl<'de> Deserialize<'de> for U32Element {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        deserialize_u32(deserializer).map(Self)
    }
}

fn deserialize_u32_vec<'de, D>(deserializer: D) -> Result<Vec<u32>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    Vec::<U32Element>::deserialize(deserializer)
        .map(|values| values.into_iter().map(|value| value.0).collect())
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum IrBlockType {
    #[default]
    #[serde(rename = "paragraph")]
    Paragraph,
    #[serde(rename = "table")]
    Table,
    #[serde(rename = "heading")]
    Heading,
    #[serde(rename = "list")]
    List,
    #[serde(rename = "image")]
    Image,
    #[serde(rename = "separator")]
    Separator,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum FileType {
    #[serde(rename = "hwpx")]
    Hwpx,
    #[serde(rename = "hwp")]
    Hwp,
    #[serde(rename = "hwp3")]
    Hwp3,
    #[serde(rename = "hwpml")]
    Hwpml,
    #[serde(rename = "pdf")]
    Pdf,
    #[serde(rename = "xlsx")]
    Xlsx,
    #[serde(rename = "xls")]
    Xls,
    #[serde(rename = "docx")]
    Docx,
    #[serde(rename = "pptx")]
    Pptx,
    #[serde(rename = "image")]
    Image,
    #[default]
    #[serde(rename = "unknown")]
    Unknown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum WarningCode {
    #[serde(rename = "SKIPPED_IMAGE")]
    SkippedImage,
    #[serde(rename = "SKIPPED_OLE")]
    SkippedOle,
    #[serde(rename = "TRUNCATED_TABLE")]
    TruncatedTable,
    #[serde(rename = "OCR_FALLBACK")]
    OcrFallback,
    #[serde(rename = "UNSUPPORTED_ELEMENT")]
    UnsupportedElement,
    #[serde(rename = "BROKEN_ZIP_RECOVERY")]
    BrokenZipRecovery,
    #[serde(rename = "HIDDEN_TEXT_FILTERED")]
    HiddenTextFiltered,
    #[serde(rename = "MALFORMED_XML")]
    MalformedXml,
    #[serde(rename = "PARTIAL_PARSE")]
    PartialParse,
    #[serde(rename = "LENIENT_CFB_RECOVERY")]
    LenientCfbRecovery,
    #[serde(rename = "NEEDS_OCR")]
    NeedsOcr,
    #[serde(rename = "OCR_FAILED")]
    OcrFailed,
    #[serde(rename = "OCR_APPLIED")]
    OcrApplied,
    #[serde(rename = "OCR_LOW_CONF")]
    OcrLowConf,
    #[serde(rename = "COM_EMPTY")]
    ComEmpty,
    #[serde(rename = "DRM_COM_FALLBACK")]
    DrmComFallback,
    #[serde(rename = "PAGE_BOUNDARY_APPROXIMATE")]
    PageBoundaryApproximate,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PageMode {
    #[serde(rename = "layout")]
    Layout,
    #[serde(rename = "section")]
    Section,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum TableClassificationKind {
    #[serde(rename = "semantic-table")]
    SemanticTable,
    #[serde(rename = "non-tabular-layout")]
    NonTabularLayout,
    #[default]
    #[serde(rename = "uncertain")]
    Uncertain,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TableClassificationReason {
    #[serde(rename = "repeated-row-schema")]
    RepeatedRowSchema,
    #[serde(rename = "grid-regularity")]
    GridRegularity,
    #[serde(rename = "high-active-density")]
    HighActiveDensity,
    #[serde(rename = "column-type-consistency")]
    ColumnTypeConsistency,
    #[serde(rename = "nested-structure-wrapper")]
    NestedStructureWrapper,
    #[serde(rename = "span-irregularity")]
    SpanIrregularity,
    #[serde(rename = "spacer-bands")]
    SpacerBands,
    #[serde(rename = "extreme-sparsity")]
    ExtremeSparsity,
    #[serde(rename = "diagram-context-keyword")]
    DiagramContextKeyword,
    #[serde(rename = "low-evidence")]
    LowEvidence,
    #[serde(rename = "ambiguous-scores")]
    AmbiguousScores,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum OcrReason {
    #[serde(rename = "vector_text")]
    VectorText,
    #[serde(rename = "low_text")]
    LowText,
    #[serde(rename = "high_pua")]
    HighPua,
    #[serde(rename = "high_control")]
    HighControl,
    #[serde(rename = "high_replacement")]
    HighReplacement,
    #[serde(rename = "garbled_hangul")]
    GarbledHangul,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ListType {
    #[serde(rename = "ordered")]
    Ordered,
    #[serde(rename = "unordered")]
    Unordered,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct IrSpan {
    pub text: String,
    #[serde(default, deserialize_with = "no_null", skip_serializing_if = "is_none")]
    pub bold: Option<bool>,
    #[serde(default, deserialize_with = "no_null", skip_serializing_if = "is_none")]
    pub italic: Option<bool>,
    #[serde(default, deserialize_with = "no_null", skip_serializing_if = "is_none")]
    pub strike: Option<bool>,
    #[serde(default, deserialize_with = "no_null", skip_serializing_if = "is_none")]
    pub underline: Option<bool>,
    #[serde(default, deserialize_with = "no_null", skip_serializing_if = "is_none")]
    pub code: Option<bool>,
    #[serde(default, deserialize_with = "no_null", skip_serializing_if = "is_none")]
    pub placeholder: Option<bool>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ImageData {
    pub data: Vec<u8>,
    pub mime_type: String,
    #[serde(default, deserialize_with = "no_null", skip_serializing_if = "is_none")]
    pub filename: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BoundingBox {
    #[serde(deserialize_with = "deserialize_u32")]
    pub page: u32,
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct InlineStyle {
    #[serde(default, deserialize_with = "no_null", skip_serializing_if = "is_none")]
    pub bold: Option<bool>,
    #[serde(default, deserialize_with = "no_null", skip_serializing_if = "is_none")]
    pub italic: Option<bool>,
    #[serde(default, deserialize_with = "no_null", skip_serializing_if = "is_none")]
    pub strike: Option<bool>,
    #[serde(default, deserialize_with = "no_null", skip_serializing_if = "is_none")]
    pub underline: Option<bool>,
    #[serde(default, deserialize_with = "no_null", skip_serializing_if = "is_none")]
    pub font_size: Option<f64>,
    #[serde(default, deserialize_with = "no_null", skip_serializing_if = "is_none")]
    pub font_name: Option<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct TableClassificationSummary {
    pub kind: TableClassificationKind,
    pub confidence: f64,
    pub semantic_score: f64,
    pub non_tabular_score: f64,
    pub reasons: Vec<TableClassificationReason>,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct IrCell {
    pub text: String,
    #[serde(deserialize_with = "deserialize_u32")]
    pub col_span: u32,
    #[serde(deserialize_with = "deserialize_u32")]
    pub row_span: u32,
    #[serde(default, deserialize_with = "no_null", skip_serializing_if = "is_none")]
    pub blocks: Option<Vec<IrBlock>>,
    #[serde(default, deserialize_with = "no_null", skip_serializing_if = "is_none")]
    pub is_header: Option<bool>,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct IrTable {
    #[serde(deserialize_with = "deserialize_u32")]
    pub rows: u32,
    #[serde(deserialize_with = "deserialize_u32")]
    pub cols: u32,
    pub cells: Vec<Vec<IrCell>>,
    #[serde(default, deserialize_with = "no_null", skip_serializing_if = "is_none")]
    pub render_as_table: Option<bool>,
    pub has_header: bool,
    #[serde(default, deserialize_with = "no_null", skip_serializing_if = "is_none")]
    pub classification: Option<TableClassificationSummary>,
    #[serde(default, deserialize_with = "no_null", skip_serializing_if = "is_none")]
    pub source_id: Option<String>,
    #[serde(default, deserialize_with = "no_null", skip_serializing_if = "is_none")]
    pub regions: Option<Vec<BoundingBox>>,
    #[serde(default, deserialize_with = "no_null", skip_serializing_if = "is_none")]
    pub caption: Option<String>,
    #[serde(default, deserialize_with = "no_null", skip_serializing_if = "is_none")]
    pub caption_blocks: Option<Vec<IrBlock>>,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct IrBlock {
    #[serde(rename = "type")]
    pub kind: IrBlockType,
    #[serde(default, deserialize_with = "no_null", skip_serializing_if = "is_none")]
    pub text: Option<String>,
    #[serde(default, deserialize_with = "no_null", skip_serializing_if = "is_none")]
    pub table: Option<IrTable>,
    #[serde(
        default,
        deserialize_with = "deserialize_optional_u32",
        skip_serializing_if = "is_none"
    )]
    pub level: Option<u32>,
    #[serde(
        default,
        deserialize_with = "deserialize_optional_u32",
        skip_serializing_if = "is_none"
    )]
    pub page_number: Option<u32>,
    #[serde(default, deserialize_with = "no_null", skip_serializing_if = "is_none")]
    pub bbox: Option<BoundingBox>,
    #[serde(default, deserialize_with = "no_null", skip_serializing_if = "is_none")]
    pub style: Option<InlineStyle>,
    #[serde(default, deserialize_with = "no_null", skip_serializing_if = "is_none")]
    pub list_type: Option<ListType>,
    #[serde(default, deserialize_with = "no_null", skip_serializing_if = "is_none")]
    pub children: Option<Vec<IrBlock>>,
    #[serde(default, deserialize_with = "no_null", skip_serializing_if = "is_none")]
    pub href: Option<String>,
    #[serde(default, deserialize_with = "no_null", skip_serializing_if = "is_none")]
    pub footnote_text: Option<String>,
    #[serde(default, deserialize_with = "no_null", skip_serializing_if = "is_none")]
    pub image_data: Option<ImageData>,
    #[serde(default, deserialize_with = "no_null", skip_serializing_if = "is_none")]
    pub spans: Option<Vec<IrSpan>>,
    #[serde(default, deserialize_with = "no_null", skip_serializing_if = "is_none")]
    pub quote: Option<bool>,
    #[serde(default, deserialize_with = "no_null", skip_serializing_if = "is_none")]
    pub indent: Option<f64>,
    #[serde(
        default,
        deserialize_with = "deserialize_optional_u32",
        skip_serializing_if = "is_none"
    )]
    pub list_depth: Option<u32>,
}

impl IrBlock {
    pub fn paragraph(text: impl Into<String>) -> Self {
        Self {
            kind: IrBlockType::Paragraph,
            text: Some(text.into()),
            ..Self::default()
        }
    }
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct DocumentMetadata {
    #[serde(default, deserialize_with = "no_null", skip_serializing_if = "is_none")]
    pub title: Option<String>,
    #[serde(default, deserialize_with = "no_null", skip_serializing_if = "is_none")]
    pub author: Option<String>,
    #[serde(default, deserialize_with = "no_null", skip_serializing_if = "is_none")]
    pub creator: Option<String>,
    #[serde(default, deserialize_with = "no_null", skip_serializing_if = "is_none")]
    pub created_at: Option<String>,
    #[serde(default, deserialize_with = "no_null", skip_serializing_if = "is_none")]
    pub modified_at: Option<String>,
    #[serde(
        default,
        deserialize_with = "deserialize_optional_u32",
        skip_serializing_if = "is_none"
    )]
    pub page_count: Option<u32>,
    #[serde(default, deserialize_with = "no_null", skip_serializing_if = "is_none")]
    pub page_mode: Option<PageMode>,
    #[serde(default, deserialize_with = "no_null", skip_serializing_if = "is_none")]
    pub version: Option<String>,
    #[serde(default, deserialize_with = "no_null", skip_serializing_if = "is_none")]
    pub description: Option<String>,
    #[serde(default, deserialize_with = "no_null", skip_serializing_if = "is_none")]
    pub keywords: Option<Vec<String>>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ParseWarning {
    #[serde(
        default,
        deserialize_with = "deserialize_optional_u32",
        skip_serializing_if = "is_none"
    )]
    pub page: Option<u32>,
    pub message: String,
    pub code: WarningCode,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct OutlineItem {
    #[serde(deserialize_with = "deserialize_u32")]
    pub level: u32,
    pub text: String,
    #[serde(
        default,
        deserialize_with = "deserialize_optional_u32",
        skip_serializing_if = "is_none"
    )]
    pub page_number: Option<u32>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PageMarkdown {
    #[serde(deserialize_with = "deserialize_u32")]
    pub page_number: u32,
    pub markdown: String,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ChunkGranularity {
    Block,
    #[default]
    Section,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum DocChunkType {
    Text,
    Table,
    Heading,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct DocChunkTable {
    #[serde(deserialize_with = "deserialize_u32")]
    pub rows: u32,
    #[serde(deserialize_with = "deserialize_u32")]
    pub cols: u32,
    #[serde(default, deserialize_with = "no_null", skip_serializing_if = "is_none")]
    pub cells: Option<Vec<Vec<String>>>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct DocChunk {
    pub id: String,
    #[serde(rename = "type")]
    pub kind: DocChunkType,
    pub breadcrumb: Vec<String>,
    pub text: String,
    #[serde(
        default,
        deserialize_with = "deserialize_optional_u32",
        skip_serializing_if = "is_none"
    )]
    pub page: Option<u32>,
    pub block_range: [u32; 2],
    #[serde(default, deserialize_with = "no_null", skip_serializing_if = "is_none")]
    pub table: Option<DocChunkTable>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ChunkOptions {
    #[serde(default, deserialize_with = "no_null", skip_serializing_if = "is_none")]
    pub include_table_cells: Option<bool>,
    #[serde(default, deserialize_with = "no_null", skip_serializing_if = "is_none")]
    pub granularity: Option<ChunkGranularity>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ClassifyContext {
    #[serde(default, deserialize_with = "no_null", skip_serializing_if = "is_none")]
    pub nearby_text: Option<Vec<String>>,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum TableRepresentation {
    #[default]
    Gfm,
    Html,
    Visual,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ExtractedImage {
    pub filename: String,
    pub data: Vec<u8>,
    pub mime_type: String,
    #[serde(default, deserialize_with = "no_null", skip_serializing_if = "is_none")]
    pub source: Option<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PageQuality {
    #[serde(deserialize_with = "deserialize_u32")]
    pub page: u32,
    #[serde(deserialize_with = "deserialize_u32")]
    pub text_chars: u32,
    pub hangul_ratio: f64,
    pub control_char_ratio: f64,
    pub replacement_char_ratio: f64,
    pub pua_ratio: f64,
    pub needs_ocr: bool,
    #[serde(default, deserialize_with = "no_null", skip_serializing_if = "is_none")]
    pub ocr_reason: Option<OcrReason>,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct DocumentQualitySummary {
    #[serde(deserialize_with = "deserialize_u32")]
    pub total_pages: u32,
    #[serde(deserialize_with = "deserialize_u32")]
    pub total_text_chars: u32,
    pub avg_hangul_ratio: f64,
    pub avg_control_char_ratio: f64,
    pub avg_replacement_char_ratio: f64,
    pub avg_pua_ratio: f64,
    #[serde(deserialize_with = "deserialize_u32")]
    pub low_text_page_count: u32,
    #[serde(deserialize_with = "deserialize_u32")]
    pub high_pua_page_count: u32,
    pub needs_ocr: bool,
    #[serde(deserialize_with = "deserialize_u32_vec")]
    pub ocr_candidate_pages: Vec<u32>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ParseSuccess {
    pub file_type: FileType,
    #[serde(
        default,
        deserialize_with = "deserialize_optional_u32",
        skip_serializing_if = "is_none"
    )]
    pub page_count: Option<u32>,
    #[serde(default, deserialize_with = "no_null", skip_serializing_if = "is_none")]
    pub is_image_based: Option<bool>,
    #[serde(
        serialize_with = "serialize_true",
        deserialize_with = "deserialize_true"
    )]
    pub success: bool,
    pub markdown: String,
    pub blocks: Vec<IrBlock>,
    #[serde(default, deserialize_with = "no_null", skip_serializing_if = "is_none")]
    pub metadata: Option<DocumentMetadata>,
    #[serde(default, deserialize_with = "no_null", skip_serializing_if = "is_none")]
    pub outline: Option<Vec<OutlineItem>>,
    #[serde(default, deserialize_with = "no_null", skip_serializing_if = "is_none")]
    pub warnings: Option<Vec<ParseWarning>>,
    #[serde(default, deserialize_with = "no_null", skip_serializing_if = "is_none")]
    pub images: Option<Vec<ExtractedImage>>,
    #[serde(default, deserialize_with = "no_null", skip_serializing_if = "is_none")]
    pub pages: Option<Vec<PageMarkdown>>,
    #[serde(default, deserialize_with = "no_null", skip_serializing_if = "is_none")]
    pub page_quality: Option<Vec<PageQuality>>,
    #[serde(default, deserialize_with = "no_null", skip_serializing_if = "is_none")]
    pub quality_summary: Option<DocumentQualitySummary>,
}

impl Default for ParseSuccess {
    fn default() -> Self {
        Self {
            file_type: FileType::default(),
            page_count: None,
            is_image_based: None,
            success: true,
            markdown: String::new(),
            blocks: Vec::new(),
            metadata: None,
            outline: None,
            warnings: None,
            images: None,
            pages: None,
            page_quality: None,
            quality_summary: None,
        }
    }
}

fn deserialize_true<'de, D>(deserializer: D) -> Result<bool, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let value = bool::deserialize(deserializer)?;
    if value {
        Ok(true)
    } else {
        Err(serde::de::Error::custom("success must be true"))
    }
}

fn serialize_true<S>(_: &bool, serializer: S) -> Result<S::Ok, S::Error>
where
    S: serde::Serializer,
{
    serializer.serialize_bool(true)
}

impl ParseSuccess {
    pub fn new(file_type: FileType, markdown: impl Into<String>, blocks: Vec<IrBlock>) -> Self {
        Self {
            file_type,
            success: true,
            markdown: markdown.into(),
            blocks,
            ..Self::default()
        }
    }
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ParseFailure {
    pub file_type: FileType,
    #[serde(
        default,
        deserialize_with = "deserialize_optional_u32",
        skip_serializing_if = "is_none"
    )]
    pub page_count: Option<u32>,
    #[serde(default, deserialize_with = "no_null", skip_serializing_if = "is_none")]
    pub is_image_based: Option<bool>,
    #[serde(
        serialize_with = "serialize_false",
        deserialize_with = "deserialize_false"
    )]
    pub success: bool,
    pub error: String,
    #[serde(default, deserialize_with = "no_null", skip_serializing_if = "is_none")]
    pub code: Option<crate::ErrorCode>,
}

fn deserialize_false<'de, D>(deserializer: D) -> Result<bool, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let value = bool::deserialize(deserializer)?;
    if !value {
        Ok(false)
    } else {
        Err(serde::de::Error::custom("success must be false"))
    }
}

fn serialize_false<S>(_: &bool, serializer: S) -> Result<S::Ok, S::Error>
where
    S: serde::Serializer,
{
    serializer.serialize_bool(false)
}

impl ParseFailure {
    pub fn new(file_type: FileType, error: impl Into<String>) -> Self {
        Self {
            file_type,
            success: false,
            error: error.into(),
            ..Self::default()
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
// Keep direct public payloads so callers can construct `ParseResult::Success(value)` without boxing.
#[allow(clippy::large_enum_variant)]
pub enum ParseResult {
    Success(ParseSuccess),
    Failure(ParseFailure),
}
