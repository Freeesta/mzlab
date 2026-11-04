//! A tolerant reader of XHTML, EPA HTML and eCFR XML for the normative builder: tags into a tree, text with block breaks, tables as rows of cells.
//! Names are lowercased; `td`, `th`, `tr`, `p` and `li` close themselves when a sibling starts; entities are decoded.

impl Node {
    pub fn attr(&self, key: &str) -> Option<&str> {
        self.attrs
            .iter()
            .find(|(k, _)| k == key)
            .map(|(_, v)| v.as_str())
    }

    /// Every descendant element called `name`, in document order (not inside a match).
    pub fn find_all<'a>(&'a self, name: &str) -> Vec<&'a Node> {
        let mut out = Vec::new();
        self.collect(name, &mut out);
        out
    }

    fn collect<'a>(&'a self, name: &str, out: &mut Vec<&'a Node>) {
        for k in &self.kids {
            if k.name == name {
                out.push(k);
            }
            k.collect(name, out);
        }
    }

    /// The text, with a line break after block elements; `skip` names elements left out with their content.
    pub fn lines(&self, skip: &[&str]) -> Vec<String> {
        let mut buf = String::new();
        self.write_text(skip, &mut buf);
        buf.split('\n')
            .map(|l| l.split_whitespace().collect::<Vec<_>>().join(" "))
            .filter(|l| !l.is_empty())
            .collect()
    }

    fn write_text(&self, skip: &[&str], buf: &mut String) {
        if self.name.is_empty() {
            buf.push_str(&self.text);
            return;
        }
        // a note mark: a link to an anchor of the same page
        if skip.contains(&self.name.as_str())
            || (self.name == "a" && self.attr("href").is_some_and(|h| h.starts_with('#')))
        {
            return;
        }
        let block = BLOCK.contains(&self.name.as_str());
        if block {
            buf.push('\n');
        }
        for k in &self.kids {
            k.write_text(skip, buf);
        }
        if block {
            buf.push('\n');
        }
    }

    /// The text on one line (blocks joined by a space).
    pub fn flat(&self) -> String {
        self.lines(&[]).join(" ")
    }
}

impl Cell {
    pub fn text(&self) -> String {
        self.lines.join(" ")
    }
}

/// One element (or a run of text when `name` is empty).
#[derive(Debug, Default, Clone)]
pub struct Node {
    pub name: String,
    pub text: String,
    pub attrs: Vec<(String, String)>,
    pub kids: Vec<Node>,
}

const VOID: [&str; 8] = ["br", "hr", "img", "col", "meta", "link", "input", "wbr"];
const BLOCK: [&str; 14] = [
    "p", "div", "br", "li", "tr", "table", "h1", "h2", "h3", "h4", "caption", "fp-1", "fp-2", "ul",
];
// Elements dropped with their content: scripts and styles.
const SKIP: [&str; 2] = ["script", "style"];

/// The text of an entity body (between `&` and `;`), if known.
fn entity(body: &str) -> Option<String> {
    if let Some(n) = body.strip_prefix('#') {
        let code = match n.strip_prefix(['x', 'X']) {
            Some(h) => u32::from_str_radix(h, 16).ok()?,
            None => n.parse().ok()?,
        };
        return char::from_u32(code).map(String::from);
    }
    let c = match body {
        "amp" => '&',
        "lt" => '<',
        "gt" => '>',
        "quot" => '"',
        "apos" => '\'',
        "nbsp" => ' ',
        "mu" | "micro" => 'µ',
        "ndash" => '–',
        "mdash" => '—',
        "deg" => '°',
        "beta" => 'β',
        "alpha" => 'α',
        "gamma" => 'γ',
        "delta" => 'δ',
        "ge" => '≥',
        "le" => '≤',
        _ => return None,
    };
    Some(c.to_string())
}

pub fn decode(s: &str) -> String {
    if !s.contains('&') {
        return s.to_string();
    }
    let mut out = String::with_capacity(s.len());
    let mut rest = s;
    while let Some(i) = rest.find('&') {
        out.push_str(&rest[..i]);
        let tail = &rest[i + 1..];
        match tail
            .find(';')
            .filter(|&j| j <= 10)
            .and_then(|j| entity(&tail[..j]).map(|e| (j, e)))
        {
            Some((j, e)) => {
                out.push_str(&e);
                rest = &tail[j + 1..];
            }
            None => {
                out.push('&');
                rest = tail;
            }
        }
    }
    out.push_str(rest);
    out
}

