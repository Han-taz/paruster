use crate::hwpx::sections::{SectionInput, lower_sections, order_section_paths};
use crate::hwpx::styles::StyleCatalog;
use kordoc_ir::{
    ErrorCode, IrBlock, IrBlockType, IrCell, IrTable, PageMode, PageNumber, PageSelection,
    ParseOptions, WarningCode,
};

fn section(path: &str, body: &str) -> SectionInput {
    SectionInput::new(
        path,
        format!("<hs:sec xmlns:hs=\"urn:hs\" xmlns:hp=\"urn:hp\">{body}</hs:sec>").into_bytes(),
    )
}

#[test]
fn section_depth_201_is_partial_parse() {
    let deep = format!(
        "{}<hp:p><hp:run><hp:t>hidden</hp:t></hp:run></hp:p>{}",
        "<w>".repeat(200),
        "</w>".repeat(200)
    );
    let output = lower_sections(
        &[section("Contents/section0.xml", &deep)],
        &StyleCatalog::default(),
        None,
        &ParseOptions::default(),
    )
    .unwrap();
    assert!(output.blocks.is_empty());
    assert_eq!(output.warnings[0].code, WarningCode::PartialParse);
    assert_eq!(output.warnings[0].page, Some(1));
}

#[test]
fn malformed_middle_section_preserves_neighbors() {
    let inputs = [
        section(
            "Contents/section0.xml",
            "<hp:p><hp:run><hp:t>first</hp:t></hp:run></hp:p>",
        ),
        SectionInput::new(
            "Contents/section1.xml",
            b"<hs:sec><hp:p><hp:t>partial".to_vec(),
        ),
        section(
            "Contents/section2.xml",
            "<hp:p><hp:run><hp:t>third</hp:t></hp:run></hp:p>",
        ),
    ];
    let output = lower_sections(
        &inputs,
        &StyleCatalog::default(),
        None,
        &ParseOptions::default(),
    )
    .unwrap();
    assert_eq!(output.blocks.len(), 2);
    assert_eq!(output.blocks[0].text.as_deref(), Some("first"));
    assert_eq!(output.blocks[1].text.as_deref(), Some("third"));
    assert_eq!(output.blocks[0].page_number, Some(1));
    assert_eq!(output.blocks[1].page_number, Some(3));
    assert_eq!(output.warnings.len(), 1);
    assert_eq!(output.warnings[0].code, WarningCode::PartialParse);
}

#[test]
fn spine_order_wins_over_zip_order() {
    let available = [
        "Contents/section0.xml".to_owned(),
        "Contents/section1.xml".to_owned(),
    ];
    let spine = [
        "Contents/section1.xml".to_owned(),
        "Contents/section0.xml".to_owned(),
    ];
    let ordered = order_section_paths(&available, Some(&spine)).unwrap();
    assert_eq!(ordered, spine);
}

#[test]
fn falls_back_to_numeric_section_order() {
    let available = [
        "Contents/section12.xml".to_owned(),
        "Contents/section2.xml".to_owned(),
        "Contents/section1.xml".to_owned(),
    ];
    let ordered = order_section_paths(&available, None).unwrap();
    assert_eq!(
        ordered,
        [
            "Contents/section1.xml",
            "Contents/section2.xml",
            "Contents/section12.xml"
        ]
    );
    assert_eq!(order_section_paths(&available, Some(&[])).unwrap(), ordered);
}

#[test]
fn keeps_paragraph_run_spans_and_heading_outline() {
    let header = br#"<hh:head xmlns:hh="urn:hh"><hh:paraPr id="4" outlineLvl="2"/><hh:charPr id="7" bold="true"/></hh:head>"#;
    let styles = StyleCatalog::parse(header).unwrap();
    let input = section(
        "Contents/section0.xml",
        "<hp:p paraPrIDRef=\"4\"><hp:run charPrIDRef=\"7\"><hp:t>Bold</hp:t></hp:run><hp:run><hp:t> tail</hp:t></hp:run></hp:p>",
    );
    let output = lower_sections(&[input], &styles, None, &ParseOptions::default()).unwrap();
    let block = &output.blocks[0];
    assert_eq!(block.kind, IrBlockType::Heading);
    assert_eq!(block.text.as_deref(), Some("Bold tail"));
    assert_eq!(block.level, Some(2));
    let spans = block.spans.as_ref().unwrap();
    assert_eq!(spans.len(), 2);
    assert_eq!(spans[0].text, "Bold");
    assert_eq!(spans[0].bold, Some(true));
    assert_eq!(spans[1].text, " tail");
    assert_eq!(output.outline[0].text, "Bold tail");
}

