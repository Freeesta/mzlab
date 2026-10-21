//! Minimal HTML reading for the Teoria pages: tokens, attributes, entities. Used by `blocchi`; the pages are generated, so well-formed.
/// Elements whose content is never text for the tutor (scripts, drawings, interactive figures, menus).
pub(crate) fn skipped(name: &str, classes: &str) -> bool {
    matches!(
        name,
        "script"
            | "style"
            | "nav"
            | "svg"
            | "noscript"
            | "canvas"
            | "button"
            | "select"
            | "textarea"
            | "head"
    ) || (name == "div"
        && classes
            .split_whitespace()
            .any(|c| c == "sim" || c == "eispec"))
}

pub(crate) fn is_void(name: &str) -> bool {
    matches!(
        name,
        "br" | "img" | "meta" | "link" | "hr" | "input" | "wbr"
    )
}

/// Tags that end the paragraph being collected.
pub(crate) fn breaks(name: &str) -> bool {
    matches!(
        name,
        "p" | "li"
            | "tr"
            | "ul"
            | "ol"
            | "table"
            | "div"
            | "h1"
            | "h2"
            | "h3"
            | "h4"
            | "dt"
            | "dd"
            | "details"
            | "summary"
            | "figure"
            | "figcaption"
            | "blockquote"
            | "section"
            | "aside"
            | "hr"
            | "br"
            | "caption"
            | "pre"
            | "dl"
    )
}

pub(crate) struct El {
    pub name: String,
    pub classes: String,
    pub id: Option<String>,
}

pub(crate) enum Tok<'a> {
    Open(String, &'a str),
    Close(String),
    Text(&'a str),
}

/// Value of `name="..."` in the attribute text of a tag.
pub(crate) fn attr(attrs: &str, name: &str) -> Option<String> {
    let key = format!("{name}=\"");
    let mut from = 0;
    while let Some(i) = attrs[from..].find(&key) {
        let at = from + i;
        if at == 0 || attrs.as_bytes()[at - 1].is_ascii_whitespace() {
            let v = &attrs[at + key.len()..];
            return v.find('"').map(|e| v[..e].to_string());
        }
        from = at + key.len();
    }
    None
}

pub(crate) fn tokens(html: &str) -> Vec<Tok<'_>> {
    let mut out = Vec::new();
    let mut at = 0;
    while at < html.len() {
        match html[at..].find('<') {
            None => {
                out.push(Tok::Text(&html[at..]));
                break;
            }
            Some(p) => {
                if p > 0 {
                    out.push(Tok::Text(&html[at..at + p]));
                }
                let start = at + p;
                let rest = &html[start..];
                if rest.starts_with("<!--") {
                    at = rest.find("-->").map_or(html.len(), |e| start + e + 3);
                    continue;
                }
                let Some(gt) = rest.find('>') else {
                    out.push(Tok::Text(rest));
                    break;
                };
                let inner = &rest[1..gt];
                at = start + gt + 1;
                if let Some(name) = inner.strip_prefix('/') {
                    out.push(Tok::Close(name.trim().to_ascii_lowercase()));
                } else if inner.starts_with(|c: char| c.is_ascii_alphabetic()) {
                    let end = inner
                        .find(|c: char| c.is_ascii_whitespace() || c == '/')
                        .unwrap_or(inner.len());
                    out.push(Tok::Open(inner[..end].to_ascii_lowercase(), &inner[end..]));
                    if matches!(
                        inner[..end].to_ascii_lowercase().as_str(),
                        "script" | "style"
                    ) {
                        let close = format!("</{}", inner[..end].to_ascii_lowercase());
                        at = html[at..].find(&close).map_or(html.len(), |e| at + e);
                    }
                }
            }
        }
    }
    out
}

pub(crate) fn decode(text: &str, out: &mut String) {
    let mut rest = text;
    while let Some(a) = rest.find('&') {
        out.push_str(&rest[..a]);
        rest = &rest[a..];
        let known = rest
            .find(';')
            .filter(|&e| e <= 10)
            .and_then(|e| entity(&rest[1..e]).map(|c| (e, c)));
        match known {
            Some((e, c)) => {
                out.push(c);
                rest = &rest[e + 1..];
            }
            None => {
                out.push('&');
                rest = &rest[1..];
            }
        }
    }
    out.push_str(rest);
}

fn entity(name: &str) -> Option<char> {
    if let Some(n) = name.strip_prefix('#') {
        let code = match n.strip_prefix(['x', 'X']) {
            Some(h) => u32::from_str_radix(h, 16).ok()?,
            None => n.parse().ok()?,
        };
        return char::from_u32(code);
    }
    Some(match name {
        "amp" => '&',
        "lt" => '<',
        "gt" => '>',
        "quot" => '"',
        "apos" => '\'',
        "nbsp" => ' ',
        "ndash" => '–',
        "mdash" => '—',
        "hellip" => '…',
        "lsquo" => '‘',
        "rsquo" => '’',
        "ldquo" => '“',
        "rdquo" => '”',
        "laquo" => '«',
        "raquo" => '»',
        "middot" => '·',
        "times" => '×',
        "minus" => '−',
        "plusmn" => '±',
        "deg" => '°',
        _ => return None,
    })
}

pub(crate) fn squash(s: &str) -> String {
    s.split_whitespace().collect::<Vec<_>>().join(" ")
}
