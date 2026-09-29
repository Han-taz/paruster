use kordoc_ir::{ErrorCode, KordocError};

pub const MAX_INPUT_BYTES: usize = 524_288_000;
pub const MAX_ARCHIVE_ENTRIES: u64 = 100_000;
pub const MAX_UNCOMPRESSED_BYTES: u64 = 1_073_741_824;

pub(crate) fn validate_input_len(len: usize) -> Result<(), KordocError> {
    if len > MAX_INPUT_BYTES {
        Err(KordocError::new(
            ErrorCode::OutputTooLarge,
            format!("Input exceeds the {MAX_INPUT_BYTES}-byte limit"),
        ))
    } else {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn input_limit_is_inclusive_and_rejects_the_next_byte() {
        assert_eq!(validate_input_len(MAX_INPUT_BYTES), Ok(()));
        assert_eq!(
            validate_input_len(MAX_INPUT_BYTES + 1).unwrap_err().code,
            ErrorCode::OutputTooLarge
        );
    }
}