#[test]
fn retains_footnotes_and_endnotes() {
    let input = section(
        "Contents/section0.xml",
        "<hp:p><hp:run><hp:t>body</hp:t></hp:run></hp:p><hp:footNote><hp:subList><hp:p><hp:run><hp:t>foot</hp:t></hp:run></hp:p></hp:subList></hp:footNote><hp:endNote><hp:subList><hp:p><hp:run><hp:t>end</hp:t></hp:run></hp:p></hp:subList></hp:endNote>",
    );
    let output = lower_sections(
        &[input],
        &StyleCatalog::default(),
        None,
        &ParseOptions::default(),
    )
    .unwrap();
    assert_eq!(output.blocks.len(), 3);
    assert_eq!(output.blocks[1].footnote_text.as_deref(), Some("foot"));
    assert_eq!(output.blocks[2].footnote_text.as_deref(), Some("end"));
}

#[test]
fn lowers_inline_notes_in_source_order_and_attaches_them_to_host() {
    let input = section(
        "Contents/section0.xml",
        "<hp:p><hp:run><hp:t>A</hp:t><hp:ctrl><hp:footNote number=\"7\"><hp:subList><hp:p><hp:run><hp:t>foot</hp:t></hp:run></hp:p></hp:subList></hp:footNote></hp:ctrl><hp:t>B</hp:t><hp:ctrl><hp:endNote number=\"2\" prefixChar=\"47928\" suffixChar=\"65289\"><hp:subList><hp:p><hp:run><hp:t>end</hp:t></hp:run></hp:p></hp:subList></hp:endNote></hp:ctrl><hp:t>C</hp:t></hp:run></hp:p>",
    );
    let output = lower_sections(
        &[input],
        &StyleCatalog::default(),
        None,
        &ParseOptions::default(),
    )
    .unwrap();
    assert_eq!(output.blocks.len(), 1);
    assert_eq!(output.blocks[0].text.as_deref(), Some("A7)B문2）C"));
    assert_eq!(
        output.blocks[0]
            .spans
            .as_ref()
            .unwrap()
            .iter()
            .map(|span| span.text.as_str())
            .collect::<String>(),
        "A7)B문2）C"
    );
    assert_eq!(output.blocks[0].footnote_text.as_deref(), Some("foot\nend"));
}

#[test]
fn carries_page_evidence_through_a_spanning_final_paragraph_and_next_section() {
    let inputs = [
        section(
            "Contents/section0.xml",
            "<hp:p><hp:linesegarray><hp:lineseg vertpos=\"3000\"/><hp:lineseg vertpos=\"0\"/></hp:linesegarray><hp:run><hp:t>spanning final paragraph</hp:t></hp:run></hp:p>",
        ),
        section(
            "Contents/section1.xml",
            "<hp:p><hp:linesegarray><hp:lineseg vertpos=\"0\"/></hp:linesegarray><hp:run><hp:t>next section</hp:t></hp:run></hp:p>",
        ),
    ];
    let output = lower_sections(
        &inputs,
        &StyleCatalog::default(),
        None,
        &ParseOptions::default(),
    )
    .unwrap();
    assert_eq!(output.blocks[0].page_number, Some(1));
    assert_eq!(output.blocks[1].page_number, Some(3));
    assert_eq!(
        output
            .page_evidence
            .iter()
            .map(|evidence| evidence.page_number)
            .collect::<Vec<_>>(),
        [1, 2, 3]
    );
}

