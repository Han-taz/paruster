use crate::limits::{BudgetError, PdfBudget};
use std::fmt;

#[derive(Debug, PartialEq, Eq)]
pub enum StreamDecodeError {
    Budget(BudgetError),
    Malformed,
}

impl fmt::Display for StreamDecodeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Budget(error) => error.fmt(f),
            Self::Malformed => f.write_str("malformed PDF stream encoding"),
        }
    }
}

impl std::error::Error for StreamDecodeError {}

impl From<BudgetError> for StreamDecodeError {
    fn from(value: BudgetError) -> Self {
        Self::Budget(value)
    }
}

pub fn copy_unfiltered(input: &[u8], budget: &mut PdfBudget) -> Result<Vec<u8>, StreamDecodeError> {
    budget.charge_decoded(input.len() as u64)?;
    Ok(input.to_vec())
}

pub fn decode_ascii_hex(
    input: &[u8],
    budget: &mut PdfBudget,
) -> Result<Vec<u8>, StreamDecodeError> {
    let mut output = Vec::new();
    let mut high = None;
    for byte in input.iter().copied().take_while(|byte| *byte != b'>') {
        if byte.is_ascii_whitespace() {
            continue;
        }
        let nibble = (byte as char)
            .to_digit(16)
            .ok_or(StreamDecodeError::Malformed)? as u8;
        if let Some(first) = high.take() {
            budget.charge_decoded(1)?;
            output.push((first << 4) | nibble);
        } else {
            high = Some(nibble);
        }
    }
    if let Some(last) = high {
        budget.charge_decoded(1)?;
        output.push(last << 4);
    }
    Ok(output)
}
