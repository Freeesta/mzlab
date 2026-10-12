//! Everything the side files of a `.D` folder say, kept as (origin, name, value) so that nothing the instrument wrote is dropped: XML files are flattened
//! (an element with a `Name` and a `Value` child becomes `Name = Value`, other leaves are `path = text`, attributes `path@attribute = value`), `.ini` files are
//! `[section] key = value`, the method report gives its tune parameters, and the text of the plain files (method report, run start, converter log) is kept whole.
//! A file that cannot be read this way is kept as one item with its size. Binary data files (`MSScan.bin`, `MSPeak.bin`, `data.ms`) are not listed here.

use quick_xml::events::Event;
use quick_xml::Reader;

#[derive(Debug, Clone, PartialEq)]
pub struct Item {
    /// The file the item comes from, relative to the `.D` folder (`AcqData/Devices.xml`).
    pub origin: String,
    pub name: String,
    pub value: String,
}

#[derive(Default)]
struct Node {
    name: String,
    attrs: Vec<(String, String)>,
    text: String,
    kids: Vec<Node>,
}

fn text_of(b: &[u8]) -> String {
    String::from_utf8_lossy(b)
        .trim_start_matches('\u{feff}')
        .to_string()
}

fn attrs_of(e: &quick_xml::events::BytesStart) -> Vec<(String, String)> {
    e.attributes()
        .with_checks(false)
        .filter_map(|a| a.ok())
        .map(|a| {
            (
                a.key.as_ref().to_string(),
                a.normalized_value(quick_xml::XmlVersion::Implicit1_0)
                    .map(|c| c.into_owned())
                    .unwrap_or_default(),
            )
        })
        .collect()
}

fn close(stack: &mut Vec<Node>) {
    if stack.len() > 1 {
        if let Some(n) = stack.pop() {
            if let Some(p) = stack.last_mut() {
                p.kids.push(n);
            }
        }
    }
}

fn tree(xml: &str) -> Option<Node> {
    let mut r = Reader::from_str(xml);
    let mut stack = vec![Node::default()];
    loop {
        match r.read_event().ok()? {
            Event::Start(e) => stack.push(Node {
                name: e.name().as_ref().to_string(),
                attrs: attrs_of(&e),
                ..Default::default()
            }),
            Event::Empty(e) => {
                stack.push(Node {
                    name: e.name().as_ref().to_string(),
                    attrs: attrs_of(&e),
                    ..Default::default()
                });
                close(&mut stack);
            }
            Event::End(_) => close(&mut stack),
            Event::Text(t) => {
                if let Some(n) = stack.last_mut() {
                    n.text.push_str(&t.xml10_content());
                }
            }
            Event::CData(t) => {
                if let Some(n) = stack.last_mut() {
                    n.text.push_str(&t.into_inner());
                }
            }
            Event::Eof => break,
            _ => {}
        }
    }
    while stack.len() > 1 {
        close(&mut stack);
    }
    stack.pop()
}

fn emit(n: &Node, path: &str, origin: &str, out: &mut Vec<Item>) {
    let here = if path.is_empty() {
        n.name.clone()
    } else {
        format!("{path}/{}", n.name)
    };
    let kid = |k: &str| {
        n.kids
            .iter()
            .find(|c| c.name == k)
            .map(|c| c.text.trim().to_string())
    };
    if let (Some(name), Some(value)) = (kid("Name"), kid("Value")) {
        out.push(Item {
            origin: origin.into(),
            name,
            value,
        });
        for c in n
            .kids
            .iter()
            .filter(|c| c.name != "Name" && c.name != "Value" && c.name != "DisplayName")
        {
            emit(c, &here, origin, out);
        }
        return;
    }
    for (k, v) in &n.attrs {
        out.push(Item {
            origin: origin.into(),
            name: format!("{here}@{k}"),
            value: v.clone(),
        });
    }
    let t = n.text.trim();
    if n.kids.is_empty() && !t.is_empty() {
        out.push(Item {
            origin: origin.into(),
            name: here.clone(),
            value: t.to_string(),
        });
    }
    for c in &n.kids {
        emit(c, &here, origin, out);
    }
}

fn ini(text: &str, origin: &str, out: &mut Vec<Item>) {
    let mut section = String::new();
    for l in text.lines().map(str::trim).filter(|l| !l.is_empty()) {
        if let Some(s) = l.strip_prefix('[').and_then(|l| l.strip_suffix(']')) {
            section = s.to_string();
        } else if let Some((k, v)) = l.split_once('=') {
            let name = if section.is_empty() {
                k.trim().to_string()
            } else {
                format!("[{section}] {}", k.trim())
            };
            out.push(Item {
                origin: origin.into(),
                name,
                value: v.trim().to_string(),
            });
        }
    }
}

/// The tune parameters of the method report: lines `   <number>   :   NAME` between the tune header and its end.
fn tune(text: &str, origin: &str, out: &mut Vec<Item>) {
    let mut on = false;
    for l in text.lines() {
        if l.contains("TUNE PARAMETERS") {
            on = true;
        } else if l.contains("END OF TUNE PARAMETERS") {
            on = false;
        } else if on {
            if let Some((v, n)) = l.split_once(':') {
                if !n.trim().is_empty() && !v.trim().is_empty() {
                    out.push(Item {
                        origin: origin.into(),
                        name: format!("tune {}", n.trim()),
                        value: v.trim().to_string(),
                    });
                }
            }
        }
    }
}

/// The items of the given files (`name` relative to the `.D` folder, bytes).
pub fn collect(files: &[(&str, &[u8])]) -> Vec<Item> {
    let mut out = Vec::new();
    for (name, bytes) in files {
        let lower = name.to_lowercase();
        let before = out.len();
        if lower.ends_with(".xml") {
            if let Some(t) = tree(&text_of(bytes)) {
                emit(&t, "", name, &mut out);
            }
        } else if lower.ends_with(".ini") {
            ini(&text_of(bytes), name, &mut out);
        } else if lower.ends_with(".txt") || lower.ends_with(".xsd") {
            let t = super::method::decode(bytes);
            tune(&t, name, &mut out);
            out.push(Item {
                origin: name.to_string(),
                name: "text".into(),
                value: t,
            });
            continue;
        } else if lower.ends_with(".log") {
            out.push(Item {
                origin: name.to_string(),
                name: "text".into(),
                value: super::method::decode(bytes),
            });
            continue;
        }
        if out.len() == before {
            out.push(Item {
                origin: name.to_string(),
                name: "bytes".into(),
                value: bytes.len().to_string(),
            });
        }
    }
    out
}
