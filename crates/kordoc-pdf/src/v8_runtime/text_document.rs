use kordoc_ir::{ErrorCode, KordocError};
use serde::de::{DeserializeSeed, MapAccess, SeqAccess, Visitor};
use serde::{Deserialize, Deserializer, Serialize};
use std::fmt;

const MAX_PAGES: usize = 200;
const MAX_ITEMS: usize = 100_000;
const MAX_ITEM_TEXT: usize = 64 * 1024;
const MAX_TEXT: usize = 2 * 1024 * 1024;
const MAX_FONT: usize = 128;
const MAX_FONTS: usize = 512 * 1024;
const MAX_METADATA_VALUE: usize = 4 * 1024;
const MAX_METADATA: usize = 16 * 1024;
const MAX_RESPONSE: usize = 4 * 1024 * 1024;

#[derive(Debug, Clone, Copy)]
pub(crate) struct TextDocumentLimits {
    pub(crate) max_pages: usize,
    pub(crate) max_items: usize,
    pub(crate) max_item_text_bytes: usize,
    pub(crate) max_text_bytes: usize,
    pub(crate) max_font_name_bytes: usize,
    pub(crate) max_font_names_bytes: usize,
    pub(crate) max_metadata_value_bytes: usize,
    pub(crate) max_metadata_bytes: usize,
    pub(crate) max_response_bytes: usize,
}

impl Default for TextDocumentLimits {
    fn default() -> Self {
        Self {
            max_pages: MAX_PAGES,
            max_items: MAX_ITEMS,
            max_item_text_bytes: MAX_ITEM_TEXT,
            max_text_bytes: MAX_TEXT,
            max_font_name_bytes: MAX_FONT,
            max_font_names_bytes: MAX_FONTS,
            max_metadata_value_bytes: MAX_METADATA_VALUE,
            max_metadata_bytes: MAX_METADATA,
            max_response_bytes: MAX_RESPONSE,
        }
    }
}