#[test]
fn inherits_section_note_number_formats_and_applies_note_local_decorations() {
    let input = section(
        "Contents/section0.xml",
        "<hp:p><hp:run><hp:secPr><hp:footNotePr><hp:autoNumFormat type=\"DIGIT\" userChar=\"\" prefixChar=\"문\" suffixChar=\"）\"/></hp:footNotePr><hp:endNotePr><hp:autoNumFormat type=\"DIGIT\" userChar=\"\" prefixChar=\"E\" suffixChar=\".\"/></hp:endNotePr></hp:secPr><hp:t>A</hp:t><hp:ctrl><hp:footNote number=\"3\"><hp:subList><hp:p><hp:run><hp:t>foot</hp:t></hp:run></hp:p></hp:subList></hp:footNote></hp:ctrl><hp:t>B</hp:t><hp:ctrl><hp:endNote number=\"4\" prefixChar=\"40\" suffixChar=\"41\"><hp:subList><hp:p><hp:run><hp:t>end</hp:t></hp:run></hp:p></hp:subList></hp:endNote></hp:ctrl><hp:t>C</hp:t><hp:ctrl><hp:endNote number=\"5\"><hp:subList><hp:p><hp:run><hp:t>inherited end</hp:t></hp:run></hp:p></hp:subList></hp:endNote></hp:ctrl><hp:t>D</hp:t></hp:run></hp:p>",
    );
    let output = lower_sections(
        &[input],
        &StyleCatalog::default(),
        None,
        &ParseOptions::default(),
    )
    .unwrap();
    assert_eq!(output.blocks[0].text.as_deref(), Some("A문3）B(4)CE5.D"));
}

#[test]
fn inherits_user_character_note_format_and_honors_note_local_user_character() {
    let input = section(
        "Contents/section0.xml",
        "<hp:p><hp:run><hp:secPr><hp:footNotePr><hp:autoNumFormat type=\"USER_CHAR\" userChar=\"*\" prefixChar=\"\" suffixChar=\"\"/></hp:footNotePr></hp:secPr><hp:t>A</hp:t><hp:ctrl><hp:footNote number=\"3\"><hp:subList><hp:p><hp:run><hp:t>inherited</hp:t></hp:run></hp:p></hp:subList></hp:footNote></hp:ctrl><hp:t>B</hp:t><hp:ctrl><hp:footNote number=\"4\" userChar=\"9733\"><hp:subList><hp:p><hp:run><hp:t>overridden</hp:t></hp:run></hp:p></hp:subList></hp:footNote></hp:ctrl><hp:t>C</hp:t></hp:run></hp:p>",
    );
    let output = lower_sections(
        &[input],
        &StyleCatalog::default(),
        None,
        &ParseOptions::default(),
    )
    .unwrap();
    assert_eq!(output.blocks[0].text.as_deref(), Some("A*B★C"));
}

#[test]
fn distinguishes_missing_note_suffix_from_absent_note_format() {
    let input = section(
        "Contents/section0.xml",
        "<hp:p><hp:run><hp:secPr><hp:footNotePr><hp:autoNumFormat type=\"USER_CHAR\" userChar=\"*\"/></hp:footNotePr><hp:endNotePr><hp:autoNumFormat type=\"USER_CHAR\" userChar=\"+\" suffixChar=\"\"/></hp:endNotePr></hp:secPr><hp:t>A</hp:t><hp:ctrl><hp:footNote number=\"3\"><hp:subList><hp:p><hp:run><hp:t>foot</hp:t></hp:run></hp:p></hp:subList></hp:footNote></hp:ctrl><hp:t>B</hp:t><hp:ctrl><hp:endNote number=\"4\"><hp:subList><hp:p><hp:run><hp:t>end</hp:t></hp:run></hp:p></hp:subList></hp:endNote></hp:ctrl><hp:t>C</hp:t></hp:run></hp:p>",
    );
    let output = lower_sections(
        &[input],
        &StyleCatalog::default(),
        None,
        &ParseOptions::default(),
    )
    .unwrap();
    assert_eq!(output.blocks[0].text.as_deref(), Some("A*B+C"));

    let no_formats = section(
        "Contents/section0.xml",
        "<hp:p><hp:run><hp:t>A</hp:t><hp:ctrl><hp:footNote number=\"3\"><hp:subList><hp:p><hp:run><hp:t>foot</hp:t></hp:run></hp:p></hp:subList></hp:footNote></hp:ctrl><hp:t>B</hp:t><hp:ctrl><hp:endNote number=\"4\"><hp:subList><hp:p><hp:run><hp:t>end</hp:t></hp:run></hp:p></hp:subList></hp:endNote></hp:ctrl><hp:t>C</hp:t></hp:run></hp:p>",
    );
    let output = lower_sections(
        &[no_formats],
        &StyleCatalog::default(),
        None,
        &ParseOptions::default(),
    )
    .unwrap();
    assert_eq!(output.blocks[0].text.as_deref(), Some("A3)B4)C"));
}

