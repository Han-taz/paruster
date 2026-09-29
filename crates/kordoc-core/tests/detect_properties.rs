use kordoc_core::detect_format;
use proptest::prelude::*;

proptest! {
    #![proptest_config(ProptestConfig::with_cases(128))]

    #[test]
    fn bounded_arbitrary_bytes_are_panic_free_and_deterministic(bytes in prop::collection::vec(any::<u8>(), 0..4096)) {
        let first = detect_format(&bytes);
        let second = detect_format(&bytes);
        prop_assert_eq!(first, second);
    }
}
