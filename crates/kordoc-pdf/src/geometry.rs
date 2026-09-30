//! Private projection of bounded PDF.js geometry into Rust page coordinates.

use kordoc_ir::{ErrorCode, KordocError};

use crate::v8_runtime::text_document::{PdfJsPage, PdfJsTextItem};

#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct PdfPageFrame {
    pub(crate) page_number: u32,
    pub(crate) origin_x: f64,
    pub(crate) origin_y: f64,
    pub(crate) width: f64,
    pub(crate) height: f64,
    pub(crate) rotation: i32,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct PdfTextPosition {
    pub(crate) x: f64,
    pub(crate) y: f64,
}

pub(crate) fn page_frame(page: &PdfJsPage) -> Result<PdfPageFrame, KordocError> {
    let [x1, y1, x2, y2] = page.view_box;
    let width = x2 - x1;
    let height = y2 - y1;
    if page.page_number == 0
        || page.rotation % 90 != 0
        || ![x1, y1, x2, y2, width, height]
            .into_iter()
            .all(f64::is_finite)
        || width <= 0.0
        || height <= 0.0
    {
        return Err(invalid_geometry("invalid PDF page frame"));
    }

    Ok(PdfPageFrame {
        page_number: page.page_number,
        origin_x: x1,
        origin_y: y1,
        width,
        height,
        rotation: page.rotation,
    })
}

pub(crate) fn base_text_position(
    frame: &PdfPageFrame,
    item: &PdfJsTextItem,
) -> Result<PdfTextPosition, KordocError> {
    if !item.transform.into_iter().all(f64::is_finite) {
        return Err(invalid_geometry("invalid PDF text transform"));
    }

    let x = js_math_round(item.transform[4]) - frame.origin_x;
    let y = js_math_round(item.transform[5]) - frame.origin_y;
    if !x.is_finite() || !y.is_finite() {
        return Err(invalid_geometry("invalid derived PDF text position"));
    }

    Ok(PdfTextPosition { x, y })
}

pub(crate) fn js_math_round(value: f64) -> f64 {
    let lower = value.floor();
    let rounded = if value - lower >= 0.5 {
        lower + 1.0
    } else {
        lower
    };

    if rounded == 0.0 && value.is_sign_negative() {
        -0.0
    } else {
        rounded
    }
}

fn invalid_geometry(message: &'static str) -> KordocError {
    KordocError::new(ErrorCode::ParseError, message)
}
