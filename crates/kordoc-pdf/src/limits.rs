use std::fmt;

pub const MAX_SOURCE_BYTES: u64 = 524_288_000;
pub const MAX_OBJECTS: u64 = 1_000_000;
pub const MAX_DEREFERENCES: u64 = 2_000_000;
pub const MAX_OBJECT_DEPTH: u32 = 64;
pub const MAX_FORM_DEPTH: u32 = 32;
pub const MAX_STREAM_BYTES: u64 = 32 * 1024 * 1024;
pub const MAX_DECODED_BYTES: u64 = 256 * 1024 * 1024;
pub const MAX_OPERATORS_PER_PAGE: u64 = 500_000;
pub const MAX_OPERATORS_TOTAL: u64 = 5_000_000;
pub const MAX_GLYPHS_PER_PAGE: u64 = 1_000_000;
pub const MAX_GLYPHS_TOTAL: u64 = 10_000_000;
pub const MAX_TEXT_BYTES: u64 = 100 * 1024 * 1024;
pub const MAX_IR_BYTES: u64 = 256 * 1024 * 1024;
pub const MAX_PIXELS: u64 = 36_000_000;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BudgetError {
    SourceBytes,
    Objects,
    Dereferences,
    ObjectDepth,
    FormDepth,
    StreamBytes,
    DecodedBytes,
    PageOperators,
    TotalOperators,
    PageGlyphs,
    TotalGlyphs,
    TextBytes,
    IrBytes,
    Pixels,
    ArithmeticOverflow,
}

impl fmt::Display for BudgetError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "PDF resource limit exceeded: {self:?}")
    }
}

impl std::error::Error for BudgetError {}

#[derive(Debug, Default, Clone)]
pub struct PdfBudget {
    objects: u64,
    dereferences: u64,
    object_depth: u32,
    form_depth: u32,
    stream_bytes: u64,
    decoded_bytes: u64,
    total_operators: u64,
    page_operators: u64,
    total_glyphs: u64,
    page_glyphs: u64,
    text_bytes: u64,
    ir_bytes: u64,
    pixels: u64,
}

fn add(counter: &mut u64, amount: u64, limit: u64, error: BudgetError) -> Result<(), BudgetError> {
    let next = counter
        .checked_add(amount)
        .ok_or(BudgetError::ArithmeticOverflow)?;
    if next > limit {
        return Err(error);
    }
    *counter = next;
    Ok(())
}

impl PdfBudget {
    pub fn charge_source(&self, bytes: u64) -> Result<(), BudgetError> {
        if bytes > MAX_SOURCE_BYTES {
            Err(BudgetError::SourceBytes)
        } else {
            Ok(())
        }
    }

    pub fn charge_object(&mut self) -> Result<(), BudgetError> {
        add(&mut self.objects, 1, MAX_OBJECTS, BudgetError::Objects)
    }

    pub fn charge_objects(&mut self, count: u64) -> Result<(), BudgetError> {
        add(&mut self.objects, count, MAX_OBJECTS, BudgetError::Objects)
    }

    pub fn charge_deref(&mut self) -> Result<(), BudgetError> {
        add(
            &mut self.dereferences,
            1,
            MAX_DEREFERENCES,
            BudgetError::Dereferences,
        )
    }

    pub fn enter_object(&mut self) -> Result<(), BudgetError> {
        let next = self
            .object_depth
            .checked_add(1)
            .ok_or(BudgetError::ArithmeticOverflow)?;
        if next > MAX_OBJECT_DEPTH {
            return Err(BudgetError::ObjectDepth);
        }
        self.object_depth = next;
        Ok(())
    }

    pub fn leave_object(&mut self) {
        self.object_depth = self.object_depth.saturating_sub(1);
    }

    pub fn enter_form(&mut self) -> Result<(), BudgetError> {
        let next = self
            .form_depth
            .checked_add(1)
            .ok_or(BudgetError::ArithmeticOverflow)?;
        if next > MAX_FORM_DEPTH {
            return Err(BudgetError::FormDepth);
        }
        self.form_depth = next;
        Ok(())
    }

    pub fn leave_form(&mut self) {
        self.form_depth = self.form_depth.saturating_sub(1);
    }

    pub fn with_object<T>(
        &mut self,
        f: impl FnOnce(&mut Self) -> Result<T, BudgetError>,
    ) -> Result<T, BudgetError> {
        self.enter_object()?;
        let result = f(self);
        self.leave_object();
        result
    }

    pub fn with_form<T>(
        &mut self,
        f: impl FnOnce(&mut Self) -> Result<T, BudgetError>,
    ) -> Result<T, BudgetError> {
        self.enter_form()?;
        let result = f(self);
        self.leave_form();
        result
    }

    pub fn charge_decoded(&mut self, bytes: u64) -> Result<(), BudgetError> {
        let stream_next = self
            .stream_bytes
            .checked_add(bytes)
            .ok_or(BudgetError::ArithmeticOverflow)?;
        if stream_next > MAX_STREAM_BYTES {
            return Err(BudgetError::StreamBytes);
        }
        let total_next = self
            .decoded_bytes
            .checked_add(bytes)
            .ok_or(BudgetError::ArithmeticOverflow)?;
        if total_next > MAX_DECODED_BYTES {
            return Err(BudgetError::DecodedBytes);
        }
        self.stream_bytes = stream_next;
        self.decoded_bytes = total_next;
        Ok(())
    }

    pub fn finish_stream(&mut self) {
        self.stream_bytes = 0;
    }

    pub fn charge_operator(&mut self) -> Result<(), BudgetError> {
        add(
            &mut self.page_operators,
            1,
            MAX_OPERATORS_PER_PAGE,
            BudgetError::PageOperators,
        )?;
        add(
            &mut self.total_operators,
            1,
            MAX_OPERATORS_TOTAL,
            BudgetError::TotalOperators,
        )
    }

    pub fn finish_page(&mut self) {
        self.page_operators = 0;
        self.page_glyphs = 0;
    }

    pub fn charge_glyph(&mut self) -> Result<(), BudgetError> {
        add(
            &mut self.page_glyphs,
            1,
            MAX_GLYPHS_PER_PAGE,
            BudgetError::PageGlyphs,
        )?;
        add(
            &mut self.total_glyphs,
            1,
            MAX_GLYPHS_TOTAL,
            BudgetError::TotalGlyphs,
        )
    }

    pub fn charge_text(&mut self, bytes: u64) -> Result<(), BudgetError> {
        add(
            &mut self.text_bytes,
            bytes,
            MAX_TEXT_BYTES,
            BudgetError::TextBytes,
        )
    }

    pub fn charge_ir(&mut self, bytes: u64) -> Result<(), BudgetError> {
        add(
            &mut self.ir_bytes,
            bytes,
            MAX_IR_BYTES,
            BudgetError::IrBytes,
        )
    }

    pub fn charge_pixels(&mut self, pixels: u64) -> Result<(), BudgetError> {
        if pixels > MAX_PIXELS {
            Err(BudgetError::Pixels)
        } else {
            Ok(())
        }
    }
}
