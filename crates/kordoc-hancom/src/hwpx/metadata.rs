//! Bounded metadata-only extraction from package metadata records.

#![allow(
    dead_code,
    reason = "H1b metadata path is wired by the H3 integration join"
)]

use kordoc_ir::DocumentMetadata;

use super::{package::Package, xml};

pub(crate) fn extract_metadata(
    package: &mut Package<'_>,
) -> Result<DocumentMetadata, kordoc_ir::KordocError> {
    let mut metadata = DocumentMetadata::default();
    for path in ["Contents/content.hpf", "content.hpf"] {
        if let Some(bytes) = package.read(path)? {
            read_opf(&xml::parse_critical(&bytes)?, &mut metadata);
            break;
        }
    }
    if metadata.title.is_none() && metadata.author.is_none() {
        for path in ["meta.xml", "META-INF/meta.xml", "docProps/core.xml"] {
            if let Some(bytes) = package.read(path)? {
                match xml::parse(&bytes) {
                    Ok(root) => read_dublin_core(&root, &mut metadata),
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

fn read_opf(root: &xml::XmlNode, metadata: &mut DocumentMetadata) {
    let Some(md) = root.descendants("metadata").into_iter().next() else {
        return;
    };
    let mut values = std::collections::HashMap::<String, String>::new();
    let mut title = None;
    for node in &md.children {
        let value = node.text_content().trim().to_owned();
        if value.is_empty() {
            continue;
        }
        match node.name.as_str() {
            "title" => {
                title.get_or_insert(value);
            }
            "meta" => {
                let name = node.attr("name").unwrap_or_default();
                if !name.is_empty() {
                    values.entry(name.to_owned()).or_insert(value);
                }
            }
            _ => continue,
        };
    }
    metadata.title = metadata.title.take().or(title);
    metadata.author = metadata
        .author
        .take()
        .or_else(|| values.get("creator").cloned());
    metadata.description = metadata.description.take().or_else(|| {
        values
            .get("description")
            .or_else(|| values.get("subject"))
            .cloned()
    });
    if metadata.keywords.is_none()
        && let Some(value) = values.get("keyword")
    {
        let keywords: Vec<_> = value
            .split([',', ';'])
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .map(str::to_owned)
            .collect();
        if !keywords.is_empty() {
            metadata.keywords = Some(keywords);
        }
    }
    if metadata.created_at.is_none() {
        metadata.created_at = values.get("CreatedDate").cloned().and_then(normalize_date);
    }
    if metadata.modified_at.is_none() {
        metadata.modified_at = values.get("ModifiedDate").cloned().and_then(normalize_date);
    }
}

fn read_dublin_core(root: &xml::XmlNode, metadata: &mut DocumentMetadata) {
    fn first(root: &xml::XmlNode, names: &[&str]) -> Option<String> {
        for name in names {
            if let Some(node) = root.descendants(name).into_iter().next() {
                let value = node.text_content().trim().to_owned();
                if !value.is_empty() {
                    return Some(value);
                }
            }
        }
        None
    }
    metadata.title = metadata.title.take().or_else(|| first(root, &["title"]));
    metadata.author = metadata
        .author
        .take()
        .or_else(|| first(root, &["creator", "lastModifiedBy"]));
    metadata.description = metadata
        .description
        .take()
        .or_else(|| first(root, &["description", "subject"]));
    metadata.created_at = metadata
        .created_at
        .take()
        .or_else(|| first(root, &["created", "creation-date"]));
    metadata.modified_at = metadata
        .modified_at
        .take()
        .or_else(|| first(root, &["modified", "date"]));
    if metadata.keywords.is_none()
        && let Some(value) = first(root, &["keyword", "keywords"])
    {
        let values: Vec<_> = value
            .split([',', ';'])
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .map(str::to_owned)
            .collect();
        if !values.is_empty() {
            metadata.keywords = Some(values);
        }
    }
}

fn normalize_date(value: String) -> Option<String> {
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
    Some(if strict_local {
        value.replacen(' ', "T", 1)
    } else {
        value
    })
}

#[cfg(test)]
#[path = "metadata/tests.rs"]
mod tests;