impl<'de> Deserialize<'de> for PdfJsTextDocument {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        DocumentSeed(TextDocumentLimits::default()).deserialize(deserializer)
    }
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub(crate) struct PdfJsTextDocument {
    pub(crate) page_count: u32,
    pub(crate) metadata: PdfJsMetadata,
    pub(crate) pages: Vec<PdfJsPage>,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub(crate) struct PdfJsMetadata {
    pub(crate) title: Option<String>,
    pub(crate) author: Option<String>,
    pub(crate) creator: Option<String>,
    pub(crate) subject: Option<String>,
    pub(crate) keywords: Option<String>,
    pub(crate) creation_date: Option<String>,
    pub(crate) modified_date: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub(crate) struct PdfJsPage {
    pub(crate) page_number: u32,
    pub(crate) view_box: [f64; 4],
    pub(crate) rotation: i32,
    pub(crate) items: Vec<PdfJsTextItem>,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub(crate) struct PdfJsTextItem {
    pub(crate) text: String,
    pub(crate) width: f64,
    pub(crate) height: f64,
    pub(crate) transform: [f64; 6],
    pub(crate) font_name: String,
}

#[derive(Clone, Copy)]
struct DecodeBudget {
    limits: TextDocumentLimits,
    items: usize,
    text_bytes: usize,
    font_bytes: usize,
    metadata_bytes: usize,
}

struct RejectExtraValue(&'static str);
impl<'de> DeserializeSeed<'de> for RejectExtraValue {
    type Value = ();
    fn deserialize<D: Deserializer<'de>>(self, deserializer: D) -> Result<(), D::Error> {
        struct RejectVisitor(&'static str);
        impl<'de> Visitor<'de> for RejectVisitor {
            type Value = ();
            fn expecting(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                f.write_str("no sequence element beyond the configured cap")
            }
            fn visit_bool<E: serde::de::Error>(self, _: bool) -> Result<(), E> {
                Err(E::custom(format!("PDFJS_LIMIT:{}", self.0)))
            }
            fn visit_i64<E: serde::de::Error>(self, _: i64) -> Result<(), E> {
                Err(E::custom(format!("PDFJS_LIMIT:{}", self.0)))
            }
            fn visit_u64<E: serde::de::Error>(self, _: u64) -> Result<(), E> {
                Err(E::custom(format!("PDFJS_LIMIT:{}", self.0)))
            }
            fn visit_f64<E: serde::de::Error>(self, _: f64) -> Result<(), E> {
                Err(E::custom(format!("PDFJS_LIMIT:{}", self.0)))
            }
            fn visit_str<E: serde::de::Error>(self, _: &str) -> Result<(), E> {
                Err(E::custom(format!("PDFJS_LIMIT:{}", self.0)))
            }
            fn visit_string<E: serde::de::Error>(self, _: String) -> Result<(), E> {
                Err(E::custom(format!("PDFJS_LIMIT:{}", self.0)))
            }
            fn visit_unit<E: serde::de::Error>(self) -> Result<(), E> {
                Err(E::custom(format!("PDFJS_LIMIT:{}", self.0)))
            }
            fn visit_seq<A: SeqAccess<'de>>(self, _: A) -> Result<(), A::Error> {
                Err(serde::de::Error::custom(format!("PDFJS_LIMIT:{}", self.0)))
            }
            fn visit_map<A: MapAccess<'de>>(self, _: A) -> Result<(), A::Error> {
                Err(serde::de::Error::custom(format!("PDFJS_LIMIT:{}", self.0)))
            }
        }
        deserializer.deserialize_any(RejectVisitor(self.0))
    }
}

struct DocumentSeed(TextDocumentLimits);
impl<'de> DeserializeSeed<'de> for DocumentSeed {
    type Value = PdfJsTextDocument;
    fn deserialize<D: Deserializer<'de>>(self, deserializer: D) -> Result<Self::Value, D::Error> {
        struct DocumentVisitor(DecodeBudget);
        impl<'de> Visitor<'de> for DocumentVisitor {
            type Value = PdfJsTextDocument;
            fn expecting(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                f.write_str("a bounded PDF.js text document")
            }
            fn visit_map<A: MapAccess<'de>>(mut self, mut map: A) -> Result<Self::Value, A::Error> {
                let mut page_count = None;
                let mut metadata = None;
                let mut pages = None;
                while let Some(key) = map.next_key::<String>()? {
                    match key.as_str() {
                        "page_count" if page_count.is_none() => {
                            page_count = Some(map.next_value::<u32>()?)
                        }
                        "metadata" if metadata.is_none() => {
                            metadata = Some(map.next_value_seed(MetadataSeed(&mut self.0))?)
                        }
                        "pages" if pages.is_none() => {
                            pages = Some(map.next_value_seed(PagesSeed(&mut self.0))?)
                        }
                        "page_count" | "metadata" | "pages" => {
                            return Err(serde::de::Error::duplicate_field(
                                "PDF.js text document field",
                            ));
                        }
                        _ => {
                            return Err(serde::de::Error::unknown_field(
                                &key,
                                &["page_count", "metadata", "pages"],
                            ));
                        }
                    }
                }
                Ok(PdfJsTextDocument {
                    page_count: page_count
                        .ok_or_else(|| serde::de::Error::missing_field("page_count"))?,
                    metadata: metadata
                        .ok_or_else(|| serde::de::Error::missing_field("metadata"))?,
                    pages: pages.ok_or_else(|| serde::de::Error::missing_field("pages"))?,
                })
            }
        }
        deserializer.deserialize_map(DocumentVisitor(DecodeBudget {
            limits: self.0,
            items: 0,
            text_bytes: 0,
            font_bytes: 0,
            metadata_bytes: 0,
        }))
    }
}

struct PagesSeed<'a>(&'a mut DecodeBudget);
impl<'de> DeserializeSeed<'de> for PagesSeed<'_> {
    type Value = Vec<PdfJsPage>;
    fn deserialize<D: Deserializer<'de>>(self, deserializer: D) -> Result<Self::Value, D::Error> {
        struct PagesVisitor<'a>(&'a mut DecodeBudget);
        impl<'de> Visitor<'de> for PagesVisitor<'_> {
            type Value = Vec<PdfJsPage>;
            fn expecting(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                f.write_str("a bounded page sequence")
            }
            fn visit_seq<A: SeqAccess<'de>>(self, mut seq: A) -> Result<Self::Value, A::Error> {
                let mut pages = Vec::new();
                loop {
                    if pages.len() >= self.0.limits.max_pages {
                        let _ = seq.next_element_seed(RejectExtraValue("page count exceeded"))?;
                        break;
                    }
                    pages.try_reserve(1).map_err(|_| {
                        serde::de::Error::custom("PDFJS_LIMIT:page allocation failed")
                    })?;
                    let Some(page) = seq.next_element_seed(PageSeed(self.0))? else {
                        break;
                    };
                    pages.push(page);
                }
                Ok(pages)
            }
        }
        deserializer.deserialize_seq(PagesVisitor(self.0))
    }
}

