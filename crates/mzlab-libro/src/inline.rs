//! Inline Markdown of the Libro dialect: `code`, $math$, **bold**, *italic*, _em_, ~sub~, ^sup^, [text](link.md#x), [@key; @key], @eq:id / @fig:id, [I].
//! Hand-written scanners (no regex crate: the Python reference uses lookarounds); raw HTML is never passed through, everything is escaped.
use crate::{escape, LibroError};

/// What the inline renderer collects for the page.
#[derive(Default)]
pub struct Ctx {
    pub teoria: bool,
    pub ipotesi: usize,
    pub citazioni: Vec<String>,
}

pub fn is_w(c: char) -> bool {
    c.is_alphanumeric() || c == '_'
}

fn is_key(c: char) -> bool {
    is_w(c) || matches!(c, ':' | '.' | '-')
}

type Matcher<'a> = dyn FnMut(&[char], usize) -> Option<(usize, String)> + 'a;

fn replace_all(text: &str, f: &mut Matcher) -> String {
    let c: Vec<char> = text.chars().collect();
    let mut out = String::with_capacity(text.len());
    let mut i = 0;
    while i < c.len() {
        if let Some((end, rep)) = f(&c, i) {
            out.push_str(&rep);
            i = end;
        } else {
            out.push(c[i]);
            i += 1;
        }
    }
    out
}

fn s(c: &[char]) -> String {
    c.iter().collect()
}

/// Delimited run `open content close` with the content made of chars other than the delimiter (at least one, no newline).
fn run(c: &[char], i: usize, delim: char) -> Option<usize> {
    if c.get(i) != Some(&delim) {
        return None;
    }
    let mut j = i + 1;
    while j < c.len() && c[j] != delim && c[j] != '\n' {
        j += 1;
    }
    (j > i + 1 && c.get(j) == Some(&delim)).then_some(j)
}

pub fn render(text: &str, file: &str, line: usize, ctx: &mut Ctx) -> Result<String, LibroError> {
    let mut hold: Vec<String> = Vec::new();
    // `code` and $math$ are held before escaping
    let mut t = replace_all(text, &mut |c, i| {
        run(c, i, '`').map(|j| {
            (
                j + 1,
                keep(
                    &mut hold,
                    format!("<code>{}</code>", escape(&s(&c[i + 1..j]), true)),
                ),
            )
        })
    });
    t = replace_all(&t, &mut |c, i| {
        if c[i] != '$' || (i > 0 && (c[i - 1] == '\\' || c[i - 1] == '$')) {
            return None;
        }
        run(c, i, '$').map(|j| {
            (
                j + 1,
                keep(
                    &mut hold,
                    format!(
                        "<span class=\"tex\">{}</span>",
                        escape(&s(&c[i + 1..j]), true)
                    ),
                ),
            )
        })
    });
    t = escape(&t, false);
    t = replace_all(&t, &mut |c, i| {
        cite(c, i).map(|(end, body)| (end, keep(&mut hold, cite_html(&body, ctx))))
    });
    // links may fail on an unsupported target: the first error stops the render
    let mut err: Option<LibroError> = None;
    let teoria = ctx.teoria;
    t = replace_all(&t, &mut |c, i| {
        let (end, label, href) = link(c, i)?;
        match link_html(&label, &href, file, line, teoria) {
            Ok(html) => Some((end, keep(&mut hold, html))),
            Err(x) => {
                err.get_or_insert(x);
                Some((end, String::new()))
            }
        }
    });
    if let Some(e) = err {
        return Err(e);
    }
    ctx.ipotesi += t.matches("[I]").count();
    let tag = keep(
        &mut hold,
        "<span class=\"ipotesi\" title=\"hypothesis, to be verified\">[I]</span>".to_string(),
    );
    t = t.replace("[I]", &tag);
    t = replace_all(&t, &mut |c, i| {
        reference(c, i).map(|(end, kind, id)| {
            (
                end,
                keep(
                    &mut hold,
                    format!("<a class=\"ref\" href=\"#{kind}-{id}\">{{{{{kind}:{id}}}}}</a>"),
                ),
            )
        })
    });
    t = replace_all(&t, &mut bold);
    t = replace_all(&t, &mut |c, i| emph(c, i, '*', "i"));
    t = replace_all(&t, &mut |c, i| emph(c, i, '_', "em"));
    t = replace_all(&t, &mut |c, i| tight(c, i, '~', "sub"));
    t = replace_all(&t, &mut |c, i| tight(c, i, '^', "sup"));
    Ok(restore(&t, &hold))
}

