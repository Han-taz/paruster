use kordoc_ir::{
    BoundingBox, DocumentMetadata, DocumentQualitySummary, ErrorCode, ExtractedImage, ImageData,
    InlineStyle, IrBlock, IrBlockType, IrCell, IrSpan, IrTable, KordocError, ListType, OcrReason,
    OutlineItem, PageMarkdown, PageMode, PageQuality, ParseFailure, ParseResult, ParseSuccess,
    ParseWarning, TableClassificationKind, TableClassificationReason, TableClassificationSummary,
    WarningCode,
};
use serde::Serialize;
use serde::de::DeserializeOwned;
use serde_json::Value;

fn assert_optional_fields_reject_null<T>(base: T, fields: &[&str])
where
    T: DeserializeOwned + Serialize,
{
    let base = serde_json::to_value(base).unwrap();
    assert!(
        serde_json::from_value::<T>(base.clone()).is_ok(),
        "omitted optional fields failed to default to None"
    );
    for field in fields {
        let mut candidate = base.clone();
        candidate[field] = Value::Null;
        assert!(
            serde_json::from_value::<T>(candidate).is_err(),
            "optional field {field} accepted explicit null"
        );
    }
}

#[test]
fn every_optional_wire_field_rejects_null_but_allows_omission() {
    assert_optional_fields_reject_null(
        IrSpan::default(),
        &[
            "bold",
            "italic",
            "strike",
            "underline",
            "code",
            "placeholder",
        ],
    );
    assert_optional_fields_reject_null(
        ImageData {
            data: vec![],
            mime_type: "image/png".into(),
            filename: None,
        },
        &["filename"],
    );
    assert_optional_fields_reject_null(
        InlineStyle::default(),
        &[
            "bold",
            "italic",
            "strike",
            "underline",
            "fontSize",
            "fontName",
        ],
    );
    assert_optional_fields_reject_null(IrCell::default(), &["blocks", "isHeader"]);
    assert_optional_fields_reject_null(
        IrTable::default(),
        &[
            "renderAsTable",
            "classification",
            "sourceId",
            "regions",
            "caption",
            "captionBlocks",
        ],
    );
    assert_optional_fields_reject_null(
        IrBlock::default(),
        &[
            "text",
            "table",
            "level",
            "pageNumber",
            "bbox",
            "style",
            "listType",
            "children",
            "href",
            "footnoteText",
            "imageData",
            "spans",
            "quote",
            "indent",
            "listDepth",
        ],
    );
    assert_optional_fields_reject_null(
        DocumentMetadata::default(),
        &[
            "title",
            "author",
            "creator",
            "createdAt",
            "modifiedAt",
            "pageCount",
            "pageMode",
            "version",
            "description",
            "keywords",
        ],
    );
    assert_optional_fields_reject_null(
        ParseWarning {
            page: None,
            message: "warning".into(),
            code: WarningCode::PartialParse,
        },
        &["page"],
    );
    assert_optional_fields_reject_null(
        OutlineItem {
            level: 1,
            text: "heading".into(),
            page_number: None,
        },
        &["pageNumber"],
    );
    assert_optional_fields_reject_null(
        ExtractedImage {
            filename: "image.png".into(),
            data: vec![],
            mime_type: "image/png".into(),
            source: None,
        },
        &["source"],
    );
    assert_optional_fields_reject_null(PageQuality::default(), &["ocrReason"]);

    assert_optional_fields_reject_null(
        ParseSuccess::new(kordoc_ir::FileType::Pdf, "", vec![]),
        &[
            "pageCount",
            "isImageBased",
            "metadata",
            "outline",
            "warnings",
            "images",
            "pages",
            "pageQuality",
            "qualitySummary",
        ],
    );
    assert_optional_fields_reject_null(
        ParseFailure::new(kordoc_ir::FileType::Unknown, "error"),
        &["pageCount", "isImageBased", "code"],
    );
}

