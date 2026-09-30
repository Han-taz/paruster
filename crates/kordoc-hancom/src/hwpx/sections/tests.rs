use crate::hwpx::budget::LoweringBudget;
use crate::hwpx::sections::{
    SectionInput, lower_sections, lower_sections_impl, lower_sections_with_package,
    order_section_paths,
};
use crate::hwpx::styles::StyleCatalog;
use kordoc_ir::{
    ErrorCode, IrBlock, IrBlockType, IrCell, IrTable, PageMode, PageNumber, PageSelection,
    ParseOptions, WarningCode,
};

fn lower_with_budget_limit(
    inputs: &[SectionInput],
    styles: &StyleCatalog,
    options: &ParseOptions,
    limit: usize,
) -> Result<super::SectionOutput, kordoc_ir::KordocError> {
    let mut budget = LoweringBudget::with_limit(limit);
    lower_sections_impl(inputs, styles, None, options, None, &mut budget)
}

#[test]
fn section_budget_accepts_exact_empty_block_charge_and_rejects_one_less() {
    let inputs = [section("Contents/section0.xml", "<hp:p/>")];
    let options = ParseOptions {
        keep_empty_paragraphs: Some(true),
        ..ParseOptions::default()
    };

    let exact = (0..=4096)
        .find(|limit| {
            lower_with_budget_limit(&inputs, &StyleCatalog::default(), &options, *limit).is_ok()
        })
        .expect("small empty paragraph has a finite minimum budget");
    let output = lower_with_budget_limit(&inputs, &StyleCatalog::default(), &options, exact)
        .expect("minimum budget should lower the empty paragraph");
    assert_eq!(output.blocks.len(), 1);

    let error = lower_with_budget_limit(&inputs, &StyleCatalog::default(), &options, exact - 1)
        .unwrap_err();
    assert_eq!(error.code, ErrorCode::OutputTooLarge);
}

#[test]
fn omits_spans_for_empty_unstyled_paragraphs() {
    let inputs = [section("Contents/section0.xml", "<hp:p/>")];
    let output = lower_sections(
        &inputs,
        &StyleCatalog::default(),
        None,
        &ParseOptions {
            keep_empty_paragraphs: Some(true),
            ..ParseOptions::default()
        },
    )
    .unwrap();
    assert!(output.blocks[0].spans.is_none());
}

#[test]
fn nested_note_text_amplification_is_budgeted() {
    let note_text = "n".repeat(4096);
    let input = section(
        "Contents/section0.xml",
        &format!(
            "<hp:p><hp:run><hp:t>host</hp:t><hp:ctrl><hp:footNote><hp:subList><hp:p><hp:run><hp:t>{note_text}</hp:t></hp:run></hp:p></hp:subList></hp:footNote></hp:ctrl></hp:run></hp:p>"
        ),
    );
    let limit = note_text.len() * 2;
    let error = lower_with_budget_limit(
        &[input],
        &StyleCatalog::default(),
        &ParseOptions::default(),
        limit,
    )
    .unwrap_err();
    assert_eq!(error.code, ErrorCode::OutputTooLarge);
}

#[test]
fn repeated_note_markers_charge_large_format_text_before_copying() {
    let prefix = "p".repeat(4096);
    let body = format!(
        "<hp:footNotePr><hp:autoNumFormat type=\"DIGIT\" prefixChar=\"{prefix}\"/></hp:footNotePr><hp:p><hp:run><hp:t>x</hp:t><hp:ctrl><hp:footNote><hp:subList><hp:p><hp:run><hp:t>note</hp:t></hp:run></hp:p></hp:subList></hp:footNote></hp:ctrl></hp:run></hp:p>"
    );
    let input = section("Contents/section0.xml", &body);
    let error = lower_with_budget_limit(
        &[input],
        &StyleCatalog::default(),
        &ParseOptions::default(),
        prefix.len() * 3 + 2048,
    )
    .unwrap_err();
    assert_eq!(error.code, ErrorCode::OutputTooLarge);
}

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
fn preserves_header_and_trailing_empty_cells() {
    let output = lower_sections(
        &[section("Contents/section0.xml", "<hp:tbl><hp:tr><hp:tc header=\"1\"><hp:cellAddr rowAddr=\"0\" colAddr=\"0\"/><hp:subList><hp:p><hp:run><hp:t>Head</hp:t></hp:run></hp:p></hp:subList></hp:tc><hp:tc><hp:cellAddr rowAddr=\"0\" colAddr=\"2\"/><hp:subList/></hp:tc></hp:tr></hp:tbl>")],
        &StyleCatalog::default(),
        None,
        &ParseOptions {
            keep_trailing_empty_cols: Some(true),
            ..ParseOptions::default()
        },
    )
    .unwrap();
    let table = output.blocks[0].table.as_ref().unwrap();
    assert_eq!(table.cols, 3);
    assert_eq!(table.cells[0][0].is_header, Some(true));
    assert_eq!(table.cells[0][2].text, "");
}