#[test]
fn resets_layout_page_number_between_sections() {
    let inputs = [
        section(
            "Contents/section0.xml",
            "<hp:p><hp:linesegarray><hp:lineseg vertpos=\"0\"/></hp:linesegarray><hp:run><hp:t>one</hp:t></hp:run></hp:p>",
        ),
        section(
            "Contents/section1.xml",
            "<hp:p><hp:linesegarray><hp:lineseg vertpos=\"0\"/></hp:linesegarray><hp:run><hp:t>two</hp:t></hp:run></hp:p>",
        ),
    ];
    let output = lower_sections(
        &inputs,
        &StyleCatalog::default(),
        None,
        &ParseOptions::default(),
    )
    .unwrap();
    assert_eq!(
        output
            .blocks
            .iter()
            .map(|block| block.page_number)
            .collect::<Vec<_>>(),
        [Some(1), Some(2)]
    );
}

#[test]
fn infers_explicit_and_intra_paragraph_page_breaks() {
    let input = section(
        "Contents/section0.xml",
        "<hp:p><hp:linesegarray><hp:lineseg vertpos=\"3000\"/></hp:linesegarray><hp:run><hp:t>first</hp:t></hp:run></hp:p><hp:p pageBreak=\"1\"><hp:linesegarray><hp:lineseg vertpos=\"0\"/></hp:linesegarray><hp:run><hp:t>explicit</hp:t></hp:run></hp:p><hp:p><hp:linesegarray><hp:lineseg vertpos=\"4000\"/><hp:lineseg vertpos=\"0\"/></hp:linesegarray><hp:run><hp:t>spanning</hp:t></hp:run></hp:p><hp:p><hp:linesegarray><hp:lineseg vertpos=\"500\"/></hp:linesegarray><hp:run><hp:t>after</hp:t></hp:run></hp:p>",
    );
    let output = lower_sections(
        &[input],
        &StyleCatalog::default(),
        None,
        &ParseOptions::default(),
    )
    .unwrap();
    assert_eq!(
        output
            .blocks
            .iter()
            .map(|block| block.page_number)
            .collect::<Vec<_>>(),
        [Some(1), Some(2), Some(2), Some(3)]
    );
}

