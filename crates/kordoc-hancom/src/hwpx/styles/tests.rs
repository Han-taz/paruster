use crate::hwpx::styles::StyleCatalog;

#[test]
fn parses_outline_and_inline_style_references() {
    let catalog = StyleCatalog::parse(
        br#"<head><paraPr id="3" outlineLvl="1"/><charPr id="8" bold="true"/></head>"#,
    )
    .unwrap();
    let section =
        crate::hwpx::xml::parse(b"<p paraPrIDRef=\"3\"><run charPrIDRef=\"8\"/></p>").unwrap();
    assert_eq!(catalog.paragraph_level(&section), Some(1));
    assert_eq!(
        catalog.character_style(&section.children[0]).unwrap().bold,
        Some(true)
    );
}
