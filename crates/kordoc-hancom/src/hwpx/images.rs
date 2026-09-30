//! Package-backed HWPX image resolution.

use crate::hwpx::package::Package;
use crate::hwpx::xml::{XmlContent, XmlNode};
use kordoc_ir::{
    ErrorCode, ExtractedImage, ImageData, IrBlock, IrBlockType, KordocError, ParseWarning,
    WarningCode,
};
use std::collections::{HashMap, HashSet};

#[derive(Default)]
pub(crate) struct ImageCache {
    by_ref: HashMap<String, Option<ExtractedImage>>,
    warned: HashSet<String>,
}

pub(crate) fn image_reference(node: &XmlNode) -> Option<String> {
    fn find(node: &XmlNode) -> Option<String> {
        if matches!(node.name.as_str(), "imgRect" | "img" | "imgClip")
            && let Some(reference) = node.attr("binaryItemIDRef").or_else(|| node.attr("href"))
        {
            return Some(reference.to_owned());
        }
        if let Some(reference) = node.attr("binaryItemIDRef") {
            return Some(reference.to_owned());
        }
        for part in &node.content {
            if let XmlContent::Child(index) = part
                && let Some(found) = find(&node.children[*index])
            {
                return Some(found);
            }
        }
        None
    }
    find(node)
}

pub(crate) fn resolve_image(
    reference: &str,
    page: Option<u32>,
    package: &mut Package<'_>,
    cache: &mut ImageCache,
    images: &mut Vec<ExtractedImage>,
    warnings: &mut Vec<ParseWarning>,
) -> Result<IrBlock, KordocError> {
    if !cache.by_ref.contains_key(reference) {
        let mut found = None;
        for path in image_candidates(reference) {
            match package.read(&path) {
                Ok(Some(data)) => {
                    let Some(mime) = mime_from_path(&path).or_else(|| sniff_mime(&data)) else {
                        continue;
                    };
                    let filename =
                        format!("image_{:03}.{}", images.len() + 1, ext_from_mime(&mime));
                    let image = ExtractedImage {
                        filename: filename.clone(),
                        data: data.clone(),
                        mime_type: mime.clone(),
                        source: Some(path),
                    };
                    images.push(image.clone());
                    found = Some(image);
                    break;
                }
                Ok(None) => continue,
                Err(error) if error.code == ErrorCode::Corrupted => continue,
                Err(error) => return Err(error),
            }
        }
        cache.by_ref.insert(reference.to_owned(), found);
    }
    if let Some(image) = cache.by_ref.get(reference).and_then(Clone::clone) {
        Ok(IrBlock {
            kind: IrBlockType::Image,
            text: Some(image.filename.clone()),
            image_data: Some(ImageData {
                data: image.data,
                mime_type: image.mime_type,
                filename: Some(reference.to_owned()),
            }),
            page_number: page,
            ..IrBlock::default()
        })
    } else {
        if cache.warned.insert(reference.to_owned()) {
            warnings.push(ParseWarning {
                page,
                message: format!("Optional HWPX image is missing: {reference}"),
                code: WarningCode::SkippedImage,
            });
        }
        Ok(IrBlock::paragraph(format!("[Image: {reference}]")))
    }
}

fn image_candidates(reference: &str) -> Vec<String> {
    let clean = reference.trim_start_matches('/');
    let mut paths = vec![
        clean.to_owned(),
        format!("BinData/{clean}"),
        format!("Contents/BinData/{clean}"),
    ];
    if !clean.rsplit('/').next().unwrap_or(clean).contains('.') {
        for base in [
            format!("BinData/{clean}"),
            format!("Contents/BinData/{clean}"),
        ] {
            paths.extend(
                [
                    "png", "jpg", "jpeg", "bmp", "gif", "tif", "tiff", "svg", "wmf", "emf",
                ]
                .into_iter()
                .map(|ext| format!("{base}.{ext}")),
            );
        }
    }
    paths
}
fn mime_from_path(path: &str) -> Option<String> {
    match path
        .rsplit('.')
        .next()
        .unwrap_or("")
        .to_ascii_lowercase()
        .as_str()
    {
        "jpg" | "jpeg" => "image/jpeg",
        "png" => "image/png",
        "gif" => "image/gif",
        "bmp" => "image/bmp",
        "tif" | "tiff" => "image/tiff",
        "svg" => "image/svg+xml",
        "wmf" => "image/wmf",
        "emf" => "image/emf",
        _ => return None,
    }
    .to_owned()
    .into()
}