fn keep(hold: &mut Vec<String>, html: String) -> String {
    hold.push(html);
    format!("\0{}\0", hold.len() - 1)
}

/// `\x00N\x00` placeholders back to their HTML, in a single pass (the replacement text is not scanned again).
fn restore(t: &str, hold: &[String]) -> String {
    let c: Vec<char> = t.chars().collect();
    let mut out = String::with_capacity(t.len());
    let mut i = 0;
    while i < c.len() {
        if c[i] == '\0' {
            let mut j = i + 1;
            while j < c.len() && c[j].is_ascii_digit() {
                j += 1;
            }
            if j > i + 1 && c.get(j) == Some(&'\0') {
                let n: usize = s(&c[i + 1..j]).parse().unwrap_or(usize::MAX);
                if let Some(h) = hold.get(n) {
                    out.push_str(h);
                    i = j + 1;
                    continue;
                }
            }
        }
        out.push(c[i]);
        i += 1;
    }
    out
}

fn key_run(c: &[char], mut j: usize) -> Option<usize> {
    let st = j;
    while j < c.len() && is_key(c[j]) {
        j += 1;
    }
    (j > st).then_some(j)
}

fn cite(c: &[char], i: usize) -> Option<(usize, String)> {
    if c[i] != '[' || c.get(i + 1) != Some(&'@') {
        return None;
    }
    let mut j = key_run(c, i + 2)?;
    while c.get(j) == Some(&';') {
        let mut k = j + 1;
        while k < c.len() && c[k].is_whitespace() {
            k += 1;
        }
        if c.get(k) != Some(&'@') {
            return None;
        }
        j = key_run(c, k + 1)?;
    }
    (c.get(j) == Some(&']')).then(|| (j + 1, s(&c[i + 1..j])))
}

fn cite_html(body: &str, ctx: &mut Ctx) -> String {
    let keys: Vec<String> = body
        .split(';')
        .map(|k| k.trim().chars().skip(1).collect())
        .collect();
    ctx.citazioni.extend(keys.iter().cloned());
    format!(
        "<span class=\"cit\">[{}]</span>",
        escape(&keys.join("; "), true)
    )
}

fn link(c: &[char], i: usize) -> Option<(usize, String, String)> {
    if c[i] != '[' {
        return None;
    }
    let mut j = i + 1;
    while j < c.len() && c[j] != ']' && c[j] != '\n' {
        j += 1;
    }
    if j == i + 1 || c.get(j) != Some(&']') || c.get(j + 1) != Some(&'(') {
        return None;
    }
    let mut k = j + 2;
    while k < c.len() && c[k] != ')' && !c[k].is_whitespace() {
        k += 1;
    }
    (k > j + 2 && c.get(k) == Some(&')')).then(|| (k + 1, s(&c[i + 1..j]), s(&c[j + 2..k])))
}

/// `^(https?://[^\s"<>]+|#[\w-]+|[\w./-]+\.md(#[\w-]+)?)$`
fn safe_href(h: &str) -> bool {
    let c: Vec<char> = h.chars().collect();
    for p in ["https://", "http://"] {
        if let Some(rest) = h.strip_prefix(p) {
            return !rest.is_empty()
                && rest
                    .chars()
                    .all(|x| !x.is_whitespace() && !matches!(x, '"' | '<' | '>'));
        }
    }
    if c.first() == Some(&'#') {
        return c.len() > 1 && c[1..].iter().all(|&x| is_w(x) || x == '-');
    }
    let (path, anchor) = match h.find('#') {
        Some(p) => (&h[..p], Some(&h[p + 1..])),
        None => (h, None),
    };
    let path_ok = path.len() > 3
        && path.ends_with(".md")
        && path
            .chars()
            .all(|x| is_w(x) || matches!(x, '.' | '/' | '-'));
    path_ok && anchor.is_none_or(|a| !a.is_empty() && a.chars().all(|x| is_w(x) || x == '-'))
}

