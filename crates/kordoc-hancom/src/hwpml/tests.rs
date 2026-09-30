use super::{MAX_HWPML_BYTES, check_input_size, parse, parse_with_budget};
use crate::hwpx::budget::LoweringBudget;
use kordoc_ir::{
    ErrorCode, IrBlock, IrBlockType, OutlineItem, PageNumber, PageSelection, ParseOptions,
};

const NORMAL: &[u8] =
    include_bytes!("../../tests/support/hwpml/text_fixture_inputs/normal_metadata_styles.xml");
const EMPTY_BODY: &[u8] =
    include_bytes!("../../tests/support/hwpml/text_fixture_inputs/empty_body.xml");
const EMPTY_SECTIONS: &[u8] =
    include_bytes!("../../tests/support/hwpml/text_fixture_inputs/empty_sections.xml");
const NESTED_TABLE: &[u8] =
    include_bytes!("../../tests/support/hwpml/table_fixture_inputs/nested_table.xml");
const HWPML_CAPTURES: &str = include_str!("../../tests/support/hwpml/hwpml-oracle-results.jsonl");

#[test]
fn unique_anchor_nested_table_matches_all_three_pinned_capture_blocks() {
    let cases = [
        ("nested_table_default", None),
        ("nested_table_keep_trailing_empty_cols", Some(true)),
        ("nested_table_drop_trailing_empty_cols", Some(false)),
    ];

    for (case_id, keep_trailing_empty_cols) in cases {
        let options = ParseOptions {
            keep_trailing_empty_cols,
            ..ParseOptions::default()
        };
        let parsed = parse(NESTED_TABLE, &options)
            .unwrap_or_else(|error| panic!("{case_id} should parse: {error}"));
        let capture = HWPML_CAPTURES
            .lines()
            .filter_map(|line| serde_json::from_str::<serde_json::Value>(line).ok())
            .find(|entry| entry["case_id"] == case_id)
            .unwrap_or_else(|| panic!("missing pinned capture {case_id}"));
        assert_eq!(
            serde_json::to_value(&parsed.blocks).unwrap(),
            capture["result"]["blocks"],
            "complete IR block tree differs for {case_id}"
        );
        assert_eq!(parsed.blocks.len(), 2, "{case_id}");
        let table = parsed.blocks[1].table.as_ref().expect("table block");
        assert_eq!(table.rows, 2, "{case_id}");
        assert_eq!(
            table.cols,
            if keep_trailing_empty_cols == Some(true) {
                3
            } else {
                2
            },
            "{case_id}"
        );
        assert_eq!(parsed.blocks[0].text.as_deref(), Some("표 앞 문단"));
    }
}

#[test]
fn aggregate_table_budget_path_handles_a_nested_table_and_a_sibling_table() {
    let mut xml = String::from(std::str::from_utf8(NESTED_TABLE).unwrap());
    let close = "</SECTION>";
    let index = xml.find(close).unwrap();
    xml.insert_str(
        index,
        "<TABLE RowCount=\"1\" ColCount=\"1\"><ROW><CELL RowAddr=\"0\" ColAddr=\"0\"><PARALIST><P><TEXT><CHAR>별도 표</CHAR></TEXT></P></PARALIST></CELL></ROW></TABLE>",
    );
    let parsed = parse(xml.as_bytes(), &ParseOptions::default()).unwrap();
    assert_eq!(parsed.blocks.len(), 3);
    assert_eq!(
        parsed.blocks[1].table.as_ref().unwrap().cells[0][1]
            .blocks
            .as_ref()
            .unwrap()
            .len(),
        2
    );
    assert_eq!(
        parsed.blocks[2].table.as_ref().unwrap().cells[0][0].text,
        "별도 표"
    );
}

