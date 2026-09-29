#![no_main]

use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    let _ = kordoc_core::fuzzing::preflight_zip(data);
});