fn sniff_mime(data: &[u8]) -> Option<String> {
    let mime = if data.starts_with(b"\x89PNG\r\n\x1a\n") {
        "image/png"
    } else if data.starts_with(b"\xff\xd8\xff") {
        "image/jpeg"
    } else if data.starts_with(b"GIF87a") || data.starts_with(b"GIF89a") {
        "image/gif"
    } else if data.starts_with(b"BM") {
        "image/bmp"
    } else if data.starts_with(b"II*\0") || data.starts_with(b"MM\0*") {
        "image/tiff"
    } else if data.starts_with(b"\xd7\xcd\xc6\x9a") {
        "image/wmf"
    } else if data.len() >= 44 && data.get(40..44) == Some(&b" EMF"[..]) {
        "image/emf"
    } else if data.get(..256).is_some_and(|header| {
        String::from_utf8_lossy(header)
            .to_ascii_lowercase()
            .contains("<svg")
    }) {
        "image/svg+xml"
    } else {
        return None;
    };
    Some(mime.to_owned())
}
fn ext_from_mime(mime: &str) -> &'static str {
    match mime {
        "image/jpeg" => "jpg",
        "image/png" => "png",
        "image/gif" => "gif",
        "image/bmp" => "bmp",
        "image/tiff" => "tif",
        "image/svg+xml" => "svg",
        "image/wmf" => "wmf",
        "image/emf" => "emf",
        _ => "bin",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::{Cursor, Write};
    use zip::{ZipWriter, write::SimpleFileOptions};

    fn package() -> Package<'static> {
        let mut writer = ZipWriter::new(Cursor::new(Vec::new()));
        writer
            .start_file("Contents/BinData/picture.png", SimpleFileOptions::default())
            .unwrap();
        writer.write_all(b"pngdata").unwrap();
        let bytes = Box::leak(writer.finish().unwrap().into_inner().into_boxed_slice());
        Package::open(bytes).unwrap()
    }

    #[test]
    fn resolves_referenced_images_with_bytes_and_mime() {
        let mut package = package();
        let mut cache = ImageCache::default();
        let mut images = Vec::new();
        let mut warnings = Vec::new();
        let block = resolve_image(
            "picture",
            Some(2),
            &mut package,
            &mut cache,
            &mut images,
            &mut warnings,
        )
        .unwrap();
        assert_eq!(images.len(), 1);
        assert_eq!(images[0].mime_type, "image/png");
        assert_eq!(images[0].data, b"pngdata");
        assert_eq!(block.image_data.unwrap().mime_type, "image/png");
        assert_eq!(block.page_number, Some(2));
        assert!(warnings.is_empty());
    }

    #[test]
    fn skips_missing_optional_image_with_warning() {
        let mut package = package();
        let mut cache = ImageCache::default();
        let mut images = Vec::new();
        let mut warnings = Vec::new();
        let _ = resolve_image(
            "absent",
            None,
            &mut package,
            &mut cache,
            &mut images,
            &mut warnings,
        )
        .unwrap();
        let _ = resolve_image(
            "absent",
            None,
            &mut package,
            &mut cache,
            &mut images,
            &mut warnings,
        )
        .unwrap();
        assert_eq!(warnings.len(), 1);
        assert_eq!(warnings[0].code, WarningCode::SkippedImage);
    }
}