#[test]
fn nested_table_rejects_depth_nine_captions_bad_addresses_and_ambiguous_anchors() {
    fn table_chain(levels: usize) -> String {
        let mut nested = "<TABLE RowCount=\"1\" ColCount=\"1\"><ROW><CELL RowAddr=\"0\" ColAddr=\"0\"><PARALIST><P><TEXT><CHAR>leaf</CHAR></TEXT></P></PARALIST></CELL></ROW></TABLE>".to_owned();
        for _ in 1..levels {
            nested = format!(
                "<TABLE RowCount=\"1\" ColCount=\"1\"><ROW><CELL RowAddr=\"0\" ColAddr=\"0\"><PARALIST><P><TEXT><CHAR>parent</CHAR>{nested}</TEXT></P></PARALIST></CELL></ROW></TABLE>"
            );
        }
        format!("<HWPML><BODY><SECTION>{nested}</SECTION></BODY></HWPML>")
    }
    let depth_nine = table_chain(9);
    assert_eq!(
        parse(depth_nine.as_bytes(), &ParseOptions::default())
            .unwrap_err()
            .code,
        ErrorCode::UnsupportedFormat
    );
    let depth_eight = table_chain(8);
    assert_eq!(
        parse(depth_eight.as_bytes(), &ParseOptions::default())
            .unwrap()
            .blocks[0]
            .table
            .as_ref()
            .unwrap()
            .rows,
        1
    );

    let caption = br#"<HWPML><BODY><SECTION><TABLE RowCount="1" ColCount="1"><CAPTION>caption</CAPTION><ROW><CELL RowAddr="0" ColAddr="0"/></ROW></TABLE></SECTION></BODY></HWPML>"#;
    assert_eq!(
        parse(caption, &ParseOptions::default()).unwrap_err().code,
        ErrorCode::UnsupportedFormat
    );

    let no_cells = br#"<HWPML><BODY><SECTION><TABLE RowCount="1" ColCount="1"><ROW/></TABLE></SECTION></BODY></HWPML>"#;
    assert_eq!(
        parse(no_cells, &ParseOptions::default()).unwrap_err().code,
        ErrorCode::UnsupportedFormat
    );

    let bad_address = br#"<HWPML><BODY><SECTION><TABLE RowCount="1" ColCount="1"><ROW><CELL RowAddr="1" ColAddr="0"/></ROW></TABLE></SECTION></BODY></HWPML>"#;
    assert_eq!(
        parse(bad_address, &ParseOptions::default())
            .unwrap_err()
            .code,
        ErrorCode::UnsupportedFormat
    );

    let bad_dimension = br#"<HWPML><BODY><SECTION><TABLE RowCount="1oops" ColCount="1"><ROW><CELL RowAddr="0" ColAddr="0"/></ROW></TABLE></SECTION></BODY></HWPML>"#;
    assert_eq!(
        parse(bad_dimension, &ParseOptions::default())
            .unwrap_err()
            .code,
        ErrorCode::UnsupportedFormat
    );

    let ambiguous = br#"<HWPML><BODY><SECTION><TABLE RowCount="1" ColCount="1"><ROW><CELL RowAddr="0" ColAddr="0"><PARALIST><P><TEXT><CHAR>anchor</CHAR><TABLE RowCount="1" ColCount="1"><ROW><CELL RowAddr="0" ColAddr="0"><PARALIST><P><TEXT><CHAR>inner</CHAR></TEXT></P></PARALIST></CELL></ROW></TABLE></TEXT></P></PARALIST></CELL><CELL RowAddr="0" ColAddr="0"><PARALIST><P><TEXT><CHAR>collision</CHAR></TEXT></P></PARALIST></CELL></ROW></TABLE></SECTION></BODY></HWPML>"#;
    assert_eq!(
        parse(ambiguous, &ParseOptions::default()).unwrap_err().code,
        ErrorCode::UnsupportedFormat
    );
}