fn link_html(
    label: &str,
    href: &str,
    file: &str,
    line: usize,
    teoria: bool,
) -> Result<String, LibroError> {
    let href = href.replace("&amp;", "&");
    if !safe_href(&href) {
        return Err(LibroError::new(
            file,
            line,
            format!("unsupported link target {href:?}"),
        ));
    }
    // `.md` followed by `#` or the end becomes `.html`
    let c: Vec<char> = href.chars().collect();
    let mut out = String::new();
    let mut i = 0;
    while i < c.len() {
        if c[i..].starts_with(&['.', 'm', 'd']) && (i + 3 == c.len() || c[i + 3] == '#') {
            out.push_str(".html");
            i += 3;
        } else {
            out.push(c[i]);
            i += 1;
        }
    }
    let ext = if out.starts_with("http") && teoria {
        " target=\"_blank\" rel=\"noopener\""
    } else {
        ""
    };
    Ok(format!(
        "<a href=\"{}\"{ext}>{label}</a>",
        escape(&out, true)
    ))
}

fn reference(c: &[char], i: usize) -> Option<(usize, &'static str, String)> {
    if c[i] != '@' || (i > 0 && (is_w(c[i - 1]) || c[i - 1] == '[' || c[i - 1] == ';')) {
        return None;
    }
    for kind in ["eq", "fig"] {
        let k: Vec<char> = kind.chars().collect();
        if c[i + 1..].starts_with(&k) && c.get(i + 1 + k.len()) == Some(&':') {
            let st = i + 2 + k.len();
            let mut j = st;
            while j < c.len() && (is_w(c[j]) || c[j] == '-') {
                j += 1;
            }
            if j > st {
                return Some((j, kind, s(&c[st..j])));
            }
        }
    }
    None
}

fn bold(c: &[char], i: usize) -> Option<(usize, String)> {
    if !c[i..].starts_with(&['*', '*']) {
        return None;
    }
    // lazy `(.+?)`: the first closing `**` that leaves at least one content char
    let mut k = i + 3;
    while k + 1 < c.len() {
        if c[k - 1] == '\n' {
            return None;
        }
        if c[k] == '*' && c[k + 1] == '*' {
            return Some((k + 2, format!("<b>{}</b>", s(&c[i + 2..k]))));
        }
        k += 1;
    }
    None
}

/// `*x*` / `_x_` with the word-boundary and whitespace guards of the reference regexes.
fn emph(c: &[char], i: usize, d: char, tag: &str) -> Option<(usize, String)> {
    if c[i] != d || (i > 0 && (is_w(c[i - 1]) || c[i - 1] == d || (d == '*' && c[i - 1] == '*'))) {
        return None;
    }
    if c.get(i + 1).is_some_and(|x| x.is_whitespace()) {
        return None;
    }
    let mut j = i + 1;
    while j < c.len() && c[j] != d && c[j] != '\n' {
        j += 1;
    }
    if j == i + 1 || c.get(j) != Some(&d) || c[j - 1].is_whitespace() {
        return None;
    }
    if c.get(j + 1).is_some_and(|&x| is_w(x) || x == d) {
        return None;
    }
    Some((j + 1, format!("<{tag}>{}</{tag}>", s(&c[i + 1..j]))))
}

/// `~x~` / `^x^`: no whitespace and no delimiter inside.
fn tight(c: &[char], i: usize, d: char, tag: &str) -> Option<(usize, String)> {
    if c[i] != d {
        return None;
    }
    let mut j = i + 1;
    while j < c.len() && c[j] != d && !c[j].is_whitespace() {
        j += 1;
    }
    (j > i + 1 && c.get(j) == Some(&d))
        .then(|| (j + 1, format!("<{tag}>{}</{tag}>", s(&c[i + 1..j]))))
}
