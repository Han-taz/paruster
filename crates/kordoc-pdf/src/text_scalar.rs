//! Bounded Rust projection of PDF.js text items to normalized base scalars.

use std::cmp::Ordering;

use kordoc_ir::{ErrorCode, KordocError};

use crate::geometry::js_math_round;
use crate::v8_runtime::{metadata::trim_ecmascript, text_document::PdfJsTextItem};

const MAX_ITEMS: usize = 100_000;
const MAX_ITEM_TEXT_BYTES: usize = 64 * 1024;
const MAX_TEXT_BYTES: usize = 2 * 1024 * 1024;
const MAX_FONT_NAME_BYTES: usize = 128;
const MAX_FONT_NAMES_BYTES: usize = 512 * 1024;

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct PdfBaseTextItem {
    pub(crate) text: String,
    pub(crate) x: f64,
    pub(crate) y: f64,
    pub(crate) width: f64,
    pub(crate) height: f64,
    pub(crate) font_size: f64,
    pub(crate) font_name: String,
    pub(crate) is_hidden: bool,
    pub(crate) sequence: usize,
    pub(crate) rotated_length: Option<f64>,
}

pub(crate) fn project_base_items(
    items: Vec<PdfJsTextItem>,
) -> Result<Vec<PdfBaseTextItem>, KordocError> {
    preflight(&items)?;

    let mut output = Vec::new();
    output
        .try_reserve_exact(items.len())
        .map_err(|_| output_limit("PDF text item allocation failed"))?;

    for (index, item) in items.into_iter().enumerate() {
        let text = trim_ecmascript(&item.text);
        if text.is_empty() {
            continue;
        }

        let [a, b, c, d, e, f] = item.transform;
        let scale_x = js_math_hypot(a, b);
        let scale_y = js_math_hypot(c, d);
        let font_size = js_math_round(scale_x.max(scale_y));
        let vertical = b.abs() > a.abs() * 4.0;
        let mut x = js_math_round(e);
        let y = js_math_round(f);
        let height = js_math_round(item.height);
        let rounded_width = js_math_round(item.width);
        let (width, rotated_length) = if vertical {
            let width = 1.0_f64.max(font_size);
            if b > 0.0 {
                x -= width;
            }
            (width, Some(1.0_f64.max(rounded_width)))
        } else {
            (rounded_width, None)
        };
        if ![scale_x, scale_y, font_size, x, y, width, height]
            .into_iter()
            .all(f64::is_finite)
            || rotated_length.is_some_and(|value| !value.is_finite())
        {
            return Err(parse_error("invalid derived PDF text metrics"));
        }

        output.push(PdfBaseTextItem {
            text: try_copy(text)?,
            x,
            y,
            width,
            height,
            font_size,
            font_name: try_copy(&item.font_name)?,
            is_hidden: font_size == 0.0 || (item.width == 0.0 && !text.is_empty()),
            sequence: index + 1,
            rotated_length,
        });
    }

    output.sort_unstable_by(|left, right| {
        numeric_order(right.y, left.y)
            .then_with(|| numeric_order(left.x, right.x))
            .then_with(|| left.sequence.cmp(&right.sequence))
    });
    Ok(output)
}

fn preflight(items: &[PdfJsTextItem]) -> Result<(), KordocError> {
    if items.len() > MAX_ITEMS {
        return Err(output_limit("PDF text item count exceeded"));
    }
    let mut text_bytes = 0usize;
    let mut font_bytes = 0usize;
    for item in items {
        if !item
            .transform
            .into_iter()
            .chain([item.width, item.height])
            .all(f64::is_finite)
        {
            return Err(parse_error("invalid PDF text item geometry"));
        }
        if item.text.len() > MAX_ITEM_TEXT_BYTES {
            return Err(output_limit("PDF text item byte limit exceeded"));
        }
        if item.font_name.len() > MAX_FONT_NAME_BYTES {
            return Err(output_limit("PDF font name byte limit exceeded"));
        }
        text_bytes = text_bytes
            .checked_add(item.text.len())
            .ok_or_else(|| output_limit("PDF text aggregate byte limit exceeded"))?;
        font_bytes = font_bytes
            .checked_add(item.font_name.len())
            .ok_or_else(|| output_limit("PDF font aggregate byte limit exceeded"))?;
        if text_bytes > MAX_TEXT_BYTES {
            return Err(output_limit("PDF text aggregate byte limit exceeded"));
        }
        if font_bytes > MAX_FONT_NAMES_BYTES {
            return Err(output_limit("PDF font aggregate byte limit exceeded"));
        }
    }
    Ok(())
}

fn numeric_order(left: f64, right: f64) -> Ordering {
    left.partial_cmp(&right)
        .expect("all projected coordinates were checked finite")
}

/// Match V8 152.2.0's two-argument Math.hypot fast path for finite inputs,
/// including separately rounded arithmetic (no fused operations). Callers
/// must validate the arguments; V8 has additional NaN/infinity cases.
pub(crate) fn js_math_hypot(a: f64, b: f64) -> f64 {
    let a = a.abs();
    let b = b.abs();
    let max = a.max(b);
    if max == 0.0 || max.is_infinite() {
        return max;
    }
    let x = a / max;
    let y = b / max;
    let xx = x * x;
    let yy = y * y;
    let sum = xx + yy;
    sum.sqrt() * max
}

fn try_copy(value: &str) -> Result<String, KordocError> {
    let mut result = String::new();
    result
        .try_reserve_exact(value.len())
        .map_err(|_| output_limit("PDF text scalar allocation failed"))?;
    result.push_str(value);
    Ok(result)
}

fn output_limit(message: &'static str) -> KordocError {
    KordocError::new(ErrorCode::OutputTooLarge, message)
}

fn parse_error(message: &'static str) -> KordocError {
    KordocError::new(ErrorCode::ParseError, message)
}