#[test]
fn derives_layout_pages_from_linesegarray() {
    let input = section(
        "Contents/section0.xml",
        "<hp:p><hp:linesegarray><hp:lineseg vertpos=\"0\"/></hp:linesegarray><hp:run><hp:t>first</hp:t></hp:run></hp:p><hp:p><hp:linesegarray><hp:lineseg vertpos=\"3000\"/></hp:linesegarray><hp:run><hp:t>continuation</hp:t></hp:run></hp:p><hp:p><hp:linesegarray><hp:lineseg vertpos=\"0\"/></hp:linesegarray><hp:run><hp:t>second page</hp:t></hp:run></hp:p>",
    );
    let output = lower_sections(
        &[input],
        &StyleCatalog::default(),
        None,
        &ParseOptions::default(),
    )
    .unwrap();
    assert_eq!(output.page_mode, Some(PageMode::Layout));
    assert_eq!(
        output
            .blocks
            .iter()
            .map(|block| block.page_number)
            .collect::<Vec<_>>(),
        [Some(1), Some(1), Some(2)]
    );
    let selected = lower_sections(
        &[section(
            "Contents/section0.xml",
            "<hp:p><hp:linesegarray><hp:lineseg vertpos=\"3000\"/></hp:linesegarray><hp:run><hp:t>page one</hp:t></hp:run></hp:p><hp:p><hp:linesegarray><hp:lineseg vertpos=\"0\"/></hp:linesegarray><hp:run><hp:t>page two</hp:t></hp:run></hp:p>",
        )],
        &StyleCatalog::default(),
        None,
        &ParseOptions {
            pages: Some(PageSelection::Numbers(vec![PageNumber::new(2.0).unwrap()])),
            ..ParseOptions::default()
        },
    )
    .unwrap();
    assert_eq!(selected.page_mode, Some(PageMode::Layout));
    assert_eq!(selected.blocks.len(), 1);
    assert_eq!(selected.blocks[0].text.as_deref(), Some("page two"));
}

#[test]
fn incomplete_paragraph_layout_falls_back_for_the_whole_section() {
    let input = section(
        "Contents/section0.xml",
        "<hp:p><hp:linesegarray><hp:lineseg vertpos=\"0\"/></hp:linesegarray><hp:run><hp:t>first</hp:t></hp:run></hp:p><hp:p><hp:run><hp:t>missing layout</hp:t></hp:run></hp:p>",
    );
    let output = lower_sections(
        &[input],
        &StyleCatalog::default(),
        None,
        &ParseOptions::default(),
    )
    .unwrap();
    assert_eq!(output.page_mode, Some(PageMode::Section));
    assert_eq!(
        output
            .blocks
            .iter()
            .map(|block| block.page_number)
            .collect::<Vec<_>>(),
        [Some(1), Some(1)]
    );
    assert_eq!(
        output
            .page_evidence
            .iter()
            .map(|evidence| evidence.page_number)
            .collect::<Vec<_>>(),
        [1]
    );
}

#[test]
fn incomplete_layout_in_any_section_uses_section_pages_for_page_selection() {
    let inputs = [
        section(
            "Contents/section0.xml",
            "<hp:p><hp:linesegarray><hp:lineseg vertpos=\"0\"/></hp:linesegarray><hp:run><hp:t>first section</hp:t></hp:run></hp:p>",
        ),
        section(
            "Contents/section1.xml",
            "<hp:p><hp:run><hp:t>second section</hp:t></hp:run></hp:p>",
        ),
    ];
    let output = lower_sections(
        &inputs,
        &StyleCatalog::default(),
        None,
        &ParseOptions {
            pages: Some(PageSelection::Numbers(vec![PageNumber::new(2.0).unwrap()])),
            ..ParseOptions::default()
        },
    )
    .unwrap();
    assert_eq!(output.page_mode, Some(PageMode::Section));
    assert_eq!(output.blocks.len(), 1);
    assert_eq!(output.blocks[0].text.as_deref(), Some("second section"));
    assert_eq!(output.blocks[0].page_number, Some(2));
    assert_eq!(
        output
            .page_evidence
            .iter()
            .map(|evidence| evidence.page_number)
            .collect::<Vec<_>>(),
        [2]
    );
}