struct PageSeed<'a>(&'a mut DecodeBudget);
impl<'de> DeserializeSeed<'de> for PageSeed<'_> {
    type Value = PdfJsPage;
    fn deserialize<D: Deserializer<'de>>(self, deserializer: D) -> Result<Self::Value, D::Error> {
        struct PageVisitor<'a>(&'a mut DecodeBudget);
        impl<'de> Visitor<'de> for PageVisitor<'_> {
            type Value = PdfJsPage;
            fn expecting(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                f.write_str("a PDF.js page")
            }
            fn visit_map<A: MapAccess<'de>>(self, mut map: A) -> Result<Self::Value, A::Error> {
                let mut number = None;
                let mut view = None;
                let mut rotation = None;
                let mut items = None;
                while let Some(key) = map.next_key::<String>()? {
                    match key.as_str() {
                        "page_number" if number.is_none() => {
                            number = Some(map.next_value::<u32>()?)
                        }
                        "view_box" if view.is_none() => view = Some(map.next_value::<[f64; 4]>()?),
                        "rotation" if rotation.is_none() => {
                            rotation = Some(map.next_value::<i32>()?)
                        }
                        "items" if items.is_none() => {
                            items = Some(map.next_value_seed(ItemsSeed(self.0))?)
                        }
                        "page_number" | "view_box" | "rotation" | "items" => {
                            return Err(serde::de::Error::custom("duplicate page field"));
                        }
                        _ => {
                            return Err(serde::de::Error::unknown_field(
                                &key,
                                &["page_number", "view_box", "rotation", "items"],
                            ));
                        }
                    }
                }
                Ok(PdfJsPage {
                    page_number: number
                        .ok_or_else(|| serde::de::Error::missing_field("page_number"))?,
                    view_box: view.ok_or_else(|| serde::de::Error::missing_field("view_box"))?,
                    rotation: rotation
                        .ok_or_else(|| serde::de::Error::missing_field("rotation"))?,
                    items: items.ok_or_else(|| serde::de::Error::missing_field("items"))?,
                })
            }
        }
        deserializer.deserialize_map(PageVisitor(self.0))
    }
}

struct ItemsSeed<'a>(&'a mut DecodeBudget);
impl<'de> DeserializeSeed<'de> for ItemsSeed<'_> {
    type Value = Vec<PdfJsTextItem>;
    fn deserialize<D: Deserializer<'de>>(self, deserializer: D) -> Result<Self::Value, D::Error> {
        struct ItemsVisitor<'a>(&'a mut DecodeBudget);
        impl<'de> Visitor<'de> for ItemsVisitor<'_> {
            type Value = Vec<PdfJsTextItem>;
            fn expecting(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                f.write_str("a bounded text item sequence")
            }
            fn visit_seq<A: SeqAccess<'de>>(self, mut seq: A) -> Result<Self::Value, A::Error> {
                let mut items = Vec::new();
                loop {
                    if self.0.items >= self.0.limits.max_items {
                        let _ =
                            seq.next_element_seed(RejectExtraValue("text item count exceeded"))?;
                        break;
                    }
                    items.try_reserve(1).map_err(|_| {
                        serde::de::Error::custom("PDFJS_LIMIT:item allocation failed")
                    })?;
                    let Some(item) = seq.next_element_seed(ItemSeed(self.0))? else {
                        break;
                    };
                    items.push(item);
                }
                Ok(items)
            }
        }
        deserializer.deserialize_seq(ItemsVisitor(self.0))
    }
}

