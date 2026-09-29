//! Bounded HWPX paragraph and character style references.

// H2a's private parser seam is intentionally not wired into crate entry points until H3.
#![allow(dead_code)]

use std::collections::BTreeMap;

use kordoc_ir::{ErrorCode, InlineStyle, KordocError};

use crate::hwpx::xml::{XmlNode, parse_critical};

#[derive(Debug, Clone, Default)]
pub(crate) struct StyleCatalog {
    paragraph_levels: BTreeMap<String, u32>,
    character_styles: BTreeMap<String, InlineStyle>,
}

impl StyleCatalog {
    pub(crate) fn parse(bytes: &[u8]) -> Result<Self, KordocError> {
        let root = parse_critical(bytes)?;
        let mut catalog = Self::default();
        for node in root.descendants("paraPr") {
            if let (Some(id), Some(level)) = (node.attr("id"), node.attr("outlineLvl")) {
                let level = level.parse::<u32>().map_err(|_| {
                    KordocError::new(ErrorCode::Corrupted, "invalid paragraph outline level")
                })?;
                if level > 0 {
                    catalog.paragraph_levels.insert(id.to_owned(), level);
                }
            }
        }
        for node in root.descendants("charPr") {
            let Some(id) = node.attr("id") else { continue };
            let mut style = InlineStyle {
                bold: parse_flag(node.attr("bold")),
                italic: parse_flag(node.attr("italic")),
                underline: parse_flag(node.attr("underline")),
                strike: parse_flag(node.attr("strikeout").or_else(|| node.attr("strike"))),
                ..InlineStyle::default()
            };
            for child in &node.children {
                match child.name.as_str() {
                    "bold" => style.bold = Some(true),
                    "italic" => style.italic = Some(true),
                    "underline" => style.underline = Some(true),
                    "strikeout" | "strike" => style.strike = Some(true),
                    _ => {}
                }
            }
            catalog.character_styles.insert(id.to_owned(), style);
        }
        Ok(catalog)
    }

    pub(crate) fn paragraph_level(&self, node: &XmlNode) -> Option<u32> {
        node.attr("paraPrIDRef")
            .and_then(|id| self.paragraph_levels.get(id))
            .copied()
    }

    pub(crate) fn character_style(&self, run: &XmlNode) -> Option<&InlineStyle> {
        run.attr("charPrIDRef")
            .and_then(|id| self.character_styles.get(id))
    }
}

fn parse_flag(value: Option<&str>) -> Option<bool> {
    value.map(|value| matches!(value.to_ascii_lowercase().as_str(), "1" | "true" | "yes"))
}

#[cfg(test)]
mod tests;