#[test]
fn dropped_empty_paragraph_keeps_its_explicit_layout_page_break() {
    let input = section(
        "Contents/section0.xml",
        "<hp:p><hp:linesegarray><hp:lineseg vertpos=\"0\"/></hp:linesegarray><hp:run><hp:t>first</hp:t></hp:run></hp:p><hp:p pageBreak=\"1\"><hp:linesegarray><hp:lineseg vertpos=\"0\"/></hp:linesegarray></hp:p><hp:p><hp:linesegarray><hp:lineseg vertpos=\"100\"/></hp:linesegarray><hp:run><hp:t>next</hp:t></hp:run></hp:p>",
    );
    let output = lower_sections(
        &[input],
        &StyleCatalog::default(),
        None,
        &ParseOptions::default(),
    )
    .unwrap();
    assert_eq!(output.page_mode, Some(PageMode::Layout));
    assert_eq!(
        output
            .blocks
            .iter()
            .map(|block| (block.text.as_deref(), block.page_number))
            .collect::<Vec<_>>(),
        [(Some("first"), Some(1)), (Some("next"), Some(2))]
    );
    assert_eq!(
        output
            .page_evidence
            .iter()
            .map(|evidence| evidence.page_number)
            .collect::<Vec<_>>(),
        [1, 2]
    );
}

#[test]
fn layout_page_evidence_keeps_empty_initial_page() {
    let input = section(
        "Contents/section0.xml",
        "<hp:p><hp:linesegarray><hp:lineseg vertpos=\"3000\"/><hp:lineseg vertpos=\"0\"/></hp:linesegarray></hp:p><hp:p><hp:linesegarray><hp:lineseg vertpos=\"100\"/></hp:linesegarray><hp:run><hp:t>page two</hp:t></hp:run></hp:p>",
    );
    let output = lower_sections(
        &[input],
        &StyleCatalog::default(),
        None,
        &ParseOptions::default(),
    )
    .unwrap();
    assert_eq!(output.page_mode, Some(PageMode::Layout));
    assert_eq!(output.blocks[0].page_number, Some(2));
    assert_eq!(
        output
            .page_evidence
            .iter()
            .map(|evidence| evidence.page_number)
            .collect::<Vec<_>>(),
        [1, 2]
    );
}

#[test]
fn all_empty_layout_document_still_has_page_one_evidence() {
    let input = section(
        "Contents/section0.xml",
        "<hp:p><hp:linesegarray><hp:lineseg vertpos=\"0\"/></hp:linesegarray></hp:p>",
    );
    let output = lower_sections(
        &[input],
        &StyleCatalog::default(),
        None,
        &ParseOptions::default(),
    )
    .unwrap();
    assert_eq!(output.page_mode, Some(PageMode::Layout));
    assert!(output.blocks.is_empty());
    assert_eq!(
        output
            .page_evidence
            .iter()
            .map(|evidence| evidence.page_number)
            .collect::<Vec<_>>(),
        [1]
    );
}

#[test]
fn malformed_middle_section_remains_in_section_page_evidence() {
    let inputs = [
        section(
            "Contents/section0.xml",
            "<hp:p><hp:run><hp:t>first</hp:t></hp:run></hp:p>",
        ),
        SectionInput::new("Contents/section1.xml", b"<hs:sec><hp:p>broken".to_vec()),
        section(
            "Contents/section2.xml",
            "<hp:p><hp:run><hp:t>third</hp:t></hp:run></hp:p>",
        ),
    ];
    let output = lower_sections(
        &inputs,
        &StyleCatalog::default(),
        None,
        &ParseOptions::default(),
    )
    .unwrap();
    assert_eq!(output.page_mode, Some(PageMode::Section));
    assert_eq!(
        output
            .page_evidence
            .iter()
            .map(|evidence| evidence.page_number)
            .collect::<Vec<_>>(),
        [1, 2, 3]
    );
}

#[test]
fn multi_column_layout_suppresses_rightward_vertical_resets() {
    let input = section(
        "Contents/section0.xml",
        "<hp:secPr><hp:colPr colCount=\"2\"/></hp:secPr><hp:p><hp:linesegarray><hp:lineseg vertpos=\"3000\" horzpos=\"0\"/><hp:lineseg vertpos=\"0\" horzpos=\"10000\"/></hp:linesegarray><hp:run><hp:t>columns</hp:t></hp:run></hp:p><hp:p><hp:linesegarray><hp:lineseg vertpos=\"1000\" horzpos=\"0\"/></hp:linesegarray><hp:run><hp:t>same page</hp:t></hp:run></hp:p><hp:p><hp:linesegarray><hp:lineseg vertpos=\"0\" horzpos=\"0\"/></hp:linesegarray><hp:run><hp:t>next page</hp:t></hp:run></hp:p>",
    );
    let output = lower_sections(
        &[input],
        &StyleCatalog::default(),
        None,
        &ParseOptions::default(),
    )
    .unwrap();
    assert_eq!(output.page_mode, Some(PageMode::Layout));
    assert_eq!(
        output
            .blocks
            .iter()
            .map(|block| block.page_number)
            .collect::<Vec<_>>(),
        [Some(1), Some(1), Some(2)]
    );
}