struct ItemSeed<'a>(&'a mut DecodeBudget);
impl<'de> DeserializeSeed<'de> for ItemSeed<'_> {
    type Value = PdfJsTextItem;
    fn deserialize<D: Deserializer<'de>>(self, deserializer: D) -> Result<Self::Value, D::Error> {
        struct ItemVisitor<'a>(&'a mut DecodeBudget);
        impl<'de> Visitor<'de> for ItemVisitor<'_> {
            type Value = PdfJsTextItem;
            fn expecting(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                f.write_str("a PDF.js text item")
            }
            fn visit_map<A: MapAccess<'de>>(self, mut map: A) -> Result<Self::Value, A::Error> {
                let mut text = None;
                let mut width = None;
                let mut height = None;
                let mut transform = None;
                let mut font = None;
                while let Some(key) = map.next_key::<String>()? {
                    match key.as_str() {
                        "text" if text.is_none() => {
                            text = Some(map.next_value_seed(BoundedStringSeed {
                                field_max: self.0.limits.max_item_text_bytes,
                                total: &mut self.0.text_bytes,
                                total_max: self.0.limits.max_text_bytes,
                            })?)
                        }
                        "font_name" if font.is_none() => {
                            font = Some(map.next_value_seed(BoundedStringSeed {
                                field_max: self.0.limits.max_font_name_bytes,
                                total: &mut self.0.font_bytes,
                                total_max: self.0.limits.max_font_names_bytes,
                            })?)
                        }
                        "width" if width.is_none() => width = Some(map.next_value::<f64>()?),
                        "height" if height.is_none() => height = Some(map.next_value::<f64>()?),
                        "transform" if transform.is_none() => {
                            transform = Some(map.next_value::<[f64; 6]>()?)
                        }
                        "text" | "font_name" | "width" | "height" | "transform" => {
                            return Err(serde::de::Error::custom("duplicate text item field"));
                        }
                        _ => {
                            return Err(serde::de::Error::unknown_field(
                                &key,
                                &["text", "width", "height", "transform", "font_name"],
                            ));
                        }
                    }
                }
                self.0.items = self
                    .0
                    .items
                    .checked_add(1)
                    .ok_or_else(|| serde::de::Error::custom("text item limit exceeded"))?;
                Ok(PdfJsTextItem {
                    text: text.ok_or_else(|| serde::de::Error::missing_field("text"))?,
                    width: width.ok_or_else(|| serde::de::Error::missing_field("width"))?,
                    height: height.ok_or_else(|| serde::de::Error::missing_field("height"))?,
                    transform: transform
                        .ok_or_else(|| serde::de::Error::missing_field("transform"))?,
                    font_name: font.ok_or_else(|| serde::de::Error::missing_field("font_name"))?,
                })
            }
        }
        deserializer.deserialize_map(ItemVisitor(self.0))
    }
}

struct BoundedStringSeed<'a> {
    field_max: usize,
    total: &'a mut usize,
    total_max: usize,
}
impl<'de> DeserializeSeed<'de> for BoundedStringSeed<'_> {
    type Value = String;
    fn deserialize<D: Deserializer<'de>>(self, deserializer: D) -> Result<String, D::Error> {
        struct StringVisitor<'a>(BoundedStringSeed<'a>);
        impl<'de> Visitor<'de> for StringVisitor<'_> {
            type Value = String;
            fn expecting(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                f.write_str("a bounded UTF-8 string")
            }
            fn visit_str<E: serde::de::Error>(self, value: &str) -> Result<String, E> {
                retain_bounded::<E>(self.0, value)
            }
            fn visit_borrowed_str<E: serde::de::Error>(self, value: &'de str) -> Result<String, E> {
                retain_bounded::<E>(self.0, value)
            }
            fn visit_string<E: serde::de::Error>(mut self, value: String) -> Result<String, E> {
                check_and_charge::<E>(&mut self.0, value.len())?;
                Ok(value)
            }
        }
        deserializer.deserialize_string(StringVisitor(self))
    }
}

fn check_and_charge<E: serde::de::Error>(
    seed: &mut BoundedStringSeed<'_>,
    len: usize,
) -> Result<(), E> {
    if len > seed.field_max {
        return Err(E::custom("PDFJS_LIMIT:field byte cap exceeded"));
    }
    let total = seed
        .total
        .checked_add(len)
        .ok_or_else(|| E::custom("PDFJS_LIMIT:aggregate byte cap overflow"))?;
    if total > seed.total_max {
        return Err(E::custom("PDFJS_LIMIT:aggregate byte cap exceeded"));
    }
    *seed.total = total;
    Ok(())
}

