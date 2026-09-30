#![no_main]

use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    let first = kordoc_core::fuzzing::hwpx_xml(data);
    let second = kordoc_core::fuzzing::hwpx_xml(data);
    assert_eq!(first, second);
    if let Ok(document) = first {
        assert_eq!(
            document
                .blocks
                .first()
                .and_then(|block| block.text.as_deref()),
            Some("before")
        );
        assert_eq!(
            document
                .blocks
                .last()
                .and_then(|block| block.text.as_deref()),
            Some("after")
        );
        if document
            .warnings
            .as_deref()
            .unwrap_or_default()
            .iter()
            .any(|warning| warning.code == kordoc_ir::WarningCode::PartialParse)
        {
            assert_eq!(
                document.blocks.len(),
                2,
                "failed middle section leaked partial blocks"
            );
        }
    }
});
