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
});