#[test]
fn keeps_nested_table_and_caption_blocks_in_order() {
    let output = lower_sections(
        &[section("Contents/section0.xml", "<hp:tbl><hp:caption><hp:subList><hp:p><hp:run><hp:t>cap</hp:t></hp:run></hp:p><hp:tbl><hp:tr><hp:tc><hp:subList><hp:p><hp:run><hp:t>caption child</hp:t></hp:run></hp:p></hp:subList></hp:tc></hp:tr></hp:tbl></hp:subList></hp:caption><hp:tr><hp:tc><hp:subList><hp:p><hp:run><hp:t>before</hp:t></hp:run></hp:p><hp:tbl><hp:tr><hp:tc><hp:subList><hp:p><hp:run><hp:t>nested</hp:t></hp:run></hp:p></hp:subList></hp:tc></hp:tr></hp:tbl><hp:p><hp:run><hp:t>after</hp:t></hp:run></hp:p></hp:subList></hp:tc></hp:tr></hp:tbl>")],
        &StyleCatalog::default(), None, &ParseOptions::default()).unwrap();
    let table = output.blocks[0].table.as_ref().unwrap();
    let cell_blocks = table.cells[0][0].blocks.as_ref().unwrap();
    assert_eq!(table.cells[0][0].text, "before\nnested\nafter");
    assert_eq!(cell_blocks.len(), 3);
    assert_eq!(cell_blocks[0].text.as_deref(), Some("before"));
    assert!(cell_blocks[1].table.is_some());
    assert_eq!(cell_blocks[2].text.as_deref(), Some("after"));
    let caption_blocks = table.caption_blocks.as_ref().unwrap();
    assert_eq!(caption_blocks[0].text.as_deref(), Some("cap"));
    assert!(caption_blocks[1].table.is_some());
}

#[test]
fn table_and_image_inside_paragraph_keep_source_order() {
    use crate::hwpx::package::Package;
    use std::io::{Cursor, Write};
    use zip::{ZipWriter, write::SimpleFileOptions};
    let xml = "<hp:p><hp:run><hp:t>before</hp:t><hp:tbl><hp:tr><hp:tc><hp:subList><hp:p><hp:run><hp:t>inside</hp:t><hp:pic><hp:imgRect binaryItemIDRef=\"pic\"/></hp:pic></hp:run></hp:p></hp:subList></hp:tc></hp:tr></hp:tbl><hp:t>middle</hp:t><hp:pic><hp:imgRect binaryItemIDRef=\"pic\"/></hp:pic><hp:t>after</hp:t></hp:run></hp:p>";
    let input = section("Contents/section0.xml", xml);
    let mut writer = ZipWriter::new(Cursor::new(Vec::new()));
    writer
        .start_file("BinData/pic.png", SimpleFileOptions::default())
        .unwrap();
    writer.write_all(b"data").unwrap();
    let package_bytes = writer.finish().unwrap().into_inner();
    let mut package = Package::open(Box::leak(package_bytes.into_boxed_slice())).unwrap();
    let output = lower_sections_with_package(
        &[input],
        &StyleCatalog::default(),
        None,
        &ParseOptions::default(),
        &mut package,
    )
    .unwrap();
    assert_eq!(output.blocks.len(), 5);
    assert_eq!(output.blocks[0].text.as_deref(), Some("before"));
    assert!(output.blocks[1].table.is_some());
    assert_eq!(output.blocks[2].text.as_deref(), Some("middle"));
    assert_eq!(output.blocks[3].kind, IrBlockType::Image);
    assert_eq!(output.blocks[4].text.as_deref(), Some("after"));
    assert_eq!(output.images.len(), 1);
    let nested = output.blocks[1].table.as_ref().unwrap().cells[0][0]
        .blocks
        .as_ref()
        .unwrap();
    assert_eq!(nested[0].text.as_deref(), Some("inside"));
    assert_eq!(nested[1].kind, IrBlockType::Image);
}