#[test]
fn recursive_ir_roundtrips_without_loss() {
    let block = IrBlock {
        kind: IrBlockType::Table,
        table: Some(IrTable {
            rows: 1,
            cols: 1,
            has_header: false,
            cells: vec![vec![IrCell {
                text: "값".into(),
                col_span: 1,
                row_span: 1,
                blocks: Some(vec![IrBlock::paragraph("중첩")]),
                is_header: None,
            }]],
            caption: Some("표 1".into()),
            caption_blocks: Some(vec![IrBlock::paragraph("캡션")]),
            ..IrTable::default()
        }),
        spans: Some(vec![IrSpan {
            text: "값".into(),
            bold: Some(true),
            ..Default::default()
        }]),
        image_data: Some(ImageData {
            data: vec![0, 1, 2],
            mime_type: "image/png".into(),
            filename: None,
        }),
        ..IrBlock::default()
    };
    let json = serde_json::to_string(&block).unwrap();
    assert_eq!(serde_json::from_str::<IrBlock>(&json).unwrap(), block);
    assert!(json.contains("\"type\":\"table\""));
    assert!(!json.contains("\"kind\""));
    assert!(json.contains("\"data\":[0,1,2]"));
}

#[test]
fn optional_block_and_table_fields_use_exact_wire_names() {
    let table = IrTable {
        rows: 1,
        cols: 1,
        cells: vec![vec![IrCell {
            text: "cell".into(),
            col_span: 1,
            row_span: 1,
            blocks: None,
            is_header: Some(true),
        }]],
        render_as_table: Some(false),
        has_header: true,
        classification: Some(TableClassificationSummary {
            kind: TableClassificationKind::Uncertain,
            confidence: 0.5,
            semantic_score: 0.4,
            non_tabular_score: 0.6,
            reasons: vec![TableClassificationReason::AmbiguousScores],
        }),
        source_id: Some("source-1".into()),
        regions: Some(vec![BoundingBox {
            page: 2,
            x: 1.0,
            y: 2.0,
            width: 3.0,
            height: 4.0,
        }]),
        caption: None,
        caption_blocks: None,
    };
    let block = IrBlock {
        kind: IrBlockType::List,
        list_type: Some(ListType::Ordered),
        children: Some(vec![IrBlock::paragraph("child")]),
        href: Some("https://example.invalid".into()),
        footnote_text: Some("note".into()),
        quote: Some(true),
        indent: Some(2.0),
        list_depth: Some(3),
        table: Some(table),
        ..IrBlock::default()
    };
    let value = serde_json::to_value(block).unwrap();
    assert_eq!(value["type"], "list");
    assert_eq!(value["listType"], "ordered");
    assert_eq!(value["children"][0]["type"], "paragraph");
    assert_eq!(value["href"], "https://example.invalid");
    assert_eq!(value["footnoteText"], "note");
    assert_eq!(value["quote"], true);
    assert_eq!(value["indent"], 2.0);
    assert_eq!(value["listDepth"], 3);
    let table = &value["table"];
    assert_eq!(table["renderAsTable"], false);
    assert_eq!(table["classification"]["kind"], "uncertain");
    assert_eq!(table["sourceId"], "source-1");
    assert_eq!(table["regions"][0]["page"], 2);
    assert_eq!(table["cells"][0][0]["isHeader"], true);
    assert!(table.get("caption").is_none());
    assert!(table["cells"][0][0].get("blocks").is_none());
}

#[test]
fn block_serialization_uses_exact_keys_and_omits_unavailable_values() {
    let block = IrBlock::paragraph("hello");
    let value = serde_json::to_value(block).unwrap();
    assert_eq!(
        value,
        serde_json::json!({ "type": "paragraph", "text": "hello" })
    );
    assert!(
        serde_json::from_value::<IrBlock>(
            serde_json::json!({ "type": "paragraph", "kind": "paragraph" })
        )
        .is_err()
    );
    assert!(
        serde_json::from_value::<IrBlock>(
            serde_json::json!({ "type": "paragraph", "unknown": true })
        )
        .is_err()
    );
}

