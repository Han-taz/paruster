//! Bounded, non-expanding XML reader shared by HWPX package consumers.

// H2a's private parser seam is intentionally not wired into crate entry points until H3.
#![allow(dead_code)]

use std::borrow::Cow;
use std::collections::BTreeMap;

use kordoc_ir::{ErrorCode, KordocError};
use quick_xml::Reader;
use quick_xml::XmlVersion;
use quick_xml::escape::unescape;
use quick_xml::events::Event;

const MAX_XML_BYTES: usize = 64 * 1024 * 1024;
const MAX_TEXT_BYTES: usize = 16 * 1024 * 1024;
const MAX_DEPTH: usize = 200;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct XmlNode {
    pub(crate) name: String,
    pub(crate) attributes: BTreeMap<String, String>,
    pub(crate) text: String,
    pub(crate) children: Vec<Self>,
}

impl XmlNode {
    pub(crate) fn attr(&self, name: &str) -> Option<&str> {
        self.attributes.get(name).map(String::as_str)
    }

    pub(crate) fn descendants<'a>(&'a self, name: &str) -> Vec<&'a Self> {
        fn collect<'a>(node: &'a XmlNode, name: &str, matches: &mut Vec<&'a XmlNode>) {
            if node.name == name {
                matches.push(node);
            }
            for child in &node.children {
                collect(child, name, matches);
            }
        }
        let mut matches = Vec::new();
        collect(self, name, &mut matches);
        matches
    }

    pub(crate) fn text_content(&self) -> String {
        let mut text = self.text.clone();
        for child in &self.children {
            text.push_str(&child.text_content());
        }
        text
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum XmlFault {
    InputLimit,
    TextLimit,
    DepthLimit,
    InvalidUtf8,
    ForbiddenDeclaration,
    Malformed,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct XmlError {
    pub(crate) fault: XmlFault,
    pub(crate) message: &'static str,
}

impl XmlError {
    pub(crate) fn is_resource_limit(&self) -> bool {
        matches!(self.fault, XmlFault::InputLimit | XmlFault::TextLimit)
    }

    fn critical_error(&self) -> KordocError {
        let code = if self.is_resource_limit() {
            ErrorCode::DecompressionBomb
        } else {
            ErrorCode::Corrupted
        };
        KordocError::new(code, self.message)
    }
}

/// Parses package-wide critical XML, mapping malformed XML to the stable hard-failure class.
pub(crate) fn parse_critical(bytes: &[u8]) -> Result<XmlNode, KordocError> {
    parse(bytes).map_err(|error| error.critical_error())
}

/// Parses a bounded XML byte slice into a compact event-built tree. Entity expansion is never
/// enabled: DTDs are rejected and only the five XML predefined references and numeric references
/// are accepted.
pub(crate) fn parse(bytes: &[u8]) -> Result<XmlNode, XmlError> {
    if bytes.len() > MAX_XML_BYTES {
        return Err(XmlError {
            fault: XmlFault::InputLimit,
            message: "XML input exceeds the configured bound",
        });
    }
    let source = std::str::from_utf8(bytes).map_err(|_| XmlError {
        fault: XmlFault::InvalidUtf8,
        message: "XML input is not valid UTF-8",
    })?;
    if let Some(declaration_end) = source.find("?>")
        && source.starts_with("<?xml")
    {
        let declaration = source[..declaration_end].to_ascii_lowercase();
        if let Some(start) = declaration.find("encoding") {
            let value = declaration[start + "encoding".len()..]
                .split_once('=')
                .map(|(_, value)| value.trim().trim_matches(['\'', '"']))
                .unwrap_or("");
            if value != "utf-8" && value != "utf8" {
                return Err(XmlError {
                    fault: XmlFault::InvalidUtf8,
                    message: "XML declaration must specify UTF-8",
                });
            }
        }
    }

    let mut reader = Reader::from_str(source);
    reader.config_mut().trim_text(false);
    reader.config_mut().check_end_names = true;
    reader.config_mut().expand_empty_elements = false;

    let mut stack: Vec<XmlNode> = Vec::new();
    let mut root = None;
    let mut text_bytes = 0usize;
    loop {
        let event = reader.read_event().map_err(|_| XmlError {
            fault: XmlFault::Malformed,
            message: "malformed XML",
        })?;
        match event {
            Event::Start(element) => {
                let depth = stack.len().checked_add(1).ok_or(XmlError {
                    fault: XmlFault::DepthLimit,
                    message: "XML depth exceeds the configured bound",
                })?;
                if depth > MAX_DEPTH {
                    return Err(XmlError {
                        fault: XmlFault::DepthLimit,
                        message: "XML depth exceeds the configured bound",
                    });
                }
                stack.push(node_from_start(&element)?);
            }
            Event::Empty(element) => {
                let depth = stack.len().checked_add(1).ok_or(XmlError {
                    fault: XmlFault::DepthLimit,
                    message: "XML depth exceeds the configured bound",
                })?;
                if depth > MAX_DEPTH {
                    return Err(XmlError {
                        fault: XmlFault::DepthLimit,
                        message: "XML depth exceeds the configured bound",
                    });
                }
                append_node(&mut stack, &mut root, node_from_start(&element)?)?;
            }
            Event::End(_) => {
                let node = stack.pop().ok_or(XmlError {
                    fault: XmlFault::Malformed,
                    message: "malformed XML",
                })?;
                append_node(&mut stack, &mut root, node)?;
            }
            Event::Text(event) => {
                let decoded = event.xml_content(XmlVersion::Implicit1_0);
                let value = unescape(&decoded).map_err(|_| XmlError {
                    fault: XmlFault::Malformed,
                    message: "invalid XML entity reference",
                })?;
                add_text(&mut stack, &mut text_bytes, value)?;
            }
            Event::CData(event) => {
                let value = event.xml_content(XmlVersion::Implicit1_0);
                add_text(&mut stack, &mut text_bytes, value)?;
            }
            Event::GeneralRef(reference) => {
                let reference_name = reference.as_ref();
                let value = match reference_name {
                    "amp" => '&',
                    "lt" => '<',
                    "gt" => '>',
                    "apos" => '\'',
                    "quot" => '"',
                    _ => reference
                        .resolve_char_ref()
                        .map_err(|_| XmlError {
                            fault: XmlFault::Malformed,
                            message: "invalid XML character reference",
                        })?
                        .ok_or(XmlError {
                            fault: XmlFault::ForbiddenDeclaration,
                            message: "custom XML entities are disabled",
                        })?,
                };
                add_text(&mut stack, &mut text_bytes, Cow::Owned(value.to_string()))?;
            }
            Event::DocType(_) => {
                return Err(XmlError {
                    fault: XmlFault::ForbiddenDeclaration,
                    message: "DTD and entity declarations are disabled",
                });
            }
            Event::Eof => break,
            Event::Decl(_) | Event::Comment(_) | Event::PI(_) => {}
        }
    }
    if !stack.is_empty() || root.is_none() {
        return Err(XmlError {
            fault: XmlFault::Malformed,
            message: "malformed XML",
        });
    }
    root.ok_or(XmlError {
        fault: XmlFault::Malformed,
        message: "malformed XML",
    })
}

fn node_from_start(element: &quick_xml::events::BytesStart<'_>) -> Result<XmlNode, XmlError> {
    let name = local_name(element.name().as_ref())?;
    let mut attributes = BTreeMap::new();
    for result in element.attributes().with_checks(true) {
        let attribute = result.map_err(|_| XmlError {
            fault: XmlFault::Malformed,
            message: "malformed XML attributes",
        })?;
        let key = local_name(attribute.key.as_ref())?;
        let value = attribute
            .normalized_value(XmlVersion::Implicit1_0)
            .map_err(|_| XmlError {
                fault: XmlFault::Malformed,
                message: "malformed XML attribute value",
            })?;
        if attributes.insert(key, value.into_owned()).is_some() {
            return Err(XmlError {
                fault: XmlFault::Malformed,
                message: "duplicate XML attribute",
            });
        }
    }
    Ok(XmlNode {
        name,
        attributes,
        text: String::new(),
        children: Vec::new(),
    })
}

fn local_name(name: &str) -> Result<String, XmlError> {
    Ok(name.rsplit(':').next().unwrap_or(name).to_owned())
}

fn add_text(stack: &mut [XmlNode], total: &mut usize, value: Cow<'_, str>) -> Result<(), XmlError> {
    *total = total.checked_add(value.len()).ok_or(XmlError {
        fault: XmlFault::TextLimit,
        message: "XML text exceeds the configured bound",
    })?;
    if *total > MAX_TEXT_BYTES {
        return Err(XmlError {
            fault: XmlFault::TextLimit,
            message: "XML text exceeds the configured bound",
        });
    }
    if let Some(parent) = stack.last_mut() {
        parent.text.push_str(&value);
    } else if !value.trim().is_empty() {
        return Err(XmlError {
            fault: XmlFault::Malformed,
            message: "text outside the document root",
        });
    }
    Ok(())
}

fn append_node(
    stack: &mut [XmlNode],
    root: &mut Option<XmlNode>,
    node: XmlNode,
) -> Result<(), XmlError> {
    if let Some(parent) = stack.last_mut() {
        parent.children.push(node);
    } else if root.replace(node).is_some() {
        return Err(XmlError {
            fault: XmlFault::Malformed,
            message: "multiple XML roots",
        });
    }
    Ok(())
}

#[cfg(test)]
mod tests;