fn retain_bounded<E: serde::de::Error>(
    mut seed: BoundedStringSeed<'_>,
    value: &str,
) -> Result<String, E> {
    check_and_charge::<E>(&mut seed, value.len())?;
    let mut owned = String::new();
    owned
        .try_reserve_exact(value.len())
        .map_err(|_| E::custom("PDFJS_LIMIT:string allocation failed"))?;
    owned.push_str(value);
    Ok(owned)
}

struct MetadataSeed<'a>(&'a mut DecodeBudget);
impl<'de> DeserializeSeed<'de> for MetadataSeed<'_> {
    type Value = PdfJsMetadata;
    fn deserialize<D: Deserializer<'de>>(self, deserializer: D) -> Result<Self::Value, D::Error> {
        struct MetadataVisitor<'a>(&'a mut DecodeBudget);
        impl<'de> Visitor<'de> for MetadataVisitor<'_> {
            type Value = PdfJsMetadata;
            fn expecting(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                f.write_str("fixed PDF Info fields")
            }
            fn visit_map<A: MapAccess<'de>>(self, mut map: A) -> Result<Self::Value, A::Error> {
                let mut values: [Option<Option<String>>; 7] =
                    [None, None, None, None, None, None, None];
                while let Some(key) = map.next_key::<String>()? {
                    let index = match key.as_str() {
                        "title" => 0,
                        "author" => 1,
                        "creator" => 2,
                        "subject" => 3,
                        "keywords" => 4,
                        "creation_date" => 5,
                        "modified_date" => 6,
                        _ => {
                            return Err(serde::de::Error::unknown_field(
                                &key,
                                &[
                                    "title",
                                    "author",
                                    "creator",
                                    "subject",
                                    "keywords",
                                    "creation_date",
                                    "modified_date",
                                ],
                            ));
                        }
                    };
                    if values[index].is_some() {
                        return Err(serde::de::Error::custom("duplicate metadata field"));
                    }
                    let value = map.next_value_seed(OptionalStringSeed { budget: self.0 })?;
                    values[index] = Some(value);
                }
                let mut take =
                    |index: usize, name: &'static str| -> Result<Option<String>, A::Error> {
                        values[index]
                            .take()
                            .ok_or_else(|| serde::de::Error::missing_field(name))
                    };
                Ok(PdfJsMetadata {
                    title: take(0, "title")?,
                    author: take(1, "author")?,
                    creator: take(2, "creator")?,
                    subject: take(3, "subject")?,
                    keywords: take(4, "keywords")?,
                    creation_date: take(5, "creation_date")?,
                    modified_date: take(6, "modified_date")?,
                })
            }
        }
        deserializer.deserialize_map(MetadataVisitor(self.0))
    }
}

struct OptionalStringSeed<'a> {
    budget: &'a mut DecodeBudget,
}
impl<'de> DeserializeSeed<'de> for OptionalStringSeed<'_> {
    type Value = Option<String>;
    fn deserialize<D: Deserializer<'de>>(self, deserializer: D) -> Result<Self::Value, D::Error> {
        struct OptionalVisitor<'a>(&'a mut DecodeBudget);
        impl<'de> Visitor<'de> for OptionalVisitor<'_> {
            type Value = Option<String>;
            fn expecting(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                f.write_str("null or metadata string")
            }
            fn visit_none<E: serde::de::Error>(self) -> Result<Self::Value, E> {
                Ok(None)
            }
            fn visit_unit<E: serde::de::Error>(self) -> Result<Self::Value, E> {
                Ok(None)
            }
            fn visit_some<D: Deserializer<'de>>(self, de: D) -> Result<Self::Value, D::Error> {
                BoundedStringSeed {
                    field_max: self.0.limits.max_metadata_value_bytes,
                    total: &mut self.0.metadata_bytes,
                    total_max: self.0.limits.max_metadata_bytes,
                }
                .deserialize(de)
                .map(Some)
            }
        }
        deserializer.deserialize_option(OptionalVisitor(self.budget))
    }
}

