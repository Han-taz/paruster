//! Package-backed HWPX image resolution.

use crate::hwpx::package::Package;
use crate::hwpx::xml::{XmlContent, XmlNode};
use kordoc_ir::{
    ErrorCode, ExtractedImage, ImageData, IrBlock, IrBlockType, KordocError, ParseWarning,
    WarningCode,
};
use std::collections::HashMap;

const MAX_IMAGE_OUTPUT_BYTES: usize = 256 * 1024 * 1024;
const MAX_IMAGE_REF_BYTES: usize = u16::MAX as usize;

#[derive(Clone)]
pub(crate) struct ImageCache {
    by_ref: HashMap<String, (u64, Option<ExtractedImage>)>,
    warned: HashMap<String, u64>,
    image_output_bytes: usize,
    image_output_limit: usize,
    generation: u64,
}

impl Default for ImageCache {
    fn default() -> Self {
        Self {
            by_ref: HashMap::new(),
            warned: HashMap::new(),
            image_output_bytes: 0,
            image_output_limit: MAX_IMAGE_OUTPUT_BYTES,
            generation: 0,
        }
    }
}

impl ImageCache {
    #[cfg(test)]
    fn with_limit(image_output_limit: usize) -> Self {
        Self {
            image_output_limit,
            ..Self::default()
        }
    }

    fn charge(&mut self, amount: usize) -> Result<(), KordocError> {
        let next = self.image_output_bytes.checked_add(amount).ok_or_else(|| {
            KordocError::new(
                ErrorCode::OutputTooLarge,
                "HWPX image output exceeds its allocation limit",
            )
        })?;
        if next > self.image_output_limit {
            return Err(KordocError::new(
                ErrorCode::OutputTooLarge,
                "HWPX image output exceeds its allocation limit",
            ));
        }
        self.image_output_bytes = next;
        Ok(())
    }

    pub(crate) fn checkpoint(&self) -> (u64, usize) {
        (self.generation, self.image_output_bytes)
    }

    pub(crate) fn rollback(&mut self, checkpoint: (u64, usize)) {
        self.by_ref
            .retain(|_, (generation, _)| *generation <= checkpoint.0);
        self.warned
            .retain(|_, generation| *generation <= checkpoint.0);
        self.generation = checkpoint.0;
        self.image_output_bytes = checkpoint.1;
    }

    fn record_generation(&mut self) -> u64 {
        self.generation = self.generation.saturating_add(1);
        self.generation
    }
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
        if reference.len() <= MAX_IMAGE_REF_BYTES && safe_image_reference(reference) {
            for path in image_candidates(reference) {
                match package.read(&path) {
                    Ok(Some(data)) => {
                        let Some(mime) = mime_from_path(&path).or_else(|| sniff_mime(&data)) else {
                            continue;
                        };
                        let filename =
                            format!("image_{:03}.{}", images.len() + 1, ext_from_mime(&mime));
                        cache.charge(data.len())?;
                        let image = ExtractedImage {
                            filename,
                            data,
                            mime_type: mime,
                            source: Some(path),
                        };
                        cache.charge(image.data.len())?;
                        let metadata_bytes = image.filename.len()
                            + image.mime_type.len()
                            + image.source.as_ref().map_or(0, String::len);
                        cache.charge(metadata_bytes)?;
                        images.push(ExtractedImage {
                            filename: image.filename.clone(),
                            data: image.data.clone(),
                            mime_type: image.mime_type.clone(),
                            source: image.source.clone(),
                        });
                        found = Some(image);
                        break;
                    }
                    Ok(None) => continue,
                    Err(error) if error.code == ErrorCode::Corrupted => continue,
                    Err(error) => return Err(error),
                }
            }
        }
        cache.charge(reference.len())?;
        let generation = cache.record_generation();
        cache
            .by_ref
            .insert(reference.to_owned(), (generation, found));
    }
    if cache
        .by_ref
        .get(reference)
        .and_then(|(_, image)| image.as_ref())
        .is_some()
    {
        let image_bytes = cache
            .by_ref
            .get(reference)
            .and_then(|(_, image)| image.as_ref())
            .map_or(0, |image| image.data.len());
        cache.charge(image_bytes)?;
        let metadata_bytes = cache
            .by_ref
            .get(reference)
            .and_then(|(_, image)| image.as_ref())
            .map_or(0, |image| image.filename.len() + reference.len());
        cache.charge(metadata_bytes)?;
        let Some(image) = cache
            .by_ref
            .get(reference)
            .and_then(|(_, image)| image.as_ref())
        else {
            return skipped_image(reference, page, cache, warnings);
        };
        Ok(IrBlock {
            kind: IrBlockType::Image,
            text: Some(image.filename.clone()),
            image_data: Some(ImageData {
                data: image.data.clone(),
                mime_type: image.mime_type.clone(),
                filename: Some(reference.to_owned()),
            }),
            page_number: page,
            ..IrBlock::default()
        })
    } else {
        skipped_image(reference, page, cache, warnings)
    }
}