fn attrs_of(s: &str) -> Vec<(String, String)> {
    let mut out = Vec::new();
    let b = s.as_bytes();
    let mut i = 0;
    while i < b.len() {
        while i < b.len() && (b[i].is_ascii_whitespace() || b[i] == b'/') {
            i += 1;
        }
        let k0 = i;
        while i < b.len() && !b[i].is_ascii_whitespace() && b[i] != b'=' && b[i] != b'/' {
            i += 1;
        }
        let key = s[k0..i].to_ascii_lowercase();
        while i < b.len() && b[i].is_ascii_whitespace() {
            i += 1;
        }
        let mut val = String::new();
        if i < b.len() && b[i] == b'=' {
            i += 1;
            while i < b.len() && b[i].is_ascii_whitespace() {
                i += 1;
            }
            if i < b.len() && (b[i] == b'"' || b[i] == b'\'') {
                let q = b[i];
                i += 1;
                let v0 = i;
                while i < b.len() && b[i] != q {
                    i += 1;
                }
                val = decode(&s[v0..i]);
                i += 1;
            } else {
                let v0 = i;
                while i < b.len() && !b[i].is_ascii_whitespace() {
                    i += 1;
                }
                val = decode(&s[v0..i]);
            }
        }
        if !key.is_empty() {
            out.push((key, val));
        }
    }
    out
}

/// Whether starting `name` closes the open element `open` (HTML optional end tags).
fn closes(name: &str, open: &str) -> bool {
    match name {
        "td" | "th" => matches!(open, "td" | "th"),
        "tr" => matches!(open, "td" | "th" | "tr"),
        "tbody" | "thead" | "tfoot" => {
            matches!(open, "td" | "th" | "tr" | "tbody" | "thead" | "tfoot")
        }
        "p" | "li" => open == name,
        _ => false,
    }
}

fn push_text(stack: &mut [Node], text: String) {
    stack.last_mut().expect("root").kids.push(Node {
        text,
        ..Node::default()
    });
}

fn flush(stack: &mut [Node], src: &str, from: usize, to: usize) {
    if to > from {
        let t = decode(&src[from..to]);
        if !t.is_empty() {
            push_text(stack, t);
        }
    }
}

fn pop(stack: &mut Vec<Node>) {
    let n = stack.pop().expect("open element");
    stack.last_mut().expect("root").kids.push(n);
}

/// An end tag closes the nearest open element of that name (and what is open inside it).
fn end_tag(stack: &mut Vec<Node>, close: &str) {
    let name = close.trim().to_ascii_lowercase();
    if let Some(pos) = stack.iter().rposition(|n| n.name == name)
        && pos > 0
    {
        while stack.len() > pos {
            pop(stack);
        }
    }
}

/// A start tag (the text between `<` and `>`); returns where parsing goes on when the element is raw text to skip (`script`, `style`).
fn start_tag(stack: &mut Vec<Node>, src: &str, inner: &str, at: usize) -> Option<usize> {
    let self_close = inner.ends_with('/');
    let inner = inner.trim_end_matches('/');
    let (name, attr_text) = match inner.find(|c: char| c.is_ascii_whitespace()) {
        Some(j) => (&inner[..j], &inner[j..]),
        None => (inner, ""),
    };
    let name = name.to_ascii_lowercase();
    while stack.len() > 1 && closes(&name, &stack.last().expect("top").name) {
        pop(stack);
    }
    if SKIP.contains(&name.as_str()) && !self_close {
        let close = format!("</{name}");
        let lower = src[at..].to_ascii_lowercase();
        let i = at + lower.find(&close).unwrap_or(lower.len());
        return Some(i + src[i..].find('>').map_or(src.len() - i, |j| j + 1));
    }
    let node = Node {
        name: name.clone(),
        attrs: attrs_of(attr_text),
        ..Node::default()
    };
    if self_close || VOID.contains(&name.as_str()) {
        stack.last_mut().expect("root").kids.push(node);
    } else {
        stack.push(node);
    }
    None
}