pub(super) fn extract(
    bytes: &[u8],
    limits: TextDocumentLimits,
) -> Result<PdfJsTextDocument, KordocError> {
    if bytes.is_empty() || !bytes.starts_with(b"%PDF-") {
        return Err(parse_error("PDF input is malformed"));
    }
    if bytes.len() > 32 * 1024 * 1024 {
        return Err(output_error("PDF input exceeds runtime limit"));
    }
    let (document, json) = super::engine::extract_text_document(bytes, limits)?;
    validate_text_document(&document, limits)?;
    if json > limits.max_response_bytes {
        return Err(output_error("PDF.js output exceeds runtime limit"));
    }
    Ok(document)
}

pub(crate) fn validate_text_document(
    document: &PdfJsTextDocument,
    limits: TextDocumentLimits,
) -> Result<(), KordocError> {
    let fail = || output_error("PDF.js text document exceeds runtime limits");
    let page_count = usize::try_from(document.page_count).map_err(|_| fail())?;
    if page_count == 0 || document.pages.len() != page_count {
        return Err(parse_error("PDF.js page count does not match pages"));
    }
    if page_count > limits.max_pages {
        return Err(fail());
    }
    let mut items = 0usize;
    let mut text_bytes = 0usize;
    let mut font_bytes = 0usize;
    for (index, page) in document.pages.iter().enumerate() {
        if page.page_number as usize != index + 1
            || page.rotation % 90 != 0
            || page.view_box.iter().any(|value| !value.is_finite())
        {
            return Err(parse_error("PDF.js returned invalid page geometry"));
        }
        items = items.checked_add(page.items.len()).ok_or_else(fail)?;
        if items > limits.max_items {
            return Err(fail());
        }
        for item in &page.items {
            if !item.width.is_finite()
                || !item.height.is_finite()
                || item.transform.iter().any(|value| !value.is_finite())
            {
                return Err(parse_error("PDF.js returned invalid text geometry"));
            }
            let text_len = item.text.len();
            let font_len = item.font_name.len();
            if item.font_name.chars().any(char::is_control) {
                return Err(parse_error("PDF.js returned an invalid font name"));
            }
            if text_len > limits.max_item_text_bytes || font_len > limits.max_font_name_bytes {
                return Err(fail());
            }
            text_bytes = text_bytes.checked_add(text_len).ok_or_else(fail)?;
            font_bytes = font_bytes.checked_add(font_len).ok_or_else(fail)?;
            if text_bytes > limits.max_text_bytes || font_bytes > limits.max_font_names_bytes {
                return Err(fail());
            }
        }
    }
    let metadata = [
        &document.metadata.title,
        &document.metadata.author,
        &document.metadata.creator,
        &document.metadata.subject,
        &document.metadata.keywords,
        &document.metadata.creation_date,
        &document.metadata.modified_date,
    ];
    let mut metadata_bytes = 0usize;
    for value in metadata.into_iter().flatten() {
        if value.len() > limits.max_metadata_value_bytes {
            return Err(fail());
        }
        metadata_bytes = metadata_bytes.checked_add(value.len()).ok_or_else(fail)?;
    }
    if metadata_bytes > limits.max_metadata_bytes {
        return Err(fail());
    }
    Ok(())
}

pub(crate) fn deserialize_text_document(
    bytes: &[u8],
    limits: TextDocumentLimits,
) -> Result<PdfJsTextDocument, KordocError> {
    if bytes.len() > limits.max_response_bytes {
        return Err(output_error("PDF.js response exceeds runtime limit"));
    }
    let mut deserializer = serde_json::Deserializer::from_slice(bytes);
    let document = DocumentSeed(limits)
        .deserialize(&mut deserializer)
        .map_err(|error| {
            let message = error.to_string();
            if is_limit_error(&message) {
                output_error("PDF.js text document exceeds runtime limits")
            } else {
                parse_error("PDF.js returned an invalid text document")
            }
        })?;
    deserializer
        .end()
        .map_err(|_| parse_error("PDF.js returned trailing text document bytes"))?;
    validate_text_document(&document, limits)?;
    Ok(document)
}

pub(crate) fn is_limit_error(message: &str) -> bool {
    message.starts_with("PDFJS_LIMIT:")
}

fn parse_error(message: &str) -> KordocError {
    KordocError::new(ErrorCode::ParseError, message)
}

fn output_error(message: &str) -> KordocError {
    KordocError::new(ErrorCode::OutputTooLarge, message)
}
