use kordoc_ir::{
    DocumentMetadata, ExtractedImage, IrBlock, IrBlockType, IrCell, IrTable, PageEvidence,
    PageQuality, ParsedDocument,
};

#[test]
fn parsed_document_roundtrips_recursive_content_and_page_evidence() {
    let parsed = ParsedDocument {
        blocks: vec![
            IrBlock {
                children: Some(vec![IrBlock::paragraph("child")]),
                ..IrBlock::paragraph("body")
            },
            IrBlock {
                kind: IrBlockType::Table,
                table: Some(IrTable {
                    rows: 1,
                    cols: 1,
                    cells: vec![vec![IrCell {
                        text: "cell".into(),
                        col_span: 1,
                        row_span: 1,
                        blocks: Some(vec![IrBlock::paragraph("cell child")]),
                        ..IrCell::default()
                    }]],
                    has_header: false,
                    caption_blocks: Some(vec![IrBlock::paragraph("caption child")]),
                    ..IrTable::default()
                }),
                ..IrBlock::default()
            },
        ],
        page_count: Some(2),
        metadata: Some(DocumentMetadata {
            title: Some("fixture".into()),
            page_count: Some(9),
            ..DocumentMetadata::default()
        }),
        outline: None,
        warnings: Some(vec![]),
        images: Some(vec![ExtractedImage {
            filename: "image.png".into(),
            data: vec![0, 1, 255],
            mime_type: "image/png".into(),
            source: Some("Contents/image.png".into()),
        }]),
        is_image_based: Some(false),
        page_quality: Some(vec![PageQuality {
            page: 1,
            text_chars: 4,
            ..PageQuality::default()
        }]),
        quality_summary: None,
        page_evidence: Some(vec![
            PageEvidence { page_number: 1 },
            PageEvidence { page_number: 2 },
        ]),
    };

    let value = serde_json::to_value(&parsed).unwrap();
    assert!(value.get("markdown").is_none());
    assert_eq!(value["blocks"][0]["text"], "body");
    assert_eq!(value["blocks"][0]["children"][0]["text"], "child");
    assert_eq!(
        value["blocks"][1]["table"]["cells"][0][0]["blocks"][0]["text"],
        "cell child"
    );
    assert_eq!(
        value["blocks"][1]["table"]["captionBlocks"][0]["text"],
        "caption child"
    );
    assert_eq!(value["pageCount"], 2);
    assert_eq!(value["metadata"]["pageCount"], 9);
    assert_eq!(value["images"][0]["data"], serde_json::json!([0, 1, 255]));
    assert_eq!(value["pageEvidence"][1]["pageNumber"], 2);
    assert_eq!(
        serde_json::from_value::<ParsedDocument>(value).unwrap(),
        parsed
    );
}

#[test]
fn parse_options_keep_omitted_values_distinct_from_explicit_false() {
    use kordoc_ir::{OcrOption, PageNumber, PageSelection, ParseOptions};

    let omitted = ParseOptions::default();
    assert!(omitted.pages.is_none());
    assert!(omitted.plain.is_none());
    assert!(omitted.images.is_none());

    let explicit = ParseOptions {
        pages: Some(PageSelection::Numbers(vec![PageNumber::new(1.5).unwrap()])),
        ocr: Some(OcrOption::Force),
        remove_header_footer: Some(false),
        script_tags: Some(true),
        plain: Some(false),
        html_tables: Some(true),
        keep_trailing_empty_cols: Some(true),
        classify_tables: Some(true),
        keep_empty_paragraphs: Some(true),
        include_field_placeholders: Some(true),
        password: Some("secret".into()),
        formula_ocr: Some(true),
        dedupe_running_headers: Some(true),
        inline_images: Some(true),
        images: Some(false),
        tables: Some(false),
    };
    let Some(PageSelection::Numbers(numbers)) = explicit.pages else {
        panic!("numeric page selection was lost");
    };
    assert_eq!(numbers[0].get(), 1.5);
    assert_eq!(explicit.ocr, Some(OcrOption::Force));
    assert_eq!(explicit.remove_header_footer, Some(false));
    assert_eq!(explicit.script_tags, Some(true));
    assert_eq!(explicit.plain, Some(false));
    assert_eq!(explicit.html_tables, Some(true));
    assert_eq!(explicit.keep_trailing_empty_cols, Some(true));
    assert_eq!(explicit.classify_tables, Some(true));
    assert_eq!(explicit.keep_empty_paragraphs, Some(true));
    assert_eq!(explicit.include_field_placeholders, Some(true));
    assert_eq!(explicit.password.as_deref(), Some("secret"));
    assert_eq!(explicit.formula_ocr, Some(true));
    assert_eq!(explicit.dedupe_running_headers, Some(true));
    assert_eq!(explicit.inline_images, Some(true));
    assert_eq!(explicit.images, Some(false));
    assert_eq!(explicit.tables, Some(false));
}

#[test]
fn parsed_document_omits_unavailable_optional_evidence() {
    let parsed = ParsedDocument {
        blocks: vec![],
        page_count: None,
        metadata: None,
        outline: None,
        warnings: None,
        images: None,
        is_image_based: None,
        page_quality: None,
        quality_summary: None,
        page_evidence: None,
    };

    let value = serde_json::to_value(parsed).unwrap();
    for field in [
        "metadata",
        "pageCount",
        "outline",
        "warnings",
        "images",
        "isImageBased",
        "pageQuality",
        "qualitySummary",
        "pageEvidence",
        "markdown",
    ] {
        assert!(value.get(field).is_none(), "unexpected field {field}");
    }
}

#[test]
fn parsed_document_rejects_null_for_optional_page_count() {
    let mut value = serde_json::to_value(ParsedDocument::default()).unwrap();
    value["pageCount"] = serde_json::Value::Null;

    assert!(serde_json::from_value::<ParsedDocument>(value).is_err());
}

#[test]
fn parsed_document_rejects_unknown_fields() {
    let value = serde_json::json!({ "blocks": [], "markdown": "projection belongs to core" });

    assert!(serde_json::from_value::<ParsedDocument>(value).is_err());
}