#[test]
fn table_internal_page_split_counts_once_before_midpage_following_prose() {
    let input = section(
        "Contents/section0.xml",
        "<hp:p><hp:linesegarray><hp:lineseg vertpos=\"5000\"/></hp:linesegarray><hp:run><hp:t>host</hp:t><hp:tbl><hp:tr><hp:tc><hp:cellAddr rowAddr=\"0\" colAddr=\"0\"/><hp:subList><hp:p><hp:linesegarray><hp:lineseg vertpos=\"5000\"/></hp:linesegarray></hp:p><hp:p><hp:linesegarray><hp:lineseg vertpos=\"0\"/></hp:linesegarray></hp:p></hp:subList></hp:tc></hp:tr></hp:tbl></hp:run></hp:p><hp:p><hp:linesegarray><hp:lineseg vertpos=\"2500\"/></hp:linesegarray><hp:run><hp:t>continued below table</hp:t></hp:run></hp:p>",
    );
    let output = lower_sections(
        &[input],
        &StyleCatalog::default(),
        None,
        &ParseOptions::default(),
    )
    .unwrap();
    let prose: Vec<_> = output
        .blocks
        .iter()
        .filter(|block| block.text.as_deref().is_some_and(|text| !text.is_empty()))
        .collect();
    assert_eq!(prose[0].page_number, Some(1));
    assert_eq!(prose[1].page_number, Some(2));
    let table = output
        .blocks
        .iter()
        .find(|block| block.table.is_some())
        .unwrap();
    assert_eq!(table.page_number, Some(1));
    assert_eq!(output.page_evidence.len(), 2);
}