#[test]
fn nested_tables_inside_source_skipped_wrappers_fail_closed() {
    let paragraph_note = br#"<HWPML><BODY><SECTION><P><TEXT><FOOTNOTE><TEXT><CHAR>note</CHAR><TABLE RowCount="1" ColCount="1"><ROW><CELL RowAddr="0" ColAddr="0"/></ROW></TABLE></TEXT></FOOTNOTE></TEXT></P></SECTION></BODY></HWPML>"#;
    assert_eq!(
        parse(paragraph_note, &ParseOptions::default())
            .unwrap_err()
            .code,
        ErrorCode::UnsupportedFormat
    );

    let cell_header = br#"<HWPML><BODY><SECTION><TABLE RowCount="1" ColCount="1"><ROW><CELL RowAddr="0" ColAddr="0"><PARALIST><P><TEXT><CHAR>outer</CHAR><TABLE RowCount="1" ColCount="1"><ROW><CELL RowAddr="0" ColAddr="0"><PARALIST><P><TEXT><HEADER><TABLE RowCount="1" ColCount="1"><ROW><CELL RowAddr="0" ColAddr="0"/></ROW></TABLE></HEADER><CHAR>inner</CHAR></TEXT></P></PARALIST></CELL></ROW></TABLE></TEXT></P></PARALIST></CELL></ROW></TABLE></SECTION></BODY></HWPML>"#;
    assert_eq!(
        parse(cell_header, &ParseOptions::default())
            .unwrap_err()
            .code,
        ErrorCode::UnsupportedFormat
    );
}

#[test]
fn table_lowering_uses_existing_allocation_budget_and_no_table_behavior_is_unchanged() {
    let empty_table = br#"<HWPML><BODY><SECTION><TABLE RowCount="1" ColCount="1"><ROW><CELL RowAddr="0" ColAddr="0"/></ROW></TABLE></SECTION></BODY></HWPML>"#;
    let mut budget = LoweringBudget::with_limit(0);
    assert_eq!(
        parse_with_budget(empty_table, &ParseOptions::default(), &mut budget)
            .unwrap_err()
            .code,
        ErrorCode::OutputTooLarge
    );

    let oversized_child_text = "x".repeat(20_000);
    let invalid_geometry_with_child_text = format!(
        "<HWPML><BODY><SECTION><TABLE RowCount=\"1\" ColCount=\"201\"><ROW><CELL RowAddr=\"0\" ColAddr=\"200\"><PARALIST><P><TEXT><CHAR>{oversized_child_text}</CHAR><TABLE RowCount=\"1\" ColCount=\"1\"><ROW><CELL RowAddr=\"0\" ColAddr=\"0\"><PARALIST><P><TEXT><CHAR>nested</CHAR></TEXT></P></PARALIST></CELL></ROW></TABLE></TEXT></P></PARALIST></CELL></ROW></TABLE></SECTION></BODY></HWPML>"
    );
    let mut limited_budget = LoweringBudget::with_limit(1024);
    let error = parse_with_budget(
        invalid_geometry_with_child_text.as_bytes(),
        &ParseOptions::default(),
        &mut limited_budget,
    )
    .unwrap_err();
    assert_eq!(error.code, ErrorCode::OutputTooLarge);
    assert_eq!(limited_budget.checkpoint(), 0);

    let parsed = parse(NORMAL, &ParseOptions::default()).unwrap();
    assert_eq!(parsed.blocks.len(), 4);
    assert!(parsed.blocks.iter().all(|block| block.table.is_none()));
}

#[test]
fn normal_fixture_preserves_text_metadata_headings_and_pages() {
    let parsed = parse(NORMAL, &ParseOptions::default()).unwrap();

    assert_eq!(parsed.blocks.len(), 4);
    assert_eq!(parsed.blocks[0].kind, IrBlockType::Heading);
    assert_eq!(parsed.blocks[0].text.as_deref(), Some("첫 제목"));
    assert_eq!(parsed.blocks[0].level, Some(1));
    assert_eq!(parsed.blocks[0].page_number, Some(1));
    assert_eq!(parsed.blocks[1].kind, IrBlockType::Paragraph);
    assert_eq!(
        parsed.blocks[1].text.as_deref(),
        Some("앞\u{00a0}뒤 & 끝각주내용은문단에포함")
    );
    assert_eq!(parsed.blocks[1].page_number, Some(1));
    assert_eq!(parsed.blocks[2].kind, IrBlockType::Heading);
    assert_eq!(parsed.blocks[2].text.as_deref(), Some("둘째 구역 제목"));
    assert_eq!(parsed.blocks[2].level, Some(6));
    assert_eq!(parsed.blocks[2].page_number, Some(2));
    assert_eq!(parsed.blocks[3].kind, IrBlockType::Paragraph);
    assert_eq!(parsed.blocks[3].text.as_deref(), Some("둘째 구역 본문"));
    assert_eq!(parsed.blocks[3].page_number, Some(2));

    let metadata = parsed.metadata.unwrap();
    assert_eq!(metadata.title.as_deref(), Some("합성 HWPML 제목"));
    assert_eq!(metadata.author.as_deref(), Some("Open Fixture"));
    assert_eq!(metadata.created_at.as_deref(), Some("2026-09-30"));
    assert!(parsed.page_count.is_none());
    assert!(parsed.page_evidence.is_none());

    let outline = parsed.outline.unwrap();
    assert_eq!(outline.len(), 2);
    assert_eq!(
        (
            outline[0].level,
            outline[0].text.as_str(),
            outline[0].page_number
        ),
        (1, "첫 제목", Some(1))
    );
    assert_eq!(
        (
            outline[1].level,
            outline[1].text.as_str(),
            outline[1].page_number
        ),
        (6, "둘째 구역 제목", Some(2))
    );
}

