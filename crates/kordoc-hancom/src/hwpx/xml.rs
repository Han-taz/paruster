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
const MAX_XML_NODES: usize = 100_000;
const MAX_XML_ATTRIBUTES: usize = 100_000;
const MAX_XML_TREE_BYTES: usize = 32 * 1024 * 1024;
const NODE_ESTIMATED_BYTES: usize = 192;
const ATTRIBUTE_ESTIMATED_BYTES: usize = 96;
const CONTENT_ESTIMATED_BYTES: usize = 64;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum XmlContent {
    Text { start: usize, end: usize },
    Child(usize),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct XmlNode {
    pub(crate) name: String,
    pub(crate) attributes: BTreeMap<String, String>,
    pub(crate) text: String,
    pub(crate) children: Vec<Self>,
    pub(crate) content: Vec<XmlContent>,
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
        let mut text = String::new();
        for part in &self.content {
            match part {
                XmlContent::Text { start, end } => text.push_str(&self.text[*start..*end]),
                XmlContent::Child(index) => text.push_str(&self.children[*index].text_content()),
            }
        }
        text
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum XmlFault {
    InputLimit,
    TextLimit,
    DepthLimit,
    TreeLimit,
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
        matches!(
            self.fault,
            XmlFault::InputLimit | XmlFault::TextLimit | XmlFault::TreeLimit
        )
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
    let mut reader = Reader::from_str(source);
    reader.config_mut().trim_text(false);
    reader.config_mut().check_end_names = true;
    reader.config_mut().expand_empty_elements = false;

    let mut stack: Vec<XmlNode> = Vec::new();
    let mut root = None;
    let mut budget = XmlBudget::default();
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
                stack.push(node_from_start(&element, &mut budget)?);
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
                append_node(
                    &mut stack,
                    &mut root,
                    node_from_start(&element, &mut budget)?,
                    &mut budget,
                )?;
            }
            Event::End(_) => {
                let node = stack.pop().ok_or(XmlError {
                    fault: XmlFault::Malformed,
                    message: "malformed XML",
                })?;
                append_node(&mut stack, &mut root, node, &mut budget)?;
            }
            Event::Text(event) => {
                let decoded = event.xml_content(XmlVersion::Implicit1_0);
                let value = unescape(&decoded).map_err(|_| XmlError {
                    fault: XmlFault::Malformed,
                    message: "invalid XML entity reference",
                })?;
                add_text(&mut stack, &mut budget, value)?;
            }
            Event::CData(event) => {
                let value = event.xml_content(XmlVersion::Implicit1_0);
                add_text(&mut stack, &mut budget, value)?;
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
                add_text(&mut stack, &mut budget, Cow::Owned(value.to_string()))?;
            }
            Event::DocType(_) => {
                return Err(XmlError {
                    fault: XmlFault::ForbiddenDeclaration,
                    message: "DTD and entity declarations are disabled",
                });
            }
            Event::Eof => break,
            Event::Decl(declaration) => {
                if let Some(encoding) = declaration.encoding() {
                    let encoding = encoding.map_err(|_| XmlError {
                        fault: XmlFault::Malformed,
                        message: "malformed XML declaration",
                    })?;
                    if !encoding.eq_ignore_ascii_case("utf-8")
                        && !encoding.eq_ignore_ascii_case("utf8")
                    {
                        return Err(XmlError {
                            fault: XmlFault::InvalidUtf8,
                            message: "XML declaration must specify UTF-8",
                        });
                    }
                }
            }
            Event::Comment(_) | Event::PI(_) => {}
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

#[derive(Debug, Default)]
struct XmlBudget {
    nodes: usize,
    attributes: usize,
    text_bytes: usize,
    tree_bytes: usize,
}

impl XmlBudget {
    fn charge_tree(&mut self, bytes: usize) -> Result<(), XmlError> {
        self.tree_bytes = self.tree_bytes.checked_add(bytes).ok_or(XmlError {
            fault: XmlFault::TreeLimit,
            message: "XML tree exceeds the configured allocation bound",
        })?;
        if self.tree_bytes > MAX_XML_TREE_BYTES {
            return Err(XmlError {
                fault: XmlFault::TreeLimit,
                message: "XML tree exceeds the configured allocation bound",
            });
        }
        Ok(())
    }

    fn charge_node(&mut self, name_bytes: usize) -> Result<(), XmlError> {
        self.nodes = self.nodes.checked_add(1).ok_or(XmlError {
            fault: XmlFault::TreeLimit,
            message: "XML node count exceeds the configured bound",
        })?;
        if self.nodes > MAX_XML_NODES {
            return Err(XmlError {
                fault: XmlFault::TreeLimit,
                message: "XML node count exceeds the configured bound",
            });
        }
        self.charge_tree(
            NODE_ESTIMATED_BYTES
                .checked_add(name_bytes)
                .ok_or(XmlError {
                    fault: XmlFault::TreeLimit,
                    message: "XML tree exceeds the configured allocation bound",
                })?,
        )
    }

    fn charge_attributes(&mut self, count: usize, bytes: usize) -> Result<(), XmlError> {
        self.attributes = self.attributes.checked_add(count).ok_or(XmlError {
            fault: XmlFault::TreeLimit,
            message: "XML attribute count exceeds the configured bound",
        })?;
        if self.attributes > MAX_XML_ATTRIBUTES {
            return Err(XmlError {
                fault: XmlFault::TreeLimit,
                message: "XML attribute count exceeds the configured bound",
            });
        }
        self.charge_tree(bytes)
    }

    fn charge_text(&mut self, bytes: usize) -> Result<(), XmlError> {
        self.text_bytes = self.text_bytes.checked_add(bytes).ok_or(XmlError {
            fault: XmlFault::TextLimit,
            message: "XML text exceeds the configured bound",
        })?;
        if self.text_bytes > MAX_TEXT_BYTES {
            return Err(XmlError {
                fault: XmlFault::TextLimit,
                message: "XML text exceeds the configured bound",
            });
        }
        self.charge_tree(CONTENT_ESTIMATED_BYTES.checked_add(bytes).ok_or(XmlError {
            fault: XmlFault::TreeLimit,
            message: "XML tree exceeds the configured allocation bound",
        })?)
    }
}

fn node_from_start(
    element: &quick_xml::events::BytesStart<'_>,
    budget: &mut XmlBudget,
) -> Result<XmlNode, XmlError> {
    let element_name = element.name();
    let name = local_name(element_name.as_ref());
    budget.charge_node(name.len())?;
    let mut attribute_count = 0usize;
    let mut attribute_bytes = 0usize;
    for result in element.attributes().with_checks(false) {
        let attribute = result.map_err(|_| XmlError {
            fault: XmlFault::Malformed,
            message: "malformed XML attributes",
        })?;
        attribute_count = attribute_count.checked_add(1).ok_or(XmlError {
            fault: XmlFault::TreeLimit,
            message: "XML attribute count exceeds the configured bound",
        })?;
        attribute_bytes = attribute_bytes
            .checked_add(ATTRIBUTE_ESTIMATED_BYTES)
            .and_then(|bytes| bytes.checked_add(attribute.key.as_ref().len()))
            .and_then(|bytes| bytes.checked_add(attribute.value.len()))
            .ok_or(XmlError {
                fault: XmlFault::TreeLimit,
                message: "XML tree exceeds the configured allocation bound",
            })?;
    }
    budget.charge_attributes(attribute_count, attribute_bytes)?;
    let mut attributes = BTreeMap::new();
    for result in element.attributes().with_checks(false) {
        let attribute = result.map_err(|_| XmlError {
            fault: XmlFault::Malformed,
            message: "malformed XML attributes",
        })?;
        let key = local_name(attribute.key.as_ref()).to_owned();
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
        name: name.to_owned(),
        attributes,
        text: String::new(),
        children: Vec::new(),
        content: Vec::new(),
    })
}

fn local_name(name: &str) -> &str {
    name.rsplit(':').next().unwrap_or(name)
}

fn add_text(
    stack: &mut [XmlNode],
    budget: &mut XmlBudget,
    value: Cow<'_, str>,
) -> Result<(), XmlError> {
    budget.charge_text(value.len())?;
    if let Some(parent) = stack.last_mut() {
        let start = parent.text.len();
        parent.text.push_str(&value);
        let end = parent.text.len();
        parent.content.push(XmlContent::Text { start, end });
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
    budget: &mut XmlBudget,
) -> Result<(), XmlError> {
    if let Some(parent) = stack.last_mut() {
        budget.charge_tree(CONTENT_ESTIMATED_BYTES)?;
        let index = parent.children.len();
        parent.children.push(node);
        parent.content.push(XmlContent::Child(index));
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
