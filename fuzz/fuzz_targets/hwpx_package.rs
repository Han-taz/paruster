#![no_main]

use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    let first = kordoc_core::fuzzing::hwpx_package(data);
    let second = kordoc_core::fuzzing::hwpx_package(data);
    assert_eq!(first, second);
    if !kordoc_core::fuzzing::preflight_zip(data) {
        assert!(
            first.is_err(),
            "direct HWPX parsing bypassed strict ZIP preflight"
        );
    }
    if let Ok(document) = first {
        let image_bytes = document
            .images
            .as_deref()
            .unwrap_or_default()
            .iter()
            .try_fold(0usize, |total, image| total.checked_add(image.data.len()))
            .expect("image-byte accounting overflowed");
        assert!(image_bytes <= 256 * 1024 * 1024);
        assert!(document.page_evidence.as_deref().unwrap_or_default().len() <= 100_000);
    }
    // Exercise the real core projections and their ordered option transforms,
    // independently of the private package-only result above.
    let options = kordoc_core::ParseOptions {
        password: Some("fixture-password".into()),
        plain: Some(true),
        html_tables: Some(true),
        script_tags: Some(false),
        keep_trailing_empty_cols: Some(true),
        include_field_placeholders: Some(false),
        ..kordoc_core::ParseOptions::default()
    };
    let projected = kordoc_core::parse_hwpx_with_options(data, &options);
    assert_eq!(
        projected,
        kordoc_core::parse_hwpx_with_options(data, &options)
    );
    if !kordoc_core::fuzzing::preflight_zip(data) {
        assert!(projected.is_err(), "option parsing bypassed ZIP preflight");
    }
    if let Ok(result) = projected {
        assert!(result.markdown.len() <= 256 * 1024 * 1024);
        if let Some(pages) = result.pages {
            assert!(pages.len() <= 100_000);
            assert!(
                pages
                    .iter()
                    .all(|page| page.markdown.len() <= 256 * 1024 * 1024)
            );
        }
    }
});