#[test]
fn page_selection_keeps_original_section_ordinals_and_metadata() {
    let options = ParseOptions {
        pages: Some(PageSelection::Numbers(vec![PageNumber::new(2.0).unwrap()])),
        ..ParseOptions::default()
    };
    let parsed = parse(NORMAL, &options).unwrap();

    assert_eq!(parsed.blocks.len(), 2);
    assert!(
        parsed
            .blocks
            .iter()
            .all(|block| block.page_number == Some(2))
    );
    assert_eq!(parsed.blocks[0].text.as_deref(), Some("둘째 구역 제목"));
    assert_eq!(parsed.blocks[1].text.as_deref(), Some("둘째 구역 본문"));
    assert_eq!(parsed.outline.unwrap()[0].page_number, Some(2));
    assert_eq!(
        parsed.metadata.unwrap().title.as_deref(),
        Some("합성 HWPML 제목")
    );
}

#[test]
fn page_range_selection_matches_number_selection() {
    let options = ParseOptions {
        pages: Some(PageSelection::Range("1,2".to_owned())),
        ..ParseOptions::default()
    };
    let parsed = parse(NORMAL, &options).unwrap();
    assert_eq!(parsed.blocks.len(), 4);
    assert_eq!(parsed.blocks[0].page_number, Some(1));
    assert_eq!(parsed.blocks[2].page_number, Some(2));
}

#[test]
fn page_selection_preserves_fractional_rounding_and_range_syntax() {
    let fractional = ParseOptions {
        pages: Some(PageSelection::Numbers(vec![PageNumber::new(1.5).unwrap()])),
        ..ParseOptions::default()
    };
    let parsed = parse(NORMAL, &fractional).unwrap();
    assert_eq!(parsed.blocks.len(), 2);
    assert!(
        parsed
            .blocks
            .iter()
            .all(|block| block.page_number == Some(2))
    );

    let malformed_prefix = ParseOptions {
        pages: Some(PageSelection::Range("2abc".to_owned())),
        ..ParseOptions::default()
    };
    let parsed = parse(NORMAL, &malformed_prefix).unwrap();
    assert_eq!(parsed.blocks.len(), 2);
    assert!(
        parsed
            .blocks
            .iter()
            .all(|block| block.page_number == Some(2))
    );

    let reversed_range = ParseOptions {
        pages: Some(PageSelection::Range("2-1".to_owned())),
        ..ParseOptions::default()
    };
    assert!(parse(NORMAL, &reversed_range).unwrap().blocks.is_empty());
}

#[test]
fn empty_body_and_empty_sections_return_no_blocks_or_page_evidence() {
    let empty_body = parse(EMPTY_BODY, &ParseOptions::default()).unwrap();
    assert!(empty_body.blocks.is_empty());
    assert!(empty_body.outline.is_none());
    assert!(empty_body.page_evidence.is_none());
    assert_eq!(
        empty_body.metadata.unwrap().title.as_deref(),
        Some("빈 본문 문서")
    );

    let empty_sections = parse(EMPTY_SECTIONS, &ParseOptions::default()).unwrap();
    assert!(empty_sections.blocks.is_empty());
    assert!(empty_sections.outline.is_none());
    assert!(empty_sections.metadata.is_none());
    assert!(empty_sections.page_evidence.is_none());
}

