//! Inline HTML fragments of the preview, as the browser hands them back after an edit: parsed into nodes, put in a canonical form (to compare with what the
//! converter makes of the new source) and turned back into the inline Markdown of the dialect. Only the elements `inline.rs` writes are known; anything else is refused.

#[derive(Debug, Clone, PartialEq)]
pub(crate) enum Node {
    Text(String),
    El {
        tag: String,
        class: String,
        href: String,
        kids: Vec<Node>,
    },
}

/// The entities the converter and the browser write in text and attribute values.
fn decode(s: &str) -> String {
    s.replace("&nbsp;", "\u{a0}")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&quot;", "\"")
        .replace("&#x27;", "'")
        .replace("&#39;", "'")
        .replace("&amp;", "&")
}

fn escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}

/// `name="value"` / `name='value'` pairs of an open tag.
fn attrs(raw: &str) -> Vec<(String, String)> {
    let c: Vec<char> = raw.chars().collect();
    let mut out = Vec::new();
    let mut i = 0;
    while i < c.len() {
        while i < c.len() && c[i].is_whitespace() {
            i += 1;
        }
        let st = i;
        while i < c.len() && !c[i].is_whitespace() && c[i] != '=' {
            i += 1;
        }
        let name: String = c[st..i].iter().collect();
        let mut val = String::new();
        if c.get(i) == Some(&'=') {
            i += 1;
            if let Some(&q) = c.get(i).filter(|&&q| q == '"' || q == '\'') {
                let vs = i + 1;
                i = vs;
                while i < c.len() && c[i] != q {
                    i += 1;
                }
                val = c[vs..i].iter().collect();
                i += 1;
            } else {
                let vs = i;
                while i < c.len() && !c[i].is_whitespace() {
                    i += 1;
                }
                val = c[vs..i].iter().collect();
            }
        }
        if !name.is_empty() {
            out.push((name.to_lowercase(), decode(&val)));
        }
    }
    out
}

fn canon_tag(t: &str) -> Option<&'static str> {
    Some(match t {
        "b" | "strong" => "b",
        "i" => "i",
        "em" => "em",
        "sub" => "sub",
        "sup" => "sup",
        "code" => "code",
        "span" => "span",
        "a" => "a",
        "br" => "br",
        _ => return None,
    })
}

pub(crate) fn parse(html: &str) -> Result<Vec<Node>, String> {
    // each frame: (tag, class, href, kids so far)
    let mut stack: Vec<(String, String, String, Vec<Node>)> =
        vec![(String::new(), String::new(), String::new(), Vec::new())];
    let mut rest = html;
    while !rest.is_empty() {
        let Some(lt) = rest.find('<') else {
            push_text(&mut stack, rest);
            break;
        };
        push_text(&mut stack, &rest[..lt]);
        rest = &rest[lt..];
        let Some(gt) = rest.find('>') else {
            return Err("broken markup".into());
        };
        let inner = &rest[1..gt];
        rest = &rest[gt + 1..];
        if let Some(close) = inner.strip_prefix('/') {
            let tag = canon_tag(&close.trim().to_lowercase())
                .ok_or("formatting that can't be kept here")?;
            close_frame(&mut stack, tag)?;
        } else {
            let (name, raw) =
                inner.split_at(inner.find(char::is_whitespace).unwrap_or(inner.len()));
            let tag = canon_tag(name.to_lowercase().trim_end_matches('/'))
                .ok_or_else(|| format!("<{name}> formatting can't be kept here"))?;
            let at = attrs(raw.trim_end_matches('/'));
            let get = |k: &str| {
                at.iter()
                    .find(|a| a.0 == k)
                    .map(|a| a.1.clone())
                    .unwrap_or_default()
            };
            if tag == "br" {
                stack.last_mut().unwrap().3.push(Node::El {
                    tag: "br".into(),
                    class: String::new(),
                    href: String::new(),
                    kids: vec![],
                });
            } else {
                stack.push((tag.to_string(), get("class"), get("href"), Vec::new()));
            }
        }
    }
    if stack.len() != 1 {
        return Err("broken markup".into());
    }
    let mut kids = stack.pop().unwrap().3;
    while matches!(kids.last(), Some(Node::El { tag, .. }) if tag == "br") {
        kids.pop();
    }
    Ok(trim_edges(kids))
}

fn push_text(stack: &mut [(String, String, String, Vec<Node>)], t: &str) {
    if !t.is_empty() {
        stack.last_mut().unwrap().3.push(Node::Text(decode(t)));
    }
}

