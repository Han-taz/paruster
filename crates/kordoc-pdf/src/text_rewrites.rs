//! Private, bounded PDF text-field rewrites captured from the migration oracle.

use kordoc_ir::{ErrorCode, KordocError};
use unicode_normalization::UnicodeNormalization;

use crate::v8_runtime::metadata::{is_ecmascript_whitespace, trim_ecmascript};

const MAX_TEXT_BYTES: usize = 64 * 1024;

/// Trim and apply the targeted source-order text rewrites to one bounded item.
/// The font size is the already-rounded scalar value from the text projection.
pub(crate) fn rewrite_text(text: &str, rounded_font_size: f64) -> Result<String, KordocError> {
    if text.len() > MAX_TEXT_BYTES {
        return Err(output_limit("PDF text rewrite input exceeds byte limit"));
    }
    if !rounded_font_size.is_finite() || rounded_font_size < 0.0 || rounded_font_size.fract() != 0.0
    {
        return Err(parse_error("invalid rounded PDF font size"));
    }

    let trimmed = trim_ecmascript(text);
    let mut output = String::new();
    output
        .try_reserve_exact(trimmed.len())
        .map_err(|_| output_limit("PDF text rewrite allocation failed"))?;

    for character in trimmed.chars() {
        if (0x2f00..=0x2fd5).contains(&(character as u32)) {
            let mut normalized = character.nfkc();
            let replacement = normalized
                .next()
                .ok_or_else(|| parse_error("empty Kangxi radical normalization"))?;
            if normalized.next().is_some() {
                return Err(parse_error(
                    "unexpected expanded Kangxi radical normalization",
                ));
            }
            if replacement.len_utf8() > character.len_utf8() {
                return Err(output_limit("Kangxi radical rewrite expands text output"));
            }
            output.push(replacement);
        } else {
            output.push(character);
        }
    }

    if is_numeric_rewrite_candidate(&output) {
        output.retain(|character| character != ' ');
    }
    if rounded_font_size >= 14.0 && is_spaced_uppercase_label(&output) {
        output.retain(|character| character != ' ');
    }
    if output.len() > MAX_TEXT_BYTES {
        return Err(output_limit("PDF text rewrite output exceeds byte limit"));
    }
    Ok(output)
}

fn is_numeric_rewrite_candidate(text: &str) -> bool {
    let mut has_ascii_digit = false;
    let mut has_ascii_space = false;
    let mut permitted = true;

    for character in text.chars() {
        has_ascii_digit |= character.is_ascii_digit();
        has_ascii_space |= character == ' ';
        permitted &= character.is_ascii_digit()
            || is_ecmascript_whitespace(character)
            || matches!(character, '-' | '(' | ')' | '.' | '·' | ',' | '☎');
    }

    permitted && has_ascii_digit && has_ascii_space
}

fn is_spaced_uppercase_label(text: &str) -> bool {
    let mut group_count = 0usize;
    for group in text.split(' ') {
        let mut characters = group.chars();
        let Some(character) = characters.next() else {
            return false;
        };
        if characters.next().is_some() || !is_uppercase_label_character(character) {
            return false;
        }
        group_count += 1;
    }
    group_count >= 3 && text.contains(' ')
}

fn is_uppercase_label_character(character: char) -> bool {
    character.is_ascii_uppercase()
        || character.is_ascii_digit()
        || matches!(character, '?' | '!' | '&' | '\'' | '’')
}

fn output_limit(message: &'static str) -> KordocError {
    KordocError::new(ErrorCode::OutputTooLarge, message)
}

fn parse_error(message: &'static str) -> KordocError {
    KordocError::new(ErrorCode::ParseError, message)
}