#[test]
fn malformed_table_section_preserves_neighbor_sections_transactionally() {
    let inputs = [
        section(
            "Contents/section0.xml",
            "<hp:p><hp:run><hp:t>first</hp:t></hp:run></hp:p>",
        ),
        section(
            "Contents/section1.xml",
            "<hp:tbl><hp:tr><hp:tc><hp:cellSpan rowSpan=\"2\"/><hp:subList/></hp:tc></hp:tr></hp:tbl>",
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
    assert_eq!(
        output
            .blocks
            .iter()
            .map(|block| block.text.as_deref())
            .collect::<Vec<_>>(),
        [Some("first"), Some("third")]
    );
    assert_eq!(output.warnings.len(), 1);
    assert_eq!(output.warnings[0].code, WarningCode::PartialParse);
    assert_eq!(output.warnings[0].page, Some(2));
}

#[test]
fn malformed_section_rolls_back_staged_image_assets_and_cache() {
    use crate::hwpx::package::Package;
    use std::io::{Cursor, Write};
    use zip::{ZipWriter, write::SimpleFileOptions};
    let inputs = [
        section(
            "Contents/section0.xml",
            "<hp:p><hp:run><hp:t>first</hp:t></hp:run></hp:p>",
        ),
        section(
            "Contents/section1.xml",
            "<hp:tbl><hp:tr><hp:tc><hp:subList><hp:p><hp:run><hp:pic><hp:imgRect binaryItemIDRef=\"pic\"/></hp:pic></hp:run></hp:p></hp:subList></hp:tc><hp:tc><hp:cellSpan rowSpan=\"2\"/><hp:subList/></hp:tc></hp:tr></hp:tbl>",
        ),
        section(
            "Contents/section2.xml",
            "<hp:p><hp:run><hp:pic><hp:imgRect binaryItemIDRef=\"pic\"/></hp:pic></hp:run></hp:p>",
        ),
    ];
    let mut writer = ZipWriter::new(Cursor::new(Vec::new()));
    writer
        .start_file("BinData/pic.png", SimpleFileOptions::default())
        .unwrap();
    writer.write_all(b"asset").unwrap();
    let mut package = Package::open(Box::leak(
        writer.finish().unwrap().into_inner().into_boxed_slice(),
    ))
    .unwrap();
    let output = lower_sections_with_package(
        &inputs,
        &StyleCatalog::default(),
        None,
        &ParseOptions::default(),
        &mut package,
    )
    .unwrap();
    assert_eq!(output.images.len(), 1);
    assert_eq!(
        output
            .blocks
            .iter()
            .filter(|block| block.kind == IrBlockType::Image)
            .count(),
        1
    );
    assert_eq!(
        output
            .warnings
            .iter()
            .filter(|warning| warning.code == WarningCode::PartialParse)
            .count(),
        1
    );
}

#[test]
fn distinct_image_refs_keep_unique_filenames_across_sections() {
    use crate::hwpx::package::Package;
    use std::io::{Cursor, Write};
    use zip::{ZipWriter, write::SimpleFileOptions};
    let inputs = [
        section(
            "Contents/section0.xml",
            "<hp:p><hp:linesegarray><hp:lineseg vertpos=\"0\"/></hp:linesegarray><hp:run><hp:pic><hp:imgRect binaryItemIDRef=\"one\"/></hp:pic></hp:run></hp:p>",
        ),
        section(
            "Contents/section1.xml",
            "<hp:p><hp:linesegarray><hp:lineseg vertpos=\"0\"/></hp:linesegarray><hp:run><hp:pic><hp:imgRect binaryItemIDRef=\"two\"/></hp:pic></hp:run></hp:p>",
        ),
    ];
    let mut writer = ZipWriter::new(Cursor::new(Vec::new()));
    for path in ["BinData/one.png", "BinData/two.png"] {
        writer
            .start_file(path, SimpleFileOptions::default())
            .unwrap();
        writer.write_all(b"asset").unwrap();
    }
    let mut package = Package::open(Box::leak(
        writer.finish().unwrap().into_inner().into_boxed_slice(),
    ))
    .unwrap();
    let output = lower_sections_with_package(
        &inputs,
        &StyleCatalog::default(),
        None,
        &ParseOptions::default(),
        &mut package,
    )
    .unwrap();
    assert_eq!(
        output
            .images
            .iter()
            .map(|image| image.filename.as_str())
            .collect::<Vec<_>>(),
        ["image_001.png", "image_002.png"]
    );
}

#[test]
fn skipped_image_warning_uses_resolved_page() {
    use crate::hwpx::package::Package;
    use std::io::Cursor;
    use zip::{ZipWriter, write::SimpleFileOptions};
    let input = section(
        "Contents/section0.xml",
        "<hp:p><hp:linesegarray><hp:lineseg vertpos=\"0\"/></hp:linesegarray><hp:run><hp:pic><hp:imgRect binaryItemIDRef=\"missing\"/></hp:pic></hp:run></hp:p>",
    );
    let mut writer = ZipWriter::new(Cursor::new(Vec::new()));
    writer
        .start_file("unused", SimpleFileOptions::default())
        .unwrap();
    let mut package = Package::open(Box::leak(
        writer.finish().unwrap().into_inner().into_boxed_slice(),
    ))
    .unwrap();
    let output = lower_sections_with_package(
        &[input],
        &StyleCatalog::default(),
        None,
        &ParseOptions::default(),
        &mut package,
    )
    .unwrap();
    let warning = output
        .warnings
        .iter()
        .find(|warning| warning.code == WarningCode::SkippedImage)
        .unwrap();
    assert_eq!(warning.page, Some(1));
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

#[test]
fn click_here_guide_is_hidden_by_default_and_kept_when_requested() {
    let input = section(
        "Contents/section0.xml",
        "<hp:p><hp:run><hp:t>Label </hp:t></hp:run><hp:ctrl><hp:fieldBegin type=\"CLICK_HERE\" dirty=\"0\"><hp:parameters><hp:stringParam name=\"Command\">Direction:wstring:10:Enter name</hp:stringParam></hp:parameters></hp:fieldBegin></hp:ctrl><hp:run><hp:t>Enter name</hp:t></hp:run><hp:ctrl><hp:fieldEnd/></hp:ctrl></hp:p>",
    );
    let lower = |include_field_placeholders| {
        lower_sections(
            std::slice::from_ref(&input),
            &StyleCatalog::default(),
            None,
            &ParseOptions {
                include_field_placeholders,
                ..ParseOptions::default()
            },
        )
        .unwrap()
    };

    for option in [None, Some(false)] {
        let output = lower(option);
        assert_eq!(output.blocks[0].text.as_deref(), Some("Label Enter name"));
        let spans = output.blocks[0]
            .spans
            .as_deref()
            .expect("default output marks the guide for Markdown omission");
        assert_eq!(spans.len(), 2);
        assert_eq!(spans[0].text, "Label ");
        assert_eq!(spans[0].placeholder, None);
        assert_eq!(spans[1].text, "Enter name");
        assert_eq!(spans[1].placeholder, Some(true));
    }

    let included = lower(Some(true));
    assert_eq!(included.blocks[0].text.as_deref(), Some("Label Enter name"));
    assert!(included.blocks[0].spans.is_none());
}

#[test]
fn anchored_trailing_empty_column_is_kept_only_when_requested() {
    let input = section(
        "Contents/section0.xml",
        "<hp:tbl><hp:tr><hp:tc><hp:cellAddr rowAddr=\"0\" colAddr=\"0\"/><hp:subList><hp:p><hp:run><hp:t>Value</hp:t></hp:run></hp:p></hp:subList></hp:tc><hp:tc><hp:cellAddr rowAddr=\"0\" colAddr=\"1\"/><hp:subList><hp:p><hp:run><hp:t> </hp:t></hp:run></hp:p></hp:subList></hp:tc></hp:tr></hp:tbl>",
    );
    let columns = |keep_trailing_empty_cols| {
        lower_sections(
            std::slice::from_ref(&input),
            &StyleCatalog::default(),
            None,
            &ParseOptions {
                keep_trailing_empty_cols,
                ..ParseOptions::default()
            },
        )
        .unwrap()
        .blocks[0]
            .table
            .as_ref()
            .unwrap()
            .cols
    };

    assert_eq!(columns(None), 1);
    assert_eq!(columns(Some(false)), 1);
    assert_eq!(columns(Some(true)), 2);
}

#[test]
fn field_placeholder_in_table_cell_obeys_option() {
    let input = section(
        "Contents/section0.xml",
        "<hp:tbl><hp:tr><hp:tc><hp:cellAddr rowAddr=\"0\" colAddr=\"0\"/><hp:subList><hp:p><hp:run><hp:t>Field: </hp:t></hp:run><hp:ctrl><hp:fieldBegin type=\"CLICK_HERE\" dirty=\"0\"><hp:parameters><hp:stringParam name=\"Direction\">Enter name</hp:stringParam></hp:parameters></hp:fieldBegin></hp:ctrl><hp:run><hp:t>Enter name</hp:t></hp:run><hp:ctrl><hp:fieldEnd/></hp:ctrl></hp:p></hp:subList></hp:tc><hp:tc><hp:cellAddr rowAddr=\"0\" colAddr=\"1\"/><hp:subList><hp:p><hp:run><hp:t>Second</hp:t></hp:run></hp:p></hp:subList></hp:tc></hp:tr></hp:tbl>",
    );
    let cell = |include_field_placeholders| {
        lower_sections(
            std::slice::from_ref(&input),
            &StyleCatalog::default(),
            None,
            &ParseOptions {
                include_field_placeholders,
                ..ParseOptions::default()
            },
        )
        .unwrap()
        .blocks[0]
            .table
            .as_ref()
            .unwrap()
            .cells[0][0]
            .clone()
    };

    for options in [None, Some(false)] {
        let cell = cell(options);
        assert_eq!(cell.text, "Field: Enter name");
        let paragraph = cell
            .blocks
            .as_ref()
            .unwrap()
            .iter()
            .find(|block| block.kind == IrBlockType::Paragraph)
            .unwrap();
        assert_eq!(paragraph.spans.as_ref().unwrap()[1].placeholder, Some(true));
    }
    let included = cell(Some(true));
    assert_eq!(included.text, "Field: Enter name");
    assert!(included.blocks.is_none());
}

#[test]
fn nested_click_here_fields_do_not_match_through_inner_placeholders_in_body() {
    let input = section(
        "Contents/section0.xml",
        "<hp:p><hp:ctrl><hp:fieldBegin type=\"CLICK_HERE\" dirty=\"0\"><hp:parameters><hp:stringParam name=\"Direction\">AB</hp:stringParam></hp:parameters></hp:fieldBegin></hp:ctrl><hp:run><hp:t>A</hp:t></hp:run><hp:ctrl><hp:fieldBegin type=\"CLICK_HERE\" dirty=\"0\"><hp:parameters><hp:stringParam name=\"Direction\">B</hp:stringParam></hp:parameters></hp:fieldBegin></hp:ctrl><hp:run><hp:t>B</hp:t></hp:run><hp:ctrl><hp:fieldEnd/></hp:ctrl><hp:ctrl><hp:fieldEnd/></hp:ctrl></hp:p>",
    );
    let output = lower_sections(
        &[input],
        &StyleCatalog::default(),
        None,
        &ParseOptions::default(),
    )
    .unwrap();

    assert_eq!(output.blocks[0].text.as_deref(), Some("AB"));
    let spans = output.blocks[0].spans.as_ref().unwrap();
    assert_ne!(spans[0].placeholder, Some(true));
    assert_eq!(spans[1].placeholder, Some(true));
}

#[test]
fn nested_click_here_fields_do_not_match_through_inner_placeholders_in_table_cell() {
    let input = section(
        "Contents/section0.xml",
        "<hp:tbl><hp:tr><hp:tc><hp:cellAddr rowAddr=\"0\" colAddr=\"0\"/><hp:subList><hp:p><hp:ctrl><hp:fieldBegin type=\"CLICK_HERE\" dirty=\"0\"><hp:parameters><hp:stringParam name=\"Direction\">AB</hp:stringParam></hp:parameters></hp:fieldBegin></hp:ctrl><hp:run><hp:t>A</hp:t></hp:run><hp:ctrl><hp:fieldBegin type=\"CLICK_HERE\" dirty=\"0\"><hp:parameters><hp:stringParam name=\"Direction\">B</hp:stringParam></hp:parameters></hp:fieldBegin></hp:ctrl><hp:run><hp:t>B</hp:t></hp:run><hp:ctrl><hp:fieldEnd/></hp:ctrl><hp:ctrl><hp:fieldEnd/></hp:ctrl></hp:p></hp:subList></hp:tc><hp:tc><hp:cellAddr rowAddr=\"0\" colAddr=\"1\"/><hp:subList><hp:p><hp:run><hp:t>Second</hp:t></hp:run></hp:p></hp:subList></hp:tc></hp:tr></hp:tbl>",
    );
    let output = lower_sections(
        &[input],
        &StyleCatalog::default(),
        None,
        &ParseOptions::default(),
    )
    .unwrap();

    let cell = &output.blocks[0].table.as_ref().unwrap().cells[0][0];
    assert_eq!(cell.text, "AB");
    let paragraph = cell.blocks.as_ref().unwrap().first().unwrap();
    let spans = paragraph.spans.as_ref().unwrap();
    assert_ne!(spans[0].placeholder, Some(true));
    assert_eq!(spans[1].placeholder, Some(true));
}

#[test]
fn command_guides_use_utf16_lengths_for_supplementary_and_bmp_characters() {
    let input = section(
        "Contents/section0.xml",
        "<hp:p><hp:ctrl><hp:fieldBegin type=\"CLICK_HERE\" dirty=\"0\"><hp:parameters><hp:stringParam name=\"Command\">Direction:wstring:2:😀한글</hp:stringParam></hp:parameters></hp:fieldBegin></hp:ctrl><hp:run><hp:t>😀</hp:t></hp:run><hp:ctrl><hp:fieldEnd/></hp:ctrl></hp:p><hp:p><hp:ctrl><hp:fieldBegin type=\"CLICK_HERE\" dirty=\"0\"><hp:parameters><hp:stringParam name=\"Command\">Direction:wstring:3:😀한글</hp:stringParam></hp:parameters></hp:fieldBegin></hp:ctrl><hp:run><hp:t>😀한</hp:t></hp:run><hp:ctrl><hp:fieldEnd/></hp:ctrl></hp:p>",
    );
    let output = lower_sections(
        &[input],
        &StyleCatalog::default(),
        None,
        &ParseOptions::default(),
    )
    .unwrap();

    for (index, expected) in [(0, "😀"), (1, "😀한")] {
        assert_eq!(output.blocks[index].text.as_deref(), Some(expected));
        assert_eq!(
            output.blocks[index].spans.as_ref().unwrap()[0].placeholder,
            Some(true)
        );
    }
}

#[test]
fn command_length_cutting_a_surrogate_pair_does_not_mark_a_placeholder() {
    let input = section(
        "Contents/section0.xml",
        "<hp:p><hp:ctrl><hp:fieldBegin type=\"CLICK_HERE\" dirty=\"0\"><hp:parameters><hp:stringParam name=\"Command\">Direction:wstring:1:😀</hp:stringParam></hp:parameters></hp:fieldBegin></hp:ctrl><hp:run><hp:t>😀</hp:t></hp:run><hp:ctrl><hp:fieldEnd/></hp:ctrl></hp:p>",
    );
    let output = lower_sections(
        &[input],
        &StyleCatalog::default(),
        None,
        &ParseOptions::default(),
    )
    .unwrap();

    assert_eq!(output.blocks[0].text.as_deref(), Some("😀"));
    assert!(output.blocks[0].spans.is_none());
}

#[test]
fn empty_direction_parameter_stops_before_command_fallback() {
    let input = section(
        "Contents/section0.xml",
        "<hp:p><hp:ctrl><hp:fieldBegin type=\"CLICK_HERE\" dirty=\"0\"><hp:parameters><hp:stringParam name=\"Command\">Direction:wstring:10:Enter name</hp:stringParam><hp:stringParam name=\"Direction\"></hp:stringParam></hp:parameters></hp:fieldBegin></hp:ctrl><hp:run><hp:t>Enter name</hp:t></hp:run><hp:ctrl><hp:fieldEnd/></hp:ctrl></hp:p>",
    );
    let output = lower_sections(
        &[input],
        &StyleCatalog::default(),
        None,
        &ParseOptions::default(),
    )
    .unwrap();

    assert_eq!(output.blocks[0].text.as_deref(), Some("Enter name"));
    assert!(output.blocks[0].spans.is_none());
}

#[test]
fn escaped_literal_dollar_matches_click_here_guide() {
    let input = section(
        "Contents/section0.xml",
        "<hp:p><hp:ctrl><hp:fieldBegin type=\"CLICK_HERE\" dirty=\"0\"><hp:parameters><hp:stringParam name=\"Direction\">$HOME</hp:stringParam></hp:parameters></hp:fieldBegin></hp:ctrl><hp:run><hp:t>\\$HOME</hp:t></hp:run><hp:ctrl><hp:fieldEnd/></hp:ctrl></hp:p>",
    );
    let output = lower_sections(
        &[input],
        &StyleCatalog::default(),
        None,
        &ParseOptions::default(),
    )
    .unwrap();

    assert_eq!(output.blocks[0].text.as_deref(), Some("\\$HOME"));
    assert_eq!(
        output.blocks[0].spans.as_ref().unwrap()[0].placeholder,
        Some(true)
    );
}

#[test]
fn inline_table_paragraph_does_not_mark_fields_as_placeholders() {
    let input = section(
        "Contents/section0.xml",
        "<hp:p><hp:ctrl><hp:fieldBegin type=\"CLICK_HERE\" dirty=\"0\"><hp:parameters><hp:stringParam name=\"Direction\">Guide</hp:stringParam></hp:parameters></hp:fieldBegin></hp:ctrl><hp:run><hp:t>Before</hp:t></hp:run><hp:tbl><hp:pos treatAsChar=\"1\"/><hp:tr><hp:tc><hp:cellAddr rowAddr=\"0\" colAddr=\"0\"/><hp:subList><hp:p><hp:run><hp:t>Cell</hp:t></hp:run></hp:p></hp:subList></hp:tc></hp:tr></hp:tbl><hp:run><hp:t>Guide</hp:t></hp:run><hp:ctrl><hp:fieldEnd/></hp:ctrl></hp:p>",
    );
    let output = lower_sections(
        &[input],
        &StyleCatalog::default(),
        None,
        &ParseOptions::default(),
    )
    .unwrap();

    assert_eq!(output.blocks[0].text.as_deref(), Some("Before"));
    assert_eq!(output.blocks[1].kind, IrBlockType::Table);
    assert_eq!(output.blocks[2].text.as_deref(), Some("Guide"));
    assert!(output.blocks.iter().all(|block| {
        block
            .spans
            .as_ref()
            .is_none_or(|spans| spans.iter().all(|span| span.placeholder != Some(true)))
    }));
}

#[test]
fn floating_table_keeps_guide_tracking_across_span_flush() {
    let input = section(
        "Contents/section0.xml",
        "<hp:p><hp:run><hp:t>Lead</hp:t></hp:run><hp:ctrl><hp:fieldBegin type=\"CLICK_HERE\" dirty=\"0\"><hp:parameters><hp:stringParam name=\"Direction\">Guide</hp:stringParam></hp:parameters></hp:fieldBegin></hp:ctrl><hp:tbl><hp:pos treatAsChar=\"0\"/><hp:tr><hp:tc><hp:cellAddr rowAddr=\"0\" colAddr=\"0\"/><hp:subList><hp:p><hp:run><hp:t>Cell</hp:t></hp:run></hp:p></hp:subList></hp:tc></hp:tr></hp:tbl><hp:run><hp:t>Guide</hp:t></hp:run><hp:ctrl><hp:fieldEnd/></hp:ctrl></hp:p>",
    );
    let output = lower_sections(
        &[input],
        &StyleCatalog::default(),
        None,
        &ParseOptions::default(),
    )
    .unwrap();

    assert_eq!(output.blocks[0].text.as_deref(), Some("Lead"));
    assert_eq!(output.blocks[1].kind, IrBlockType::Table);
    assert_eq!(output.blocks[2].text.as_deref(), Some("Guide"));
    assert_eq!(
        output.blocks[2].spans.as_ref().unwrap()[0].placeholder,
        Some(true)
    );
}

#[test]
fn floating_table_guide_match_includes_text_flushed_before_the_table() {
    let input = section(
        "Contents/section0.xml",
        "<hp:p><hp:ctrl><hp:fieldBegin type=\"CLICK_HERE\" dirty=\"0\"><hp:parameters><hp:stringParam name=\"Direction\">Guide</hp:stringParam></hp:parameters></hp:fieldBegin></hp:ctrl><hp:run><hp:t>Prefix</hp:t></hp:run><hp:tbl><hp:pos treatAsChar=\"0\"/><hp:tr><hp:tc><hp:cellAddr rowAddr=\"0\" colAddr=\"0\"/><hp:subList><hp:p><hp:run><hp:t>Cell</hp:t></hp:run></hp:p></hp:subList></hp:tc></hp:tr></hp:tbl><hp:run><hp:t>Guide</hp:t></hp:run><hp:ctrl><hp:fieldEnd/></hp:ctrl></hp:p>",
    );
    let output = lower_sections(
        &[input],
        &StyleCatalog::default(),
        None,
        &ParseOptions::default(),
    )
    .unwrap();

    assert_eq!(output.blocks[0].text.as_deref(), Some("Prefix"));
    assert_eq!(output.blocks[1].kind, IrBlockType::Table);
    assert_eq!(output.blocks[2].text.as_deref(), Some("Guide"));
    assert!(output.blocks[0].spans.is_none());
    assert!(output.blocks[2].spans.is_none());
}

#[test]
fn floating_table_field_end_marks_matching_text_flushed_before_the_table() {
    let input = section(
        "Contents/section0.xml",
        "<hp:p><hp:ctrl><hp:fieldBegin type=\"CLICK_HERE\" dirty=\"0\"><hp:parameters><hp:stringParam name=\"Direction\">Guide</hp:stringParam></hp:parameters></hp:fieldBegin></hp:ctrl><hp:run><hp:t>Guide</hp:t></hp:run><hp:tbl><hp:pos treatAsChar=\"0\"/><hp:tr><hp:tc><hp:cellAddr rowAddr=\"0\" colAddr=\"0\"/><hp:subList><hp:p><hp:run><hp:t>Cell</hp:t></hp:run></hp:p></hp:subList></hp:tc></hp:tr></hp:tbl><hp:ctrl><hp:fieldEnd/></hp:ctrl></hp:p>",
    );
    let output = lower_sections(
        &[input],
        &StyleCatalog::default(),
        None,
        &ParseOptions::default(),
    )
    .unwrap();

    assert_eq!(output.blocks[0].text.as_deref(), Some("Guide"));
    assert_eq!(output.blocks[1].kind, IrBlockType::Table);
    assert_eq!(
        output.blocks[0].spans.as_ref().unwrap()[0].placeholder,
        Some(true)
    );
}

#[test]
fn many_sequential_fields_after_floating_tables_keep_local_ranges() {
    const FIELD_COUNT: usize = 512;
    let mut xml = String::from("<hp:p>");
    for _ in 0..FIELD_COUNT {
        xml.push_str(
            "<hp:ctrl><hp:fieldBegin type=\"CLICK_HERE\" dirty=\"0\"><hp:parameters><hp:stringParam name=\"Direction\">G</hp:stringParam></hp:parameters></hp:fieldBegin></hp:ctrl><hp:run><hp:t>G</hp:t></hp:run><hp:ctrl><hp:fieldEnd/></hp:ctrl><hp:tbl><hp:pos treatAsChar=\"0\"/><hp:tr><hp:tc><hp:cellAddr rowAddr=\"0\" colAddr=\"0\"/><hp:subList><hp:p><hp:run><hp:t>C</hp:t></hp:run></hp:p></hp:subList></hp:tc></hp:tr></hp:tbl>",
        );
    }
    xml.push_str("</hp:p>");
    let input = section("Contents/section0.xml", &xml);
    let output = lower_sections(
        &[input],
        &StyleCatalog::default(),
        None,
        &ParseOptions::default(),
    )
    .unwrap();

    assert_eq!(output.blocks.len(), FIELD_COUNT * 2);
    for pair in output.blocks.chunks_exact(2) {
        assert_eq!(pair[0].text.as_deref(), Some("G"));
        assert_eq!(pair[0].spans.as_ref().unwrap()[0].placeholder, Some(true));
        assert_eq!(pair[1].kind, IrBlockType::Table);
    }
}
