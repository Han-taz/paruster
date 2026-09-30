//! Aggregate, pre-allocation budget for HWPX source-neutral lowering.

use kordoc_ir::{ErrorCode, KordocError};

use super::xml::{XmlContent, XmlNode};

const MAX_LOWERING_BYTES: usize = 256 * 1024 * 1024;

/// Accounts for both retained IR text and attacker-multipliable transient IR storage.
/// The ZIP plaintext, XML tree, image-output, and logical-cell guards remain independent.
#[derive(Debug, Clone, Copy)]
pub(crate) struct LoweringBudget {
    used: usize,
    limit: usize,
}

impl Default for LoweringBudget {
    fn default() -> Self {
        Self {
            used: 0,
            limit: MAX_LOWERING_BYTES,
        }
    }
}

impl LoweringBudget {
    #[cfg(test)]
    pub(crate) fn with_limit(limit: usize) -> Self {
        Self { used: 0, limit }
    }

    pub(crate) fn checkpoint(&self) -> usize {
        self.used
    }

    pub(crate) fn rollback(&mut self, checkpoint: usize) {
        self.used = checkpoint.min(self.used);
    }

    pub(crate) fn charge_bytes(&mut self, bytes: usize) -> Result<(), KordocError> {
        let next = self.used.checked_add(bytes).ok_or_else(output_limit)?;
        if next > self.limit {
            return Err(output_limit());
        }
        self.used = next;
        Ok(())
    }

    pub(crate) fn charge_items<T>(&mut self, count: usize) -> Result<(), KordocError> {
        let bytes = std::mem::size_of::<T>()
            .checked_mul(count)
            .ok_or_else(output_limit)?;
        self.charge_bytes(bytes)
    }

    pub(crate) fn copy_str(&mut self, source: &str) -> Result<String, KordocError> {
        self.charge_bytes(source.len())?;
        let mut text = String::new();
        text.try_reserve_exact(source.len())
            .map_err(|_| output_limit())?;
        text.push_str(source);
        Ok(text)
    }

    pub(crate) fn append_str(
        &mut self,
        target: &mut String,
        source: &str,
    ) -> Result<(), KordocError> {
        self.charge_bytes(source.len())?;
        target
            .try_reserve_exact(source.len())
            .map_err(|_| output_limit())?;
        target.push_str(source);
        Ok(())
    }

    pub(crate) fn raw_xml_text(&mut self, node: &XmlNode) -> Result<String, KordocError> {
        fn visit(
            node: &XmlNode,
            budget: &mut LoweringBudget,
            out: &mut String,
        ) -> Result<(), KordocError> {
            for part in &node.content {
                match part {
                    XmlContent::Text { start, end } => {
                        budget.append_str(out, &node.text[*start..*end])?
                    }
                    XmlContent::Child(index) => visit(&node.children[*index], budget, out)?,
                }
            }
            Ok(())
        }
        let mut text = String::new();
        visit(node, self, &mut text)?;
        Ok(text)
    }

    pub(crate) fn push<T>(&mut self, target: &mut Vec<T>, item: T) -> Result<(), KordocError> {
        self.charge_items::<T>(1)?;
        target.try_reserve_exact(1).map_err(|_| output_limit())?;
        target.push(item);
        Ok(())
    }

    pub(crate) fn join_strings(
        &mut self,
        values: &[String],
        separator: &str,
    ) -> Result<String, KordocError> {
        let payload = values
            .iter()
            .try_fold(0usize, |total, value| total.checked_add(value.len()))
            .ok_or_else(output_limit)?;
        let separators = separator
            .len()
            .checked_mul(values.len().saturating_sub(1))
            .ok_or_else(output_limit)?;
        let total = payload.checked_add(separators).ok_or_else(output_limit)?;
        self.charge_bytes(total)?;
        let mut out = String::new();
        out.try_reserve_exact(total).map_err(|_| output_limit())?;
        for (index, value) in values.iter().enumerate() {
            if index > 0 {
                out.push_str(separator);
            }
            out.push_str(value);
        }
        Ok(out)
    }
}

pub(crate) fn output_limit() -> KordocError {
    KordocError::new(
        ErrorCode::OutputTooLarge,
        "HWPX lowering exceeds its allocation limit",
    )
}

#[cfg(test)]
mod tests {
    use super::LoweringBudget;
    use crate::hwpx::xml::parse;
    use kordoc_ir::{ErrorCode, IrBlock};

    #[test]
    fn inclusive_text_and_empty_block_boundary() {
        let block_bytes = std::mem::size_of::<IrBlock>();
        let mut budget = LoweringBudget::with_limit(block_bytes + 3);
        let copy = budget.copy_str("abc").unwrap();
        assert_eq!(copy, "abc");
        let mut blocks = Vec::new();
        budget.push(&mut blocks, IrBlock::default()).unwrap();
        assert_eq!(blocks.len(), 1);
        assert_eq!(
            budget.copy_str("x").unwrap_err().code,
            ErrorCode::OutputTooLarge
        );
    }

    #[test]
    fn raw_xml_text_charges_each_concatenation_before_growth() {
        let root = parse(b"<a><b>abc</b></a>").unwrap();
        let mut budget = LoweringBudget::with_limit(5);
        assert_eq!(budget.raw_xml_text(&root).unwrap(), "abc");
        assert_eq!(
            budget.raw_xml_text(&root).unwrap_err().code,
            ErrorCode::OutputTooLarge
        );
    }

    #[test]
    fn checkpoint_rolls_back_discarded_section_charge() {
        let mut budget = LoweringBudget::with_limit(3);
        let checkpoint = budget.checkpoint();
        assert_eq!(budget.copy_str("abc").unwrap(), "abc");
        budget.rollback(checkpoint);
        assert_eq!(budget.copy_str("xyz").unwrap(), "xyz");
    }
}
