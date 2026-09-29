use crate::hwpx::sections::{SectionInput, lower_sections, order_section_paths};
use crate::hwpx::styles::StyleCatalog;
use kordoc_ir::{
    IrBlock, IrBlockType, IrCell, IrTable, PageMode, PageNumber, PageSelection, ParseOptions,
    WarningCode,
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
        "<hp:p><hp:run><hp:t>A</hp:t><hp:ctrl><hp:footNote><hp:subList><hp:p><hp:run><hp:t>foot</hp:t></hp:run></hp:p></hp:subList></hp:footNote></hp:ctrl><hp:t>B</hp:t><hp:ctrl><hp:endNote><hp:subList><hp:p><hp:run><hp:t>end</hp:t></hp:run></hp:p></hp:subList></hp:endNote></hp:ctrl><hp:t>C</hp:t></hp:run></hp:p>",
    );
    let output = lower_sections(
        &[input],
        &StyleCatalog::default(),
        None,
        &ParseOptions::default(),
    )
    .unwrap();
    assert_eq!(output.blocks.len(), 1);
    assert_eq!(output.blocks[0].text.as_deref(), Some("ABC"));
    assert_eq!(output.blocks[0].footnote_text.as_deref(), Some("foot\nend"));
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