fn skipped_image(
    reference: &str,
    page: Option<u32>,
    cache: &mut ImageCache,
    warnings: &mut Vec<ParseWarning>,
) -> Result<IrBlock, KordocError> {
    if !cache.warned.contains_key(reference) {
        cache.charge(reference.len().saturating_mul(3).saturating_add(64))?;
        let generation = cache.record_generation();
        cache.warned.insert(reference.to_owned(), generation);
    } else {
        cache.charge(reference.len().saturating_add(16))?;
        warnings.push(ParseWarning {
            page,
            message: format!("Optional HWPX image is missing or unsupported: {reference}"),
            code: WarningCode::SkippedImage,
        });
    }
    Ok(IrBlock::paragraph(format!("[Image: {reference}]")))
}

fn safe_image_reference(reference: &str) -> bool {
    !reference.is_empty()
        && !reference.starts_with('/')
        && !reference.contains('\\')
        && reference
            .split('/')
            .all(|part| !part.is_empty() && part != "." && part != "..")
}

fn image_candidates(reference: &str) -> Vec<String> {
    let clean = reference;
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
            .start_file("picture.png", SimpleFileOptions::default())
            .unwrap();
        writer.write_all(b"root-image").unwrap();
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

    #[test]
    fn image_output_meter_is_inclusive_and_charges_repeated_block_copies() {
        let mut first_package = package();
        let mut cache = ImageCache::with_limit(1024);
        let mut images = Vec::new();
        let mut warnings = Vec::new();
        resolve_image(
            "picture",
            None,
            &mut first_package,
            &mut cache,
            &mut images,
            &mut warnings,
        )
        .unwrap();
        let inclusive_limit = cache.image_output_bytes;
        cache.image_output_limit = inclusive_limit;
        let mut at_limit_package = package();
        let mut at_limit_cache = ImageCache::with_limit(inclusive_limit);
        resolve_image(
            "picture",
            None,
            &mut at_limit_package,
            &mut at_limit_cache,
            &mut Vec::new(),
            &mut Vec::new(),
        )
        .unwrap();
        assert_eq!(at_limit_cache.image_output_bytes, inclusive_limit);
        let error = resolve_image(
            "picture",
            None,
            &mut first_package,
            &mut cache,
            &mut images,
            &mut warnings,
        )
        .unwrap_err();
        assert_eq!(error.code, ErrorCode::OutputTooLarge);
        assert_eq!(images.len(), 1);

        let mut package = package();
        let mut cache = ImageCache::with_limit(inclusive_limit - 1);
        let error = resolve_image(
            "picture",
            None,
            &mut package,
            &mut cache,
            &mut Vec::new(),
            &mut Vec::new(),
        )
        .unwrap_err();
        assert_eq!(error.code, ErrorCode::OutputTooLarge);
    }

    #[test]
    fn image_output_meter_covers_missing_reference_bookkeeping() {
        let mut package = package();
        let mut cache = ImageCache::with_limit(20);
        let error = resolve_image(
            "missing",
            None,
            &mut package,
            &mut cache,
            &mut Vec::new(),
            &mut Vec::new(),
        )
        .unwrap_err();
        assert_eq!(error.code, ErrorCode::OutputTooLarge);
    }

    #[test]
    fn skips_unsafe_and_overlong_references_once_without_aliasing() {
        let mut package = package();
        let mut cache = ImageCache::default();
        let mut images = Vec::new();
        let mut warnings = Vec::new();
        for reference in ["/picture.png", "../picture.png", "picture\\alias.png"] {
            resolve_image(
                reference,
                None,
                &mut package,
                &mut cache,
                &mut images,
                &mut warnings,
            )
            .unwrap();
            resolve_image(
                reference,
                None,
                &mut package,
                &mut cache,
                &mut images,
                &mut warnings,
            )
            .unwrap();
        }
        let too_long = "x".repeat(65_536);
        resolve_image(
            &too_long,
            None,
            &mut package,
            &mut cache,
            &mut images,
            &mut warnings,
        )
        .unwrap();
        resolve_image(
            &too_long,
            None,
            &mut package,
            &mut cache,
            &mut images,
            &mut warnings,
        )
        .unwrap();
        assert!(images.is_empty());
        assert_eq!(warnings.len(), 4);
        assert!(
            warnings
                .iter()
                .all(|warning| warning.code == WarningCode::SkippedImage)
        );
    }
}
