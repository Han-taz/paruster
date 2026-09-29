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

#[test]
fn parses_heading_child_and_only_real_strikeout_shapes() {
    let catalog = StyleCatalog::parse(
        br#"<head><paraPr id="3" outlineLvl="0"><heading type="OUTLINE" level="0"/></paraPr><charPr id="8"><strikeout/></charPr><charPr id="9" strike="true"><strike/></charPr></head>"#,
    )
    .unwrap();
    let section =
        crate::hwpx::xml::parse(b"<p paraPrIDRef=\"3\"><run charPrIDRef=\"8\"/></p>").unwrap();
    assert_eq!(catalog.paragraph_level(&section), Some(1));
    assert_eq!(
        catalog
            .character_style(&section.children[0])
            .unwrap()
            .strike,
        Some(true)
    );
    let false_positive = crate::hwpx::xml::parse(b"<run charPrIDRef=\"9\"/>").unwrap();
    assert_eq!(
        catalog.character_style(&false_positive).unwrap().strike,
        None
    );
}
