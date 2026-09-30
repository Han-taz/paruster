//! Bounded metadata-only extraction from package metadata records.

#![allow(
    dead_code,
    reason = "H1b metadata path is wired by the H3 integration join"
)]

use kordoc_ir::{DocumentMetadata, KordocError};

use super::{budget::LoweringBudget, package::Package, xml};

pub(crate) fn extract_metadata(
    package: &mut Package<'_>,
) -> Result<DocumentMetadata, kordoc_ir::KordocError> {
    let mut metadata = DocumentMetadata::default();
    let mut budget = LoweringBudget::default();
    for path in ["Contents/content.hpf", "content.hpf"] {
        if let Some(bytes) = package.read(path)? {
            read_opf(&xml::parse_critical(&bytes)?, &mut metadata, &mut budget)?;
            break;
        }
    }
    if metadata.title.is_none() && metadata.author.is_none() {
        for path in ["meta.xml", "META-INF/meta.xml", "docProps/core.xml"] {
            if let Some(bytes) = package.read(path)? {
                match xml::parse(&bytes) {
                    Ok(root) => read_dublin_core(&root, &mut metadata, &mut budget)?,
                    Err(error) if error.is_resource_limit() => {
                        return Err(kordoc_ir::KordocError::new(
                            kordoc_ir::ErrorCode::DecompressionBomb,
                            error.message,
                        ));
                    }
                    Err(_) => continue,
                }
                if metadata.title.is_some() || metadata.author.is_some() {
                    break;
                }
            }
        }
    }
    metadata.page_count = Some(u32::try_from(package.section_paths()?.len()).unwrap_or(u32::MAX));
    Ok(metadata)
}

fn first_descendant<'a>(root: &'a xml::XmlNode, name: &str) -> Option<&'a xml::XmlNode> {
    if root.name == name {
        return Some(root);
    }
    root.children
        .iter()
        .find_map(|child| first_descendant(child, name))
}

fn metadata_value(
    node: &xml::XmlNode,
    budget: &mut LoweringBudget,
) -> Result<Option<String>, KordocError> {
    let text = budget.raw_xml_text(node)?;
    let trimmed = text.trim();
    if trimmed.is_empty() {
        Ok(None)
    } else {
        budget.copy_str(trimmed).map(Some)
    }
}

fn keywords(value: &str, budget: &mut LoweringBudget) -> Result<Option<Vec<String>>, KordocError> {
    let mut words = Vec::new();
    for word in value
        .split([',', ';'])
        .map(str::trim)
        .filter(|s| !s.is_empty())
    {
        let word = budget.copy_str(word)?;
        budget.push(&mut words, word)?;
    }
    Ok((!words.is_empty()).then_some(words))
}

fn read_opf(
    root: &xml::XmlNode,
    metadata: &mut DocumentMetadata,
    budget: &mut LoweringBudget,
) -> Result<(), KordocError> {
    let Some(md) = first_descendant(root, "metadata") else {
        return Ok(());
    };
    let mut title = None;
    let mut creator = None;
    let mut description = None;
    let mut subject = None;
    let mut keyword = None;
    let mut created = None;
    let mut modified = None;
    for node in &md.children {
        match node.name.as_str() {
            "title" if title.is_none() => title = metadata_value(node, budget)?,
            "meta" => {
                let slot = match node.attr("name").unwrap_or_default() {
                    "creator" => &mut creator,
                    "description" => &mut description,
                    "subject" => &mut subject,
                    "keyword" => &mut keyword,
                    "CreatedDate" => &mut created,
                    "ModifiedDate" => &mut modified,
                    _ => continue,
                };
                if slot.is_none() {
                    *slot = metadata_value(node, budget)?;
                }
            }
            _ => continue,
        }
    }
    metadata.title = metadata.title.take().or(title);
    metadata.author = metadata.author.take().or(creator);
    metadata.description = metadata.description.take().or(description.or(subject));
    if metadata.keywords.is_none()
        && let Some(value) = keyword
    {
        metadata.keywords = keywords(&value, budget)?;
    }
    if metadata.created_at.is_none() {
        metadata.created_at = created.and_then(normalize_date);
    }
    if metadata.modified_at.is_none() {
        metadata.modified_at = modified.and_then(normalize_date);
    }
    Ok(())
}

fn read_dublin_core(
    root: &xml::XmlNode,
    metadata: &mut DocumentMetadata,
    budget: &mut LoweringBudget,
) -> Result<(), KordocError> {
    fn first(
        root: &xml::XmlNode,
        names: &[&str],
        budget: &mut LoweringBudget,
    ) -> Result<Option<String>, KordocError> {
        for name in names {
            if let Some(node) = first_descendant(root, name)
                && let Some(value) = metadata_value(node, budget)?
            {
                return Ok(Some(value));
            }
        }
        Ok(None)
    }
    if metadata.title.is_none() {
        metadata.title = first(root, &["title"], budget)?;
    }
    if metadata.author.is_none() {
        metadata.author = first(root, &["creator", "lastModifiedBy"], budget)?;
    }
    if metadata.description.is_none() {
        metadata.description = first(root, &["description", "subject"], budget)?;
    }
    if metadata.created_at.is_none() {
        metadata.created_at = first(root, &["created", "creation-date"], budget)?;
    }
    if metadata.modified_at.is_none() {
        metadata.modified_at = first(root, &["modified", "date"], budget)?;
    }
    if metadata.keywords.is_none()
        && let Some(value) = first(root, &["keyword", "keywords"], budget)?
    {
        metadata.keywords = keywords(&value, budget)?;
    }
    Ok(())
}

fn normalize_date(mut value: String) -> Option<String> {
    if value.starts_with("1601-01-01") {
        return None;
    }
    let date = value.as_bytes();
    let strict_local = date.len() == 19
        && date[10] == b' '
        && date.iter().enumerate().all(|(i, byte)| match i {
            4 | 7 => *byte == b'-',
            10 => *byte == b' ',
            13 | 16 => *byte == b':',
            _ => byte.is_ascii_digit(),
        });
    if strict_local {
        value.replace_range(10..11, "T");
    }
    Some(value)
}

#[cfg(test)]
#[path = "metadata/tests.rs"]
mod tests;