#[test]
fn multi_column_layout_suppresses_rightward_resets_between_paragraphs() {
    let input = section(
        "Contents/section0.xml",
        "<hp:secPr><hp:colPr colCount=\"2\"/></hp:secPr><hp:p><hp:linesegarray><hp:lineseg vertpos=\"3000\" horzpos=\"0\"/></hp:linesegarray><hp:run><hp:t>first column</hp:t></hp:run></hp:p><hp:p><hp:linesegarray><hp:lineseg vertpos=\"0\" horzpos=\"10000\"/></hp:linesegarray><hp:run><hp:t>second column</hp:t></hp:run></hp:p><hp:p><hp:linesegarray><hp:lineseg vertpos=\"0\" horzpos=\"0\"/></hp:linesegarray><hp:run><hp:t>next page</hp:t></hp:run></hp:p>",
    );
    let output = lower_sections(
        &[input],
        &StyleCatalog::default(),
        None,
        &ParseOptions::default(),
    )
    .unwrap();
    assert_eq!(output.page_mode, Some(PageMode::Layout));
    assert_eq!(
        output
            .blocks
            .iter()
            .map(|block| block.page_number)
            .collect::<Vec<_>>(),
        [Some(1), Some(1), Some(2)]
    );
}

#[test]
fn honors_keep_empty_paragraphs_only_when_true() {
    let input = section(
        "Contents/section0.xml",
        "<hp:p/><hp:p><hp:run><hp:t>text</hp:t></hp:run></hp:p>",
    );
    let omitted = lower_sections(
        std::slice::from_ref(&input),
        &StyleCatalog::default(),
        None,
        &ParseOptions::default(),
    )
    .unwrap();
    let disabled = lower_sections(
        std::slice::from_ref(&input),
        &StyleCatalog::default(),
        None,
        &ParseOptions {
            keep_empty_paragraphs: Some(false),
            ..ParseOptions::default()
        },
    )
    .unwrap();
    let enabled = lower_sections(
        &[input],
        &StyleCatalog::default(),
        None,
        &ParseOptions {
            keep_empty_paragraphs: Some(true),
            ..ParseOptions::default()
        },
    )
    .unwrap();
    assert_eq!(omitted.blocks.len(), 1);
    assert_eq!(disabled.blocks.len(), 1);
    assert_eq!(enabled.blocks.len(), 2);
    assert_eq!(enabled.blocks[0].text.as_deref(), Some(""));
}

#[test]
fn uses_layout_cache_when_all_sections_usable() {
    let input = section(
        "Contents/section0.xml",
        "<hp:p><hp:run><hp:t>one</hp:t></hp:run></hp:p><hp:p><hp:run><hp:t>two</hp:t></hp:run></hp:p>",
    );
    let cache = vec![vec![1, 2]];
    let output = lower_sections(
        &[input],
        &StyleCatalog::default(),
        Some(&cache),
        &ParseOptions::default(),
    )
    .unwrap();
    assert_eq!(output.page_mode, Some(PageMode::Layout));
    assert_eq!(output.blocks[0].page_number, Some(1));
    assert_eq!(output.blocks[1].page_number, Some(2));
}