fn close_frame(
    stack: &mut Vec<(String, String, String, Vec<Node>)>,
    tag: &str,
) -> Result<(), String> {
    if tag == "br" {
        return Ok(());
    }
    if stack.len() < 2 || stack.last().unwrap().0 != tag {
        return Err("broken markup".into());
    }
    let (t, class, href, kids) = stack.pop().unwrap();
    let parent = &mut stack.last_mut().unwrap().3;
    if t == "span" && !matches!(class.as_str(), "tex" | "cit" | "ipotesi") {
        // a span with no meaning of ours (the browser adds style spans) is dropped, its content kept
        parent.extend(kids);
    } else {
        parent.push(Node::El {
            tag: t,
            class,
            href,
            kids,
        });
    }
    Ok(())
}

/// Leading and trailing white space of the whole fragment goes (the converter trims a paragraph too); non-breaking spaces count as spaces.
fn trim_edges(mut kids: Vec<Node>) -> Vec<Node> {
    if let Some(Node::Text(t)) = kids.first_mut() {
        *t = t
            .trim_start_matches(|c: char| c.is_whitespace())
            .to_string();
    }
    if let Some(Node::Text(t)) = kids.last_mut() {
        *t = t.trim_end_matches(|c: char| c.is_whitespace()).to_string();
    }
    kids.retain(|k| !matches!(k, Node::Text(t) if t.is_empty()));
    kids
}

/// Same text, same elements, same class and link: the form two fragments are compared in.
pub(crate) fn canon(nodes: &[Node]) -> String {
    let mut out = String::new();
    for n in nodes {
        match n {
            Node::Text(t) => out.push_str(&escape(&t.replace('\u{a0}', " "))),
            Node::El {
                tag,
                class,
                href,
                kids,
            } => {
                out.push_str(&format!("<{tag}"));
                if !class.is_empty() {
                    out.push_str(&format!(" class=\"{class}\""));
                }
                if !href.is_empty() {
                    out.push_str(&format!(" href=\"{}\"", escape(href)));
                }
                out.push('>');
                out.push_str(&canon(kids));
                out.push_str(&format!("</{tag}>"));
            }
        }
    }
    out
}

fn text_of(kids: &[Node]) -> Result<String, String> {
    let mut s = String::new();
    for k in kids {
        match k {
            Node::Text(t) => s.push_str(t),
            _ => {
                return Err("formatting inside code, formulas and links can't be kept here".into())
            }
        }
    }
    Ok(s)
}

/// The inline Markdown of the dialect for these nodes.
pub(crate) fn to_md(nodes: &[Node]) -> Result<String, String> {
    let mut out = String::new();
    for n in nodes {
        match n {
            Node::Text(t) => out.push_str(t),
            Node::El {
                tag,
                class,
                href,
                kids,
            } => out.push_str(&el_md(tag, class, href, kids)?),
        }
    }
    Ok(out)
}

fn el_md(tag: &str, class: &str, href: &str, kids: &[Node]) -> Result<String, String> {
    let wrap = |m: &str| to_md(kids).map(|s| format!("{m}{s}{m}"));
    match (tag, class) {
        ("b", _) => wrap("**"),
        ("i", _) => wrap("*"),
        ("em", _) => wrap("_"),
        ("sub", _) => wrap("~"),
        ("sup", _) => wrap("^"),
        ("code", _) => text_of(kids).map(|t| format!("`{t}`")),
        ("span", "tex") => text_of(kids).map(|t| format!("${t}$")),
        ("span", "ipotesi") => Ok("[I]".into()),
        ("span", "cit") => {
            let t = text_of(kids)?;
            let keys: Vec<String> = t
                .trim_start_matches('[')
                .trim_end_matches(']')
                .split("; ")
                .map(|k| format!("@{k}"))
                .collect();
            Ok(format!("[{}]", keys.join("; ")))
        }
        ("a", "ref") => {
            let id = href
                .strip_prefix("#eq-")
                .map(|i| format!("@eq:{i}"))
                .or_else(|| href.strip_prefix("#fig-").map(|i| format!("@fig:{i}")));
            id.ok_or_else(|| "a reference can't be changed here".to_string())
        }
        ("a", _) => {
            let label = text_of(kids)?;
            Ok(format!("[{label}]({})", md_href(href)))
        }
        _ => Err("this formatting can't be kept here".into()),
    }
}

/// The reverse of the `.md` to `.html` rewrite of a local link (external links are left as they are).
fn md_href(h: &str) -> String {
    if h.starts_with("http") {
        return h.to_string();
    }
    let c: Vec<char> = h.chars().collect();
    let mut out = String::new();
    let mut i = 0;
    while i < c.len() {
        if c[i..].starts_with(&['.', 'h', 't', 'm', 'l']) && (i + 5 == c.len() || c[i + 5] == '#') {
            out.push_str(".md");
            i += 5;
        } else {
            out.push(c[i]);
            i += 1;
        }
    }
    out
}

/// The text of the nodes without their markup.
pub(crate) fn plain(nodes: &[Node]) -> String {
    nodes
        .iter()
        .map(|n| match n {
            Node::Text(t) => t.clone(),
            Node::El { kids, .. } => plain(kids),
        })
        .collect()
}
