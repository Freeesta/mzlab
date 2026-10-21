//! The Teoria pages cut into blocks (`teoria-blocchi/1`): paragraphs under the same h2/h3 up to ~800 characters, one block per glossary entry,
//! exercises (`details.q`, `details.poe`) as the bare question plus the fingerprint of the hidden answer (never its text).
//! Read by `cerca` and by the tutor Worker; the output is canonical (sorted keys, one block per line), so the same pages give the same bytes.
use mzlab_core::budget::sha256_hex;
use mzlab_libro::{norm, slug};
use serde_json::{json, Map, Value};
use std::collections::{BTreeSet, HashMap};

/// Target size of a block (characters) and the hard ceiling for a single piece that cannot be cut at a sentence.
const TARGET: usize = 800;
const CEILING: usize = 1100;
/// Words per shingle of the hidden answer.
const SHINGLE: usize = 5;

/// Elements whose content is never text for the tutor (scripts, drawings, interactive figures, menus).
fn skipped(name: &str, classes: &str) -> bool {
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

fn is_void(name: &str) -> bool {
    matches!(
        name,
        "br" | "img" | "meta" | "link" | "hr" | "input" | "wbr"
    )
}

/// Tags that end the paragraph being collected.
fn breaks(name: &str) -> bool {
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

struct El {
    name: String,
    classes: String,
    id: Option<String>,
}

enum Tok<'a> {
    Open(String, &'a str),
    Close(String),
    Text(&'a str),
}

/// Value of `name="..."` in the attribute text of a tag.
fn attr(attrs: &str, name: &str) -> Option<String> {
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

fn tokens(html: &str) -> Vec<Tok<'_>> {
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

fn decode(text: &str, out: &mut String) {
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

fn squash(s: &str) -> String {
    s.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// Where the text being read goes.
#[derive(PartialEq, Clone, Copy)]
enum Sink {
    Body,
    Kicker,
    Heading,
    Title,
    Question,
    Hidden,
    Term,
}

#[derive(Default)]
struct Page {
    file: String,
    chapter: String,
    title: String,
    blocks: Vec<Block>,
    counters: HashMap<String, usize>,
    // section being read
    sect_title: String,
    sect_anchor: String,
    paras: Vec<(String, bool)>,
}

struct Block {
    id: String,
    anchor: String,
    title: String,
    kind: &'static str,
    text: String,
    hidden: Option<String>,
}

impl Page {
    fn push(
        &mut self,
        anchor: &str,
        title: &str,
        kind: &'static str,
        text: String,
        hidden: Option<String>,
    ) {
        let k = self.counters.entry(anchor.to_string()).or_insert(0);
        *k += 1;
        self.blocks.push(Block {
            id: format!("{}#{}.{}", self.file, anchor, k),
            anchor: anchor.to_string(),
            title: title.to_string(),
            kind,
            text,
            hidden,
        });
    }

    /// Turns the paragraphs collected under the current section into blocks.
    fn emit_section(&mut self) {
        let paras = std::mem::take(&mut self.paras);
        let (anchor, title) = (self.sect_anchor.clone(), self.sect_title.clone());
        let mut cur = String::new();
        let mut fig = false;
        let mut done: Vec<(String, bool)> = Vec::new();
        for (p, f) in paras {
            for piece in cut(&p) {
                if !cur.is_empty() && cur.chars().count() + 1 + piece.chars().count() > TARGET {
                    done.push((std::mem::take(&mut cur), fig));
                    fig = false;
                }
                if !cur.is_empty() {
                    cur.push('\n');
                }
                cur.push_str(&piece);
                fig |= f;
            }
        }
        if !cur.is_empty() {
            done.push((cur, fig));
        }
        for (text, fig) in done {
            self.push(
                &anchor,
                &title,
                if fig { "figura" } else { "testo" },
                text,
                None,
            );
        }
    }
}

/// A paragraph cut into pieces of at most ~800 characters, at sentence ends when it can.
fn cut(p: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut rest: Vec<char> = p.chars().collect();
    while rest.len() > TARGET {
        let sentence = (0..TARGET)
            .rev()
            .find(|&i| matches!(rest[i], '.' | '!' | '?' | ';') && rest.get(i + 1) == Some(&' '));
        let at = sentence
            .map(|i| i + 1)
            .or_else(|| (TARGET..CEILING.min(rest.len())).find(|&i| rest[i] == ' '))
            .unwrap_or(CEILING.min(rest.len()));
        let head: String = rest[..at].iter().collect();
        out.push(head.trim().to_string());
        rest = rest[at..]
            .iter()
            .copied()
            .skip_while(|c| c.is_whitespace())
            .collect();
    }
    let tail: String = rest.iter().collect();
    if !tail.trim().is_empty() {
        out.push(tail.trim().to_string());
    }
    out
}

/// `sha` of a text: first 12 hex digits of the SHA-256 of its normalised form.
fn sha12(text: &str) -> String {
    sha256_hex(norm(text).as_bytes())[..12].to_string()
}

/// Hashed 5-word shingles of a text (6 hex digits each, sorted, unique): enough to measure how much of a hidden answer a reply copies, without the words.
pub fn shingles(text: &str) -> Vec<String> {
    let w = crate::words(text);
    let set: BTreeSet<String> = w
        .windows(SHINGLE)
        .map(|g| sha256_hex(g.join(" ").as_bytes())[..6].to_string())
        .collect();
    set.into_iter().collect()
}

fn read_page(file: &str, html: &str) -> Option<Page> {
    // the Pratica pages are exercises with live figures and `index` is the menu: neither is text to cite
    if html.contains("http-equiv=\"refresh\"")
        || file.starts_with("pratica")
        || file == "index.html"
    {
        return None;
    }
    let body = match html.find("<main") {
        Some(m) => &html[m..],
        None => html,
    };
    let mut pg = Page {
        file: file.trim_end_matches(".html").to_string(),
        ..Page::default()
    };
    let mut stack: Vec<El> = Vec::new();
    // depth of the stack at which a skipped element opened
    let mut skip: Option<usize> = None;
    let mut cur = String::new();
    let mut buf = String::new();
    let (mut sink, mut sink_depth) = (Sink::Body, 0usize);
    let mut question = String::new();
    let mut hidden = String::new();
    let mut in_exercise: Option<usize> = None;
    let mut in_figdesc: Option<usize> = None;
    let mut caption_last = false;
    let mut label_open = false;
    let mut term_anchor = String::new();
    let mut term = String::new();
    let mut in_dd = false;
    let mut kicker = String::new();

    let flush =
        |pg: &mut Page, cur: &mut String, fig: bool, caption: bool, caption_last: &mut bool| {
            let t = squash(cur);
            cur.clear();
            if !t.is_empty() {
                pg.paras.push((t, fig));
                *caption_last = caption;
            }
        };

    for tok in tokens(body) {
        match tok {
            Tok::Text(t) => {
                if skip.is_some() {
                    continue;
                }
                let target = match sink {
                    Sink::Body => &mut cur,
                    Sink::Kicker | Sink::Heading | Sink::Title | Sink::Term => &mut buf,
                    Sink::Question => &mut question,
                    Sink::Hidden => &mut hidden,
                };
                decode(t, target);
            }
            Tok::Open(name, attrs) => {
                if skip.is_some() {
                    if !is_void(&name) {
                        stack.push(El {
                            name,
                            classes: String::new(),
                            id: None,
                        });
                    }
                    continue;
                }
                let classes = attr(attrs, "class").unwrap_or_default();
                let has = |c: &str| classes.split_whitespace().any(|x| x == c);
                if skipped(&name, &classes) && !is_void(&name) {
                    skip = Some(stack.len());
                    stack.push(El {
                        name,
                        classes,
                        id: None,
                    });
                    continue;
                }
                if is_void(&name) {
                    if name == "br" && sink == Sink::Body {
                        cur.push(' ');
                    }
                    continue;
                }
                let id = attr(attrs, "id");
                let keep_label = std::mem::take(&mut label_open);
                let depth = stack.len();
                let is_exercise = name == "details" && (has("q") || has("poe"));
                let is_figdesc = name == "details" && has("figdesc");
                if is_exercise && in_exercise.is_none() {
                    flush(&mut pg, &mut cur, false, false, &mut caption_last);
                    pg.emit_section();
                    in_exercise = Some(depth);
                    question.clear();
                    hidden.clear();
                    sink = Sink::Hidden;
                    sink_depth = depth;
                } else if is_figdesc && in_figdesc.is_none() {
                    // the description joins the figure: no break before it, its summary is not text
                    in_figdesc = Some(depth);
                    match pg.paras.pop().filter(|_| caption_last) {
                        Some(last) => {
                            cur = last.0;
                            cur.push(' ');
                        }
                        None => flush(&mut pg, &mut cur, false, false, &mut caption_last),
                    }
                } else if let Some(ex) = in_exercise {
                    if name == "summary" && depth == ex + 1 {
                        sink = Sink::Question;
                        sink_depth = depth;
                    } else if breaks(&name) && sink == Sink::Hidden {
                        hidden.push(' ');
                    }
                } else if name == "summary" && in_figdesc.is_some() {
                    skip = Some(depth);
                } else if name == "h1" {
                    flush(&mut pg, &mut cur, false, false, &mut caption_last);
                    sink = Sink::Title;
                    sink_depth = depth;
                    buf.clear();
                } else if name == "h2" || name == "h3" {
                    flush(&mut pg, &mut cur, false, false, &mut caption_last);
                    pg.emit_section();
                    sink = Sink::Heading;
                    sink_depth = depth;
                    buf.clear();
                } else if name == "div" && has("kicker") {
                    sink = Sink::Kicker;
                    sink_depth = depth;
                    buf.clear();
                } else if name == "dt" {
                    flush(&mut pg, &mut cur, false, false, &mut caption_last);
                    pg.emit_section();
                    sink = Sink::Term;
                    sink_depth = depth;
                    buf.clear();
                    term_anchor = id.clone().unwrap_or_default();
                } else if name == "dd" && !term_anchor.is_empty() {
                    in_dd = true;
                    cur.clear();
                } else if in_figdesc.is_some() {
                    // paragraphs inside the description stay in one
                    if matches!(name.as_str(), "p" | "li") && !cur.is_empty() {
                        cur.push(' ');
                    }
                } else if breaks(&name) && !(name == "p" && keep_label) && !in_dd {
                    flush(&mut pg, &mut cur, false, false, &mut caption_last);
                }
                if (name == "td" || name == "th") && !cur.is_empty() && sink == Sink::Body {
                    cur.push_str(" | ");
                }
                if name == "sup"
                    && sink == Sink::Body
                    && cur.ends_with(|c: char| c.is_ascii_digit())
                {
                    cur.push('^');
                }
                if name == "span" && has("frac") {
                    cur.push(' ');
                }
                stack.push(El { name, classes, id });
            }
            Tok::Close(name) => {
                let Some(pos) = stack.iter().rposition(|e| e.name == name) else {
                    continue;
                };
                let el = stack.remove(pos);
                stack.truncate(pos);
                if let Some(d) = skip {
                    if pos > d {
                        continue;
                    }
                    skip = None;
                    continue;
                }
                if sink != Sink::Body
                    && pos == sink_depth
                    && !(sink == Sink::Hidden && name == "summary")
                {
                    match sink {
                        Sink::Kicker => kicker = squash(&buf),
                        Sink::Title => {
                            pg.title = squash(&buf);
                            pg.sect_title = pg.title.clone();
                        }
                        Sink::Heading => {
                            pg.sect_title = squash(&buf);
                            pg.sect_anchor = el.id.clone().unwrap_or_else(|| slug(&pg.sect_title));
                        }
                        Sink::Term => term = squash(&buf),
                        Sink::Body | Sink::Question | Sink::Hidden => {}
                    }
                    if sink == Sink::Question {
                        sink = Sink::Hidden;
                        sink_depth = in_exercise.unwrap_or(0);
                    } else if sink != Sink::Hidden {
                        sink = Sink::Body;
                    }
                    buf.clear();
                }
                if in_exercise == Some(pos) {
                    let q = squash(&question);
                    if !q.is_empty() {
                        let h = squash(&hidden);
                        let (anchor, title) = (pg.sect_anchor.clone(), pg.sect_title.clone());
                        pg.push(&anchor, &title, "esercizio", q, Some(h));
                    }
                    in_exercise = None;
                    sink = Sink::Body;
                    continue;
                }
                if in_figdesc == Some(pos) {
                    in_figdesc = None;
                    // the figure's text is closed here so the next paragraph does not join it
                    flush(&mut pg, &mut cur, true, false, &mut caption_last);
                    continue;
                }
                if name == "dd" && in_dd {
                    in_dd = false;
                    let def = squash(&cur);
                    cur.clear();
                    let text = format!("{term}: {def}");
                    for piece in cut(&text) {
                        let anchor = term_anchor.clone();
                        pg.push(&anchor, &term.clone(), "glossario", piece, None);
                    }
                    term_anchor.clear();
                    continue;
                }
                if name == "b"
                    && stack.last().is_some_and(|p| {
                        p.name == "div" && p.classes.split_whitespace().any(|c| c == "box")
                    })
                {
                    cur.push_str(". ");
                    label_open = true;
                } else if name == "figcaption" {
                    flush(&mut pg, &mut cur, false, true, &mut caption_last);
                } else if breaks(&name) && in_figdesc.is_none() && !in_dd && sink == Sink::Body {
                    flush(&mut pg, &mut cur, false, false, &mut caption_last);
                }
            }
        }
    }
    flush(&mut pg, &mut cur, false, false, &mut caption_last);
    pg.emit_section();
    pg.chapter = if kicker.is_empty() {
        pg.title.clone()
    } else {
        kicker
    };
    Some(pg)
}

fn block_json(file: &str, chapter: &str, b: &Block) -> Value {
    let mut m = Map::new();
    m.insert("anchor".into(), json!(b.anchor));
    m.insert("chapter".into(), json!(chapter));
    m.insert("file".into(), json!(file));
    if let Some(h) = &b.hidden {
        m.insert("hidden_sha".into(), json!(sha12(h)));
        m.insert("hidden_shingles".into(), json!(shingles(h)));
    }
    m.insert("id".into(), json!(b.id));
    m.insert("kind".into(), json!(b.kind));
    m.insert("sha".into(), json!(sha12(&b.text)));
    m.insert("text".into(), json!(b.text));
    m.insert("title".into(), json!(b.title));
    Value::Object(m)
}

/// The blocks of the pages `(file name, html)` as a `teoria-blocchi/1` value; pages that only redirect are skipped. Pages are taken in name order.
pub fn build(pages: &[(String, String)]) -> Result<Value, String> {
    let mut sorted: Vec<&(String, String)> = pages.iter().collect();
    sorted.sort_by(|a, b| a.0.cmp(&b.0));
    let mut blocks = Vec::new();
    let mut chapters = Map::new();
    let mut seen = BTreeSet::new();
    for (name, html) in sorted {
        let Some(pg) = read_page(name, html) else {
            continue;
        };
        let file = name.trim_end_matches(".html");
        chapters.insert(file.to_string(), json!(pg.title));
        for b in &pg.blocks {
            if !seen.insert(b.id.clone()) {
                return Err(format!("duplicate block id {}", b.id));
            }
            blocks.push(block_json(file, &pg.chapter, b));
        }
    }
    let mut top = Map::new();
    top.insert("blocks".into(), Value::Array(blocks.clone()));
    top.insert("chapters".into(), Value::Object(chapters));
    top.insert("n".into(), json!(blocks.len()));
    top.insert("schema".into(), json!(crate::SCHEMA));
    Ok(Value::Object(top))
}

/// Canonical text of the value `build` returns: keys in alphabetical order, one block per line, a final newline.
pub fn to_text(v: &Value) -> String {
    let blocks: Vec<String> = v["blocks"]
        .as_array()
        .map_or_else(Vec::new, |a| a.iter().map(Value::to_string).collect());
    format!(
        "{{\"blocks\":[\n{}\n],\"chapters\":{},\"n\":{},\"schema\":{}}}\n",
        blocks.join(",\n"),
        v["chapters"],
        v["n"],
        v["schema"]
    )
}