#[test]
fn supplied_layout_cache_evidence_includes_sparse_intermediate_pages() {
    let input = section(
        "Contents/section0.xml",
        "<hp:p><hp:run><hp:t>first</hp:t></hp:run></hp:p><hp:p><hp:run><hp:t>third</hp:t></hp:run></hp:p>",
    );
    let output = lower_sections(
        &[input],
        &StyleCatalog::default(),
        Some(&[vec![1, 3]]),
        &ParseOptions::default(),
    )
    .unwrap();
    assert_eq!(output.page_mode, Some(PageMode::Layout));
    assert_eq!(
        output
            .page_evidence
            .iter()
            .map(|evidence| evidence.page_number)
            .collect::<Vec<_>>(),
        [1, 2, 3]
    );
}

#[test]
fn empty_supplied_layout_cache_uses_section_page_evidence() {
    let input = section("Contents/section0.xml", "<hp:p/>");
    let cache = vec![vec![]];
    let output = lower_sections(
        &[input],
        &StyleCatalog::default(),
        Some(&cache),
        &ParseOptions::default(),
    )
    .unwrap();
    assert!(output.blocks.is_empty());
    assert_eq!(output.page_mode, Some(PageMode::Section));
    assert_eq!(
        output.page_evidence,
        vec![kordoc_ir::PageEvidence { page_number: 1 }]
    );
}

#[test]
fn supplied_layout_cache_evidence_includes_initial_pages_before_first_block() {
    let input = section(
        "Contents/section0.xml",
        "<hp:p><hp:run><hp:t>second</hp:t></hp:run></hp:p>",
    );
    let output = lower_sections(
        &[input],
        &StyleCatalog::default(),
        Some(&[vec![2]]),
        &ParseOptions::default(),
    )
    .unwrap();
    assert_eq!(output.blocks[0].page_number, Some(2));
    assert_eq!(
        output
            .page_evidence
            .iter()
            .map(|evidence| evidence.page_number)
            .collect::<Vec<_>>(),
        [1, 2]
    );
}

#[test]
fn supplied_layout_cache_page_evidence_is_bounded_before_allocation() {
    let input = section(
        "Contents/section0.xml",
        "<hp:p><hp:run><hp:t>far page</hp:t></hp:run></hp:p>",
    );
    let within_limit = lower_sections(
        std::slice::from_ref(&input),
        &StyleCatalog::default(),
        Some(&[vec![100_000]]),
        &ParseOptions::default(),
    )
    .unwrap();
    assert_eq!(within_limit.page_evidence.len(), 100_000);
    assert_eq!(within_limit.page_evidence[0].page_number, 1);
    assert_eq!(within_limit.page_evidence[99_999].page_number, 100_000);

    for page in [100_001, u32::MAX] {
        let error = lower_sections(
            std::slice::from_ref(&input),
            &StyleCatalog::default(),
            Some(&[vec![page]]),
            &ParseOptions::default(),
        )
        .unwrap_err();
        assert_eq!(error.code, ErrorCode::DecompressionBomb);
    }
}

#[test]
fn falls_back_to_section_pages_recursively() {
    let input = section(
        "Contents/section0.xml",
        "<hp:p><hp:run><hp:t>one</hp:t></hp:run></hp:p>",
    );
    let output = lower_sections(
        &[input],
        &StyleCatalog::default(),
        Some(&[]),
        &ParseOptions::default(),
    )
    .unwrap();
    assert_eq!(output.page_mode, Some(PageMode::Section));
    assert_eq!(output.blocks[0].page_number, Some(1));
    assert_eq!(output.page_evidence.len(), 1);

    let mut nested = IrBlock {
        children: Some(vec![IrBlock::paragraph("child")]),
        table: Some(IrTable {
            cells: vec![vec![IrCell {
                blocks: Some(vec![IrBlock::paragraph("cell")]),
                ..IrCell::default()
            }]],
            ..IrTable::default()
        }),
        ..IrBlock::default()
    };
    crate::hwpx::sections::assign_page_recursive(&mut nested, 4);
    assert_eq!(nested.children.as_ref().unwrap()[0].page_number, Some(4));
    assert_eq!(
        nested.table.as_ref().unwrap().cells[0][0]
            .blocks
            .as_ref()
            .unwrap()[0]
            .page_number,
        Some(4)
    );
}
