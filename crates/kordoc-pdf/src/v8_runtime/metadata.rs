//! Rust-owned normalization of the bounded raw PDF.js Info projection.

use kordoc_ir::{DocumentMetadata, ErrorCode, KordocError, PageMode};

use super::text_document::PdfJsMetadata;

const MAX_METADATA_VALUE_BYTES: usize = 4 * 1024;
const MAX_METADATA_BYTES: usize = 16 * 1024;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum PdfMetadataMode {
    FullParse,
    MetadataOnly,
}

/// Convert the raw Info dictionary strings to the existing IR metadata shape.
///
/// The input is checked again here because this helper is also callable from
/// Rust with a DTO that did not pass through the bounded worker decoder.
pub(crate) fn normalize_pdf_metadata(
    raw: &PdfJsMetadata,
    page_count: u32,
    mode: PdfMetadataMode,
) -> Result<DocumentMetadata, KordocError> {
    validate_raw_metadata(raw)?;

    Ok(DocumentMetadata {
        title: trimmed_optional(raw.title.as_deref())?,
        author: trimmed_optional(raw.author.as_deref())?,
        creator: trimmed_optional(raw.creator.as_deref())?,
        created_at: date_optional(raw.creation_date.as_deref())?,
        modified_at: date_optional(raw.modified_date.as_deref())?,
        page_count: Some(page_count),
        page_mode: match mode {
            PdfMetadataMode::FullParse => Some(PageMode::Layout),
            PdfMetadataMode::MetadataOnly => None,
        },
        version: None,
        description: trimmed_optional(raw.subject.as_deref())?,
        keywords: normalized_keywords(raw.keywords.as_deref())?,
    })
}

fn validate_raw_metadata(raw: &PdfJsMetadata) -> Result<(), KordocError> {
    let values = [
        raw.title.as_deref(),
        raw.author.as_deref(),
        raw.creator.as_deref(),
        raw.subject.as_deref(),
        raw.keywords.as_deref(),
        raw.creation_date.as_deref(),
        raw.modified_date.as_deref(),
    ];
    let mut total = 0usize;
    for value in values.into_iter().flatten() {
        if value.len() > MAX_METADATA_VALUE_BYTES {
            return Err(output_limit("PDF metadata field exceeds byte limit"));
        }
        total = total
            .checked_add(value.len())
            .ok_or_else(|| output_limit("PDF metadata aggregate exceeds byte limit"))?;
        if total > MAX_METADATA_BYTES {
            return Err(output_limit("PDF metadata aggregate exceeds byte limit"));
        }
    }
    Ok(())
}

fn trimmed_optional(value: Option<&str>) -> Result<Option<String>, KordocError> {
    let Some(value) = value else {
        return Ok(None);
    };
    let value = trim_ecmascript(value);
    if value.is_empty() {
        return Ok(None);
    }
    copy_string(value).map(Some)
}

fn normalized_keywords(value: Option<&str>) -> Result<Option<Vec<String>>, KordocError> {
    let Some(value) = value else {
        return Ok(None);
    };
    if trim_ecmascript(value).is_empty() {
        return Ok(None);
    }

    // This field is capped at 4 KiB before splitting. Since every nonempty
    // token needs at least one byte and adjacent tokens need a one-byte
    // separator, at most 2,048 tokens can be emitted.
    let mut keywords = Vec::new();
    for keyword in value.split([',', ';']) {
        let keyword = trim_ecmascript(keyword);
        if keyword.is_empty() {
            continue;
        }
        keywords
            .try_reserve(1)
            .map_err(|_| output_limit("PDF metadata keyword allocation failed"))?;
        keywords.push(copy_string(keyword)?);
    }
    Ok(Some(keywords))
}

fn date_optional(value: Option<&str>) -> Result<Option<String>, KordocError> {
    let Some(value) = value else {
        return Ok(None);
    };
    pdf_date(value)
}

/// Mirrors `/D:(\d{4})(\d{2})?(\d{2})?(\d{2})?(\d{2})?(\d{2})?/`.
fn pdf_date(value: &str) -> Result<Option<String>, KordocError> {
    let bytes = value.as_bytes();
    if bytes.len() < 6 {
        return Ok(None);
    }

    let mut found = None;
    for start in 0..=bytes.len() - 6 {
        if bytes[start] == b'D'
            && bytes[start + 1] == b':'
            && bytes[start + 2..start + 6].iter().all(u8::is_ascii_digit)
        {
            found = Some(start);
            break;
        }
    }
    let Some(start) = found else {
        return Ok(None);
    };

    let mut position = start + 6;
    let month = take_date_pair(bytes, &mut position, b"01");
    let day = take_date_pair(bytes, &mut position, b"01");
    let hour = take_date_pair(bytes, &mut position, b"00");
    let minute = take_date_pair(bytes, &mut position, b"00");
    let second = take_date_pair(bytes, &mut position, b"00");

    let mut result = String::new();
    result
        .try_reserve_exact(19)
        .map_err(|_| output_limit("PDF metadata date allocation failed"))?;
    push_ascii(&mut result, &bytes[start + 2..start + 6]);
    result.push('-');
    push_ascii(&mut result, month);
    result.push('-');
    push_ascii(&mut result, day);
    result.push('T');
    push_ascii(&mut result, hour);
    result.push(':');
    push_ascii(&mut result, minute);
    result.push(':');
    push_ascii(&mut result, second);
    Ok(Some(result))
}

fn take_date_pair<'a>(
    bytes: &'a [u8],
    position: &mut usize,
    default: &'static [u8; 2],
) -> &'a [u8] {
    if bytes
        .get(*position..position.saturating_add(2))
        .is_some_and(|pair| pair.iter().all(u8::is_ascii_digit))
    {
        let start = *position;
        *position += 2;
        &bytes[start..start + 2]
    } else {
        default
    }
}

fn push_ascii(output: &mut String, bytes: &[u8]) {
    for byte in bytes {
        debug_assert!(byte.is_ascii());
        output.push(char::from(*byte));
    }
}

fn copy_string(value: &str) -> Result<String, KordocError> {
    let mut output = String::new();
    output
        .try_reserve_exact(value.len())
        .map_err(|_| output_limit("PDF metadata string allocation failed"))?;
    output.push_str(value);
    Ok(output)
}

pub(crate) fn trim_ecmascript(value: &str) -> &str {
    value.trim_matches(is_ecmascript_whitespace)
}

fn is_ecmascript_whitespace(character: char) -> bool {
    matches!(
        character,
        '\u{0009}'..='\u{000d}'
            | '\u{0020}'
            | '\u{00a0}'
            | '\u{1680}'
            | '\u{2000}'..='\u{200a}'
            | '\u{2028}'
            | '\u{2029}'
            | '\u{202f}'
            | '\u{205f}'
            | '\u{3000}'
            | '\u{feff}'
    )
}

fn output_limit(message: &'static str) -> KordocError {
    KordocError::new(ErrorCode::OutputTooLarge, message)
}
