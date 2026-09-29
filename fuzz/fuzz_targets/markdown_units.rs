#![no_main]

use libfuzzer_sys::fuzz_target;

const MAX_FUZZ_INPUT_BYTES: usize = 8 * 1024 * 1024;
const MAX_GFM_LINES: usize = 10_000;

fn exercise_markdown_units(input: &str) {
    if input.len() > MAX_FUZZ_INPUT_BYTES {
        return;
    }

    let _ = kordoc_core::markdown_units::split_markdown_units(input);
    let lines: Vec<String> = input
        .lines()
        .take(MAX_GFM_LINES)
        .map(str::to_owned)
        .collect();
    let _ = kordoc_core::markdown_units::parse_gfm_table(&lines);
    let _ = kordoc_core::markdown_units::parse_html_table(input);
    let _ = kordoc_core::markdown_units::html_cell_inner_to_lines(input);
    let _ = kordoc_core::markdown_units::split_cell_by_top_level_tables(input);
}

fuzz_target!(|data: &[u8]| {
    if data.len() > MAX_FUZZ_INPUT_BYTES {
        return;
    }
    match std::str::from_utf8(data) {
        Ok(input) => exercise_markdown_units(input),
        Err(_) => exercise_markdown_units(&String::from_utf8_lossy(data)),
    }
});