#[test]
fn warning_enum_uses_all_frozen_protocol_values() {
    let actual = [
        WarningCode::SkippedImage,
        WarningCode::SkippedOle,
        WarningCode::TruncatedTable,
        WarningCode::OcrFallback,
        WarningCode::UnsupportedElement,
        WarningCode::BrokenZipRecovery,
        WarningCode::HiddenTextFiltered,
        WarningCode::MalformedXml,
        WarningCode::PartialParse,
        WarningCode::LenientCfbRecovery,
        WarningCode::NeedsOcr,
        WarningCode::OcrFailed,
        WarningCode::OcrApplied,
        WarningCode::OcrLowConf,
        WarningCode::ComEmpty,
        WarningCode::DrmComFallback,
        WarningCode::PageBoundaryApproximate,
    ];
    let expected = [
        "SKIPPED_IMAGE",
        "SKIPPED_OLE",
        "TRUNCATED_TABLE",
        "OCR_FALLBACK",
        "UNSUPPORTED_ELEMENT",
        "BROKEN_ZIP_RECOVERY",
        "HIDDEN_TEXT_FILTERED",
        "MALFORMED_XML",
        "PARTIAL_PARSE",
        "LENIENT_CFB_RECOVERY",
        "NEEDS_OCR",
        "OCR_FAILED",
        "OCR_APPLIED",
        "OCR_LOW_CONF",
        "COM_EMPTY",
        "DRM_COM_FALLBACK",
        "PAGE_BOUNDARY_APPROXIMATE",
    ];
    let actual: Vec<String> = actual
        .iter()
        .map(|code| serde_json::to_string(code).unwrap())
        .collect();
    let expected: Vec<String> = expected.iter().map(|code| format!("\"{code}\"")).collect();
    assert_eq!(actual, expected);
}

#[test]
fn parse_success_and_failure_keep_the_discriminator_and_optional_fields() {
    let success = ParseSuccess::new(
        kordoc_ir::FileType::Pdf,
        "body",
        vec![IrBlock::paragraph("body")],
    );
    let success_json = serde_json::to_value(ParseResult::Success(success)).unwrap();
    assert_eq!(success_json["success"], true);
    assert_eq!(success_json["fileType"], "pdf");
    assert_eq!(success_json["markdown"], "body");
    assert!(success_json.get("metadata").is_none());
    assert_eq!(
        serde_json::to_value(ParseSuccess::default()).unwrap()["success"],
        true
    );

    let failure = ParseFailure {
        file_type: kordoc_ir::FileType::Unknown,
        error: "not supported".into(),
        code: Some(ErrorCode::UnsupportedFormat),
        ..ParseFailure::default()
    };
    let failure_json = serde_json::to_value(ParseResult::Failure(failure)).unwrap();
    assert_eq!(failure_json["success"], false);
    assert_eq!(failure_json["fileType"], "unknown");
    assert_eq!(failure_json["code"], "UNSUPPORTED_FORMAT");
    assert!(failure_json.get("pageCount").is_none());

    assert!(serde_json::from_value::<ParseResult>(success_json).is_ok());
    assert!(serde_json::from_value::<ParseResult>(failure_json).is_ok());
    assert!(
        serde_json::from_value::<ParseResult>(serde_json::json!({
            "success": false,
            "fileType": "pdf",
            "markdown": "body",
            "blocks": []
        }))
        .is_err()
    );
    assert!(
        serde_json::from_value::<ParseResult>(serde_json::json!({
            "success": true,
            "fileType": "pdf",
            "error": "bad"
        }))
        .is_err()
    );
}

#[test]
fn error_codes_match_the_canonical_inventory_and_serialize_as_protocol_names() {
    let canonical: serde_json::Value =
        serde_json::from_str(include_str!("../../../contracts/errors.json"))
            .expect("canonical errors inventory is valid JSON");
    let expected: Vec<&str> = canonical["codes"]
        .as_array()
        .unwrap()
        .iter()
        .map(|entry| entry["code"].as_str().unwrap())
        .collect();
    assert_eq!(ErrorCode::ALL.len(), expected.len());
    let actual: Vec<String> = ErrorCode::ALL
        .iter()
        .map(|code| {
            serde_json::to_value(code)
                .unwrap()
                .as_str()
                .unwrap()
                .to_owned()
        })
        .collect();
    assert_eq!(actual, expected);
    assert_eq!(
        serde_json::to_string(&ErrorCode::EmptyInput).unwrap(),
        "\"EMPTY_INPUT\""
    );

    let error = KordocError::new(ErrorCode::EmptyInput, "empty input");
    assert_eq!(error.code, ErrorCode::EmptyInput);
    assert_eq!(error.to_string(), "EMPTY_INPUT: empty input");
    assert_eq!(
        serde_json::to_value(error).unwrap(),
        serde_json::json!({ "code": "EMPTY_INPUT", "message": "empty input" })
    );
}