#[test]
fn skips_structures_only_when_the_source_walk_ignores_them() {
    let xml = br#"<HWPML><BODY><SECTION>
      <HEADER><P><TEXT><CHAR>hidden</CHAR><TABLE/></TEXT></P></HEADER>
      <FOOTER><P><TEXT><CHAR>hidden</CHAR></TEXT></P></FOOTER>
      <P><TEXT><CHAR>kept</CHAR><AUTONUM>number</AUTONUM><PICTURE><CHAR>image</CHAR></PICTURE><SHAPEOBJECT><CHAR>shape</CHAR></SHAPEOBJECT></TEXT></P>
    </SECTION></BODY></HWPML>"#;
    let parsed = parse(xml, &ParseOptions::default()).unwrap();
    assert_eq!(parsed.blocks.len(), 1);
    assert_eq!(parsed.blocks[0].text.as_deref(), Some("kept"));
}

#[test]
fn recurses_through_section_wrappers_but_filters_inline_wrapper_text() {
    for wrapper in ["PICTURE", "SHAPEOBJECT", "AUTONUM"] {
        let xml = format!(
            "<HWPML><BODY><SECTION><{wrapper}><P><TEXT><CHAR>hidden</CHAR></TEXT></P></{wrapper}><P><TEXT><CHAR>kept</CHAR></TEXT></P></SECTION></BODY></HWPML>"
        );
        let parsed = parse(xml.as_bytes(), &ParseOptions::default()).unwrap();
        assert_eq!(parsed.blocks.len(), 2, "wrapper {wrapper}");
        assert_eq!(
            parsed.blocks[0].text.as_deref(),
            Some("hidden"),
            "wrapper {wrapper}"
        );
        assert_eq!(
            parsed.blocks[1].text.as_deref(),
            Some("kept"),
            "wrapper {wrapper}"
        );
    }
}

#[test]
fn rejects_tables_in_semantically_selected_section_content() {
    let outside_paragraph = br#"<HWPML><BODY><SECTION><TABLE/></SECTION></BODY></HWPML>"#;
    let inside_paragraph = br#"<HWPML><BODY><SECTION><P><TEXT><CHAR>before</CHAR><TABLE/></TEXT></P></SECTION></BODY></HWPML>"#;
    for xml in [outside_paragraph.as_slice(), inside_paragraph.as_slice()] {
        assert_eq!(
            parse(xml, &ParseOptions::default()).unwrap_err().code,
            ErrorCode::UnsupportedFormat
        );
    }
}

#[test]
fn rejects_tables_nested_in_inline_footnotes() {
    let xml = br#"<HWPML><BODY><SECTION><P><TEXT><FOOTNOTE><TEXT><CHAR>note</CHAR><TABLE/></TEXT></FOOTNOTE></TEXT></P></SECTION></BODY></HWPML>"#;
    assert_eq!(
        parse(xml, &ParseOptions::default()).unwrap_err().code,
        ErrorCode::UnsupportedFormat
    );
}

#[test]
fn paragraph_table_preflight_matches_the_paragraph_text_walk() {
    let visited_wrapper_table = br#"<HWPML><BODY><SECTION><P><TEXT><HEADER><TABLE/></HEADER></TEXT></P></SECTION></BODY></HWPML>"#;
    assert_eq!(
        parse(visited_wrapper_table, &ParseOptions::default())
            .unwrap_err()
            .code,
        ErrorCode::UnsupportedFormat
    );

    let ignored_wrapper_table = br#"<HWPML><BODY><SECTION><P><TEXT><PICTURE><TABLE/></PICTURE><CHAR>kept</CHAR></TEXT></P></SECTION></BODY></HWPML>"#;
    let parsed = parse(ignored_wrapper_table, &ParseOptions::default()).unwrap();
    assert_eq!(parsed.blocks.len(), 1);
    assert_eq!(parsed.blocks[0].text.as_deref(), Some("kept"));
}