/// Parses `src` into a root node holding the document.
pub fn parse(src: &str) -> Node {
    let mut stack: Vec<Node> = vec![Node::default()];
    let b = src.as_bytes();
    let mut i = 0;
    let mut text_from = 0;
    while i < b.len() {
        if b[i] != b'<' {
            i += 1;
            continue;
        }
        let rest = &src[i..];
        if rest.starts_with("<!--") {
            flush(&mut stack, src, text_from, i);
            i += rest.find("-->").map_or(rest.len(), |j| j + 3);
            text_from = i;
            continue;
        }
        if rest.starts_with("<![CDATA[") {
            flush(&mut stack, src, text_from, i);
            let end = rest.find("]]>").unwrap_or(rest.len());
            push_text(&mut stack, rest[9..end.max(9)].to_string());
            i += end + 3;
            text_from = i.min(b.len());
            continue;
        }
        // "<" not followed by a name, "/" or "!" or "?" is text.
        let c1 = rest.as_bytes().get(1).copied().unwrap_or(b' ');
        if !(c1.is_ascii_alphabetic() || c1 == b'/' || c1 == b'!' || c1 == b'?') {
            i += 1;
            continue;
        }
        flush(&mut stack, src, text_from, i);
        let Some(end) = rest.find('>') else {
            i = b.len();
            text_from = i;
            break;
        };
        let inner = &rest[1..end];
        i += end + 1;
        text_from = i;
        if let Some(close) = inner.strip_prefix('/') {
            end_tag(&mut stack, close);
        } else if !inner.starts_with(['!', '?'])
            && let Some(next) = start_tag(&mut stack, src, inner, i)
        {
            i = next;
            text_from = i;
        }
    }
    flush(&mut stack, src, text_from, b.len());
    while stack.len() > 1 {
        pop(&mut stack);
    }
    stack.pop().expect("root")
}

/// One table cell: its lines of text, its column span and whether it is carried down from a cell above by `rowspan`.
#[derive(Debug, Clone, Default)]
pub struct Cell {
    pub lines: Vec<String>,
    pub span: usize,
    pub carried: bool,
}

pub type Row = Vec<Cell>;

/// The rows of a table element (its own rows, not those of nested tables), header rows first as in the document. A cell with `rowspan` appears again, marked
/// `carried`, in the rows below it.
pub fn rows(table: &Node) -> Vec<Row> {
    fn walk<'a>(n: &'a Node, out: &mut Vec<&'a Node>) {
        for k in &n.kids {
            match k.name.as_str() {
                "tr" => out.push(k),
                "thead" | "tbody" | "tfoot" => walk(k, out),
                _ => {}
            }
        }
    }
    let mut trs = Vec::new();
    walk(table, &mut trs);
    // (column, rows left, cell) of the cells that still span down
    let mut pending: Vec<(usize, usize, Cell)> = Vec::new();
    let mut out = Vec::new();
    for tr in trs {
        let mut row: Row = Vec::new();
        let mut col = 0;
        let mut cells = tr
            .kids
            .iter()
            .filter(|c| c.name == "td" || c.name == "th")
            .peekable();
        loop {
            if let Some(i) = pending.iter().position(|(c, _, _)| *c == col) {
                let (_, left, cell) = &mut pending[i];
                let mut carried = cell.clone();
                carried.carried = true;
                col += carried.span.max(1);
                row.push(carried);
                *left -= 1;
                if *left == 0 {
                    pending.remove(i);
                }
                continue;
            }
            let Some(c) = cells.next() else { break };
            let num = |k: &str| c.attr(k).and_then(|s| s.parse::<usize>().ok()).unwrap_or(1);
            let cell = Cell {
                lines: c.lines(&["su", "ftref", "sup"]),
                span: num("colspan"),
                carried: false,
            };
            if num("rowspan") > 1 {
                pending.push((col, num("rowspan") - 1, cell.clone()));
            }
            col += cell.span.max(1);
            row.push(cell);
        }
        out.push(row);
    }
    out
}

/// Every table of the document with its rows.
pub fn tables(root: &Node) -> Vec<Vec<Row>> {
    root.find_all("table").into_iter().map(rows).collect()
}

/// A paragraph outside the data tables, or a data table, in document order.
pub enum Item<'a> {
    Para(String),
    Table(&'a Node),
}

/// The document as a sequence of paragraphs and data tables. A table with `border="0"` and no `oj-table` class only lays out the page (consolidated EUR-Lex
/// texts put titles and amendment marks in such tables): the paragraphs inside it are kept, the table is not.
pub fn outline(root: &Node) -> Vec<Item<'_>> {
    fn walk<'a>(n: &'a Node, out: &mut Vec<Item<'a>>) {
        for k in &n.kids {
            match k.name.as_str() {
                "table"
                    if k.attr("border") != Some("0")
                        || k.attr("class").is_some_and(|c| c.contains("oj-table")) =>
                {
                    out.push(Item::Table(k))
                }
                "p" | "h1" | "h2" | "h3" | "h4" | "fp-1" | "fp-2" | "head" => {
                    let t = k.flat();
                    if !t.is_empty() {
                        out.push(Item::Para(t));
                    }
                }
                "" => {}
                _ => walk(k, out),
            }
        }
    }
    let mut out = Vec::new();
    walk(root, &mut out);
    out
}