#[test]
fn parse_projection_types_serialize_with_camel_case_keys() {
    let image = ExtractedImage {
        filename: "a.png".into(),
        data: vec![255, 0],
        mime_type: "image/png".into(),
        source: None,
    };
    assert_eq!(
        serde_json::to_value(image).unwrap(),
        serde_json::json!({ "filename": "a.png", "data": [255, 0], "mimeType": "image/png" })
    );
    assert_eq!(
        serde_json::to_value(OutlineItem {
            level: 2,
            text: "heading".into(),
            page_number: Some(3)
        })
        .unwrap(),
        serde_json::json!({ "level": 2, "text": "heading", "pageNumber": 3 })
    );
    assert_eq!(
        serde_json::to_value(PageMarkdown {
            page_number: 1,
            markdown: "page".into()
        })
        .unwrap(),
        serde_json::json!({ "pageNumber": 1, "markdown": "page" })
    );
    let metadata = DocumentMetadata {
        page_count: Some(2),
        page_mode: Some(kordoc_ir::PageMode::Layout),
        ..DocumentMetadata::default()
    };
    let metadata_json = serde_json::to_value(metadata).unwrap();
    assert_eq!(
        metadata_json,
        serde_json::json!({ "pageCount": 2, "pageMode": "layout" })
    );

    let bbox = BoundingBox {
        page: 1,
        x: 2.0,
        y: 3.0,
        width: 4.0,
        height: 5.0,
    };
    assert_eq!(serde_json::to_value(bbox).unwrap()["width"], 4.0);
    let style = InlineStyle {
        font_size: Some(12.0),
        font_name: Some("serif".into()),
        ..Default::default()
    };
    assert_eq!(
        serde_json::to_value(style).unwrap(),
        serde_json::json!({ "fontSize": 12.0, "fontName": "serif" })
    );

    let warning = ParseWarning {
        message: "recovered".into(),
        code: WarningCode::PartialParse,
        page: Some(1),
    };
    assert_eq!(
        serde_json::to_value(warning).unwrap(),
        serde_json::json!({ "page": 1, "message": "recovered", "code": "PARTIAL_PARSE" })
    );

    let quality = PageQuality {
        page: 1,
        text_chars: 42,
        hangul_ratio: 0.8,
        control_char_ratio: 0.01,
        replacement_char_ratio: 0.02,
        pua_ratio: 0.03,
        needs_ocr: true,
        ocr_reason: Some(OcrReason::LowText),
    };
    assert_eq!(
        serde_json::to_value(quality).unwrap(),
        serde_json::json!({
            "page": 1,
            "textChars": 42,
            "hangulRatio": 0.8,
            "controlCharRatio": 0.01,
            "replacementCharRatio": 0.02,
            "puaRatio": 0.03,
            "needsOcr": true,
            "ocrReason": "low_text"
        })
    );
    let summary = DocumentQualitySummary {
        total_pages: 1,
        total_text_chars: 42,
        avg_hangul_ratio: 0.8,
        avg_control_char_ratio: 0.01,
        avg_replacement_char_ratio: 0.02,
        avg_pua_ratio: 0.03,
        low_text_page_count: 1,
        high_pua_page_count: 0,
        needs_ocr: true,
        ocr_candidate_pages: vec![1],
    };
    assert_eq!(
        serde_json::to_value(summary).unwrap(),
        serde_json::json!({
            "totalPages": 1,
            "totalTextChars": 42,
            "avgHangulRatio": 0.8,
            "avgControlCharRatio": 0.01,
            "avgReplacementCharRatio": 0.02,
            "avgPuaRatio": 0.03,
            "lowTextPageCount": 1,
            "highPuaPageCount": 0,
            "needsOcr": true,
            "ocrCandidatePages": [1]
        })
    );
}