#[test]
fn ignores_tables_only_when_their_section_is_not_selected() {
    let xml = br#"<HWPML><BODY><SECTION><TABLE/></SECTION><SECTION><P><TEXT><CHAR>selected</CHAR></TEXT></P></SECTION></BODY></HWPML>"#;
    let options = ParseOptions {
        pages: Some(PageSelection::Numbers(vec![PageNumber::new(2.0).unwrap()])),
        ..ParseOptions::default()
    };
    let parsed = parse(xml, &options).unwrap();
    assert_eq!(parsed.blocks.len(), 1);
    assert_eq!(parsed.blocks[0].text.as_deref(), Some("selected"));
    assert_eq!(parsed.blocks[0].page_number, Some(2));
}

#[test]
fn rejects_wrong_document_root_and_forbidden_dtd() {
    assert_eq!(
        parse(b"<NOT_HWPML/>", &ParseOptions::default())
            .unwrap_err()
            .code,
        ErrorCode::UnsupportedFormat
    );
    let with_dtd = br#"<!DOCTYPE HWPML [<!ENTITY x SYSTEM "file:///nonexistent/entity-probe">]><HWPML><BODY/></HWPML>"#;
    assert_eq!(
        parse(with_dtd, &ParseOptions::default()).unwrap_err().code,
        ErrorCode::Corrupted
    );
}

#[test]
fn rejects_unclosed_xml_without_partial_recovery() {
    let unclosed = b"<HWPML><BODY><SECTION><P><TEXT><CHAR>kept?</CHAR></TEXT>";
    assert_eq!(
        parse(unclosed, &ParseOptions::default()).unwrap_err().code,
        ErrorCode::Corrupted
    );
}

#[test]
fn input_limit_is_inclusive_and_rejects_the_next_byte() {
    assert!(check_input_size(MAX_HWPML_BYTES).is_ok());
    assert_eq!(
        check_input_size(MAX_HWPML_BYTES + 1).unwrap_err().code,
        ErrorCode::DecompressionBomb
    );
}

#[test]
fn oversized_nbsp_input_is_rejected_before_normalization_budgeting() {
    let mut input = vec![b'x'; MAX_HWPML_BYTES + 1];
    input[..6].copy_from_slice(b"&nbsp;");
    let mut no_output_budget = LoweringBudget::with_limit(0);
    assert_eq!(
        parse_with_budget(&input, &ParseOptions::default(), &mut no_output_budget)
            .unwrap_err()
            .code,
        ErrorCode::DecompressionBomb
    );
}

#[test]
fn nbsp_outline_and_ir_copies_fit_exact_output_budget_only() {
    let xml = br#"<HWPML><HEAD><MAPPINGTABLE><PARASHAPELIST><PARASHAPE Id="h" HeadingType="Outline" Level="0"/></PARASHAPELIST></MAPPINGTABLE></HEAD><BODY><SECTION><P ParaShape="h"><TEXT><CHAR>x&nbsp;y</CHAR></TEXT></P></SECTION></BODY></HWPML>"#;
    let normalized_text_bytes = "x\u{00a0}y".len();
    let expected = xml.len()
        + normalized_text_bytes * 3
        + std::mem::size_of::<(String, Option<u32>)>() * 2
        + std::mem::size_of::<usize>() * 2
        + 1 // copied ParaShape identifier
        + std::mem::size_of::<IrBlock>()
        + std::mem::size_of::<OutlineItem>();

    let options = ParseOptions::default();
    let mut exact = LoweringBudget::with_limit(expected);
    let parsed = parse_with_budget(xml, &options, &mut exact).unwrap();
    assert_eq!(parsed.blocks[0].text.as_deref(), Some("x\u{00a0}y"));
    assert_eq!(parsed.outline.unwrap()[0].text, "x\u{00a0}y");

    let mut one_short = LoweringBudget::with_limit(expected - 1);
    assert_eq!(
        parse_with_budget(xml, &options, &mut one_short)
            .unwrap_err()
            .code,
        ErrorCode::OutputTooLarge
    );
}

#[test]
fn shared_xml_text_limit_remains_a_hard_decompression_error() {
    let text = "x".repeat(16 * 1024 * 1024 + 1);
    let xml = format!(
        "<HWPML><BODY><SECTION><P><TEXT><CHAR>{text}</CHAR></TEXT></P></SECTION></BODY></HWPML>"
    );
    assert_eq!(
        parse(xml.as_bytes(), &ParseOptions::default())
            .unwrap_err()
            .code,
        ErrorCode::DecompressionBomb
    );
}