#[test]
fn all_other_wire_enums_use_the_schema_spellings() {
    let block_types = [
        (IrBlockType::Paragraph, "paragraph"),
        (IrBlockType::Table, "table"),
        (IrBlockType::Heading, "heading"),
        (IrBlockType::List, "list"),
        (IrBlockType::Image, "image"),
        (IrBlockType::Separator, "separator"),
    ];
    for (value, expected) in block_types {
        assert_eq!(serde_json::to_value(value).unwrap(), expected);
    }
    let file_types = [
        (kordoc_ir::FileType::Hwpx, "hwpx"),
        (kordoc_ir::FileType::Hwp, "hwp"),
        (kordoc_ir::FileType::Hwp3, "hwp3"),
        (kordoc_ir::FileType::Hwpml, "hwpml"),
        (kordoc_ir::FileType::Pdf, "pdf"),
        (kordoc_ir::FileType::Xlsx, "xlsx"),
        (kordoc_ir::FileType::Xls, "xls"),
        (kordoc_ir::FileType::Docx, "docx"),
        (kordoc_ir::FileType::Pptx, "pptx"),
        (kordoc_ir::FileType::Image, "image"),
        (kordoc_ir::FileType::Unknown, "unknown"),
    ];
    for (value, expected) in file_types {
        assert_eq!(serde_json::to_value(value).unwrap(), expected);
    }
    for (value, expected) in [
        (ListType::Ordered, "ordered"),
        (ListType::Unordered, "unordered"),
    ] {
        assert_eq!(serde_json::to_value(value).unwrap(), expected);
    }
    for (value, expected) in [(PageMode::Layout, "layout"), (PageMode::Section, "section")] {
        assert_eq!(serde_json::to_value(value).unwrap(), expected);
    }
    for (value, expected) in [
        (OcrReason::VectorText, "vector_text"),
        (OcrReason::LowText, "low_text"),
        (OcrReason::HighPua, "high_pua"),
        (OcrReason::HighControl, "high_control"),
        (OcrReason::HighReplacement, "high_replacement"),
        (OcrReason::GarbledHangul, "garbled_hangul"),
    ] {
        assert_eq!(serde_json::to_value(value).unwrap(), expected);
    }

    let classification = TableClassificationSummary {
        kind: TableClassificationKind::SemanticTable,
        confidence: 0.9,
        semantic_score: 0.8,
        non_tabular_score: 0.1,
        reasons: vec![TableClassificationReason::RepeatedRowSchema],
    };
    assert_eq!(
        serde_json::to_value(classification).unwrap(),
        serde_json::json!({
            "kind": "semantic-table",
            "confidence": 0.9,
            "semanticScore": 0.8,
            "nonTabularScore": 0.1,
            "reasons": ["repeated-row-schema"]
        })
    );
    for (value, expected) in [
        (TableClassificationKind::SemanticTable, "semantic-table"),
        (
            TableClassificationKind::NonTabularLayout,
            "non-tabular-layout",
        ),
        (TableClassificationKind::Uncertain, "uncertain"),
    ] {
        assert_eq!(serde_json::to_value(value).unwrap(), expected);
    }
    for (value, expected) in [
        (
            TableClassificationReason::RepeatedRowSchema,
            "repeated-row-schema",
        ),
        (TableClassificationReason::GridRegularity, "grid-regularity"),
        (
            TableClassificationReason::HighActiveDensity,
            "high-active-density",
        ),
        (
            TableClassificationReason::ColumnTypeConsistency,
            "column-type-consistency",
        ),
        (
            TableClassificationReason::NestedStructureWrapper,
            "nested-structure-wrapper",
        ),
        (
            TableClassificationReason::SpanIrregularity,
            "span-irregularity",
        ),
        (TableClassificationReason::SpacerBands, "spacer-bands"),
        (
            TableClassificationReason::ExtremeSparsity,
            "extreme-sparsity",
        ),
        (
            TableClassificationReason::DiagramContextKeyword,
            "diagram-context-keyword",
        ),
        (TableClassificationReason::LowEvidence, "low-evidence"),
        (
            TableClassificationReason::AmbiguousScores,
            "ambiguous-scores",
        ),
    ] {
        assert_eq!(serde_json::to_value(value).unwrap(), expected);
    }
}
