//! Line scanners of the Libro block parser: the hand-written equivalents of the regexes of tools/libro (headings, list items, ids, figures, equation ends, unsafe raw HTML).
//! Used by blocks.rs and special.rs.
use crate::is_w;

/// Python `s[a:-b]` on characters.
pub(crate) fn cut(s: &str, a: usize, b: usize) -> String {
    let c: Vec<char> = s.chars().collect();
    if a + b >= c.len() {
        String::new()
    } else {
        c[a..c.len() - b].iter().collect()
    }
}

pub(crate) fn first2(s: &str) -> String {
    s.chars().take(2).collect()
}

/// `ITEM = ^([-*]|\d+\.) +(.*)$`: (marker starts with a digit, item text).
pub(crate) fn item(ln: &str) -> Option<(bool, &str)> {
    let c = ln.as_bytes();
    let mut p = match c.first()? {
        b'-' | b'*' => 1,
        d if d.is_ascii_digit() => {
            let n = c.iter().take_while(|x| x.is_ascii_digit()).count();
            if c.get(n) != Some(&b'.') {
                return None;
            }
            n + 1
        }
        _ => return None,
    };
    if c.get(p) != Some(&b' ') {
        return None;
    }
    while c.get(p) == Some(&b' ') {
        p += 1;
    }
    Some((c[0].is_ascii_digit(), &ln[p..]))
}

/// `HEAD = ^(#{1,3}) +(.+?)\s*$`: (level, text).
pub(crate) fn head(ln: &str) -> Option<(usize, String)> {
    let level = ln.bytes().take_while(|&b| b == b'#').count();
    if !(1..=3).contains(&level) {
        return None;
    }
    let rest = &ln[level..];
    let spaces = rest.bytes().take_while(|&b| b == b' ').count();
    if spaces == 0 {
        return None;
    }
    let text = rest[spaces..].trim_end();
    if !text.is_empty() {
        Some((level, text.to_string()))
    } else if spaces >= 2 && rest[spaces..].trim().is_empty() {
        Some((level, " ".to_string()))
    } else {
        None
    }
}

/// `\s*\{#([\w-]+)\}\s*$` searched in `text`: (start of the match, id).
pub(crate) fn explicit_id(text: &str) -> Option<(usize, String)> {
    let t = text.trim_end();
    let body = t.strip_suffix('}')?;
    let open = body.rfind("{#")?;
    let id = &body[open + 2..];
    if id.is_empty() || !id.chars().all(|c| is_w(c) || c == '-') {
        return None;
    }
    Some((body[..open].trim_end().len(), id.to_string()))
}

/// `\s*\{kicker: *([^}]+)\}\s*$` searched in `text`: (start of the match, kicker).
pub(crate) fn kicker(text: &str) -> Option<(usize, String)> {
    let mut from = 0;
    while let Some(p) = text[from..].find("{kicker:") {
        let at = from + p;
        let after = &text[at + 8..];
        if let Some(close) = after.find('}') {
            if after[close + 1..].trim().is_empty() && close > 0 {
                let raw = &after[..close];
                let k = raw.trim_start_matches(' ');
                let k = if k.is_empty() { " " } else { k };
                return Some((text[..at].trim_end().len(), k.to_string()));
            }
        }
        from = at + 1;
    }
    None
}

pub(crate) fn slug(s: &str) -> String {
    let mut out = String::new();
    for ch in s.to_lowercase().chars() {
        let base = match ch {
            'à' | 'á' | 'â' | 'ã' | 'ä' | 'å' => 'a',
            'ç' => 'c',
            'è' | 'é' | 'ê' | 'ë' => 'e',
            'ì' | 'í' | 'î' | 'ï' => 'i',
            'ñ' => 'n',
            'ò' | 'ó' | 'ô' | 'õ' | 'ö' => 'o',
            'ù' | 'ú' | 'û' | 'ü' => 'u',
            'ý' | 'ÿ' => 'y',
            c => c,
        };
        out.push(if base.is_ascii_lowercase() || base.is_ascii_digit() {
            base
        } else {
            '-'
        });
    }
    let mut collapsed = String::new();
    for ch in out.chars() {
        if ch == '-' && collapsed.ends_with('-') {
            continue;
        }
        collapsed.push(ch);
    }
    let t = collapsed.strip_prefix('-').unwrap_or(&collapsed);
    let t = t.strip_suffix('-').unwrap_or(t);
    t.chars().take(48).collect()
}

/// `FIG`: `![alt](src "title") {#fig:id}` -> (alt, src, title, id).
pub(crate) fn fig_line(ln: &str) -> Option<(String, String, Option<String>, String)> {
    let rest = ln.strip_prefix("![")?;
    let a = rest.find(']')?;
    let alt = &rest[..a];
    let rest = rest[a + 1..].strip_prefix('(')?;
    let n = rest
        .find(|c: char| c == ')' || c.is_whitespace() || c == '"')
        .unwrap_or(rest.len());
    if n == 0 {
        return None;
    }
    let src = &rest[..n];
    let mut rest = &rest[n..];
    let mut cap = None;
    if let Some(r) = rest.strip_prefix(" \"") {
        if let Some(q) = r.find('"') {
            cap = Some(r[..q].to_string());
            rest = &r[q + 1..];
        }
    }
    let rest = rest
        .strip_prefix(')')?
        .trim_start()
        .strip_prefix("{#fig:")?;
    let e = rest.find('}')?;
    let id = &rest[..e];
    if id.is_empty() || !id.chars().all(|c| is_w(c) || c == '-') || !rest[e + 1..].trim().is_empty()
    {
        return None;
    }
    Some((alt.to_string(), src.to_string(), cap, id.to_string()))
}

/// `EQEND.search`: start of the closing `$$ [{#eq:id}]` at the end of `s`, and the id.
pub(crate) fn eq_end(s: &[char]) -> Option<(usize, Option<String>)> {
    for p in 0..s.len().saturating_sub(1) {
        if s[p] != '$' || s[p + 1] != '$' {
            continue;
        }
        let mut k = p + 2;
        while k < s.len() && s[k].is_whitespace() {
            k += 1;
        }
        let mut id = None;
        if s[k..].starts_with(&['{', '#', 'e', 'q', ':']) {
            let st = k + 5;
            let mut j = st;
            while j < s.len() && (is_w(s[j]) || s[j] == '-') {
                j += 1;
            }
            if j > st && s.get(j) == Some(&'}') {
                id = Some(s[st..j].iter().collect());
                k = j + 1;
                while k < s.len() && s[k].is_whitespace() {
                    k += 1;
                }
            }
        }
        if k == s.len() {
            return Some((p, id));
        }
    }
    None
}

pub(crate) fn follow_id(ln: &str) -> Option<String> {
    let r = ln.strip_prefix("{#eq:")?;
    let e = r.find('}')?;
    let id = &r[..e];
    (!id.is_empty() && id.chars().all(|c| is_w(c) || c == '-') && r[e + 1..].trim().is_empty())
        .then(|| id.to_string())
}

/// `NUMCELL = ^[−+-]?\d[\d.,\s·⁰¹²³⁴⁵⁶⁷⁸⁹⁻]*$`
pub(crate) fn numcell(c: &str) -> bool {
    let mut it = c.chars().peekable();
    if it.peek().is_some_and(|x| matches!(x, '−' | '+' | '-')) {
        it.next();
    }
    if !it.next().is_some_and(|x| x.is_ascii_digit()) {
        return false;
    }
    it.all(|x| x.is_ascii_digit() || x.is_whitespace() || "., ·⁰¹²³⁴⁵⁶⁷⁸⁹⁻".contains(x))
}

pub(crate) fn unsafe_match(body: &str) -> Option<String> {
    const TAGS: [&str; 8] = [
        "script", "iframe", "object", "embed", "link", "meta", "base", "form",
    ];
    let c: Vec<char> = body
        .chars()
        .map(|x| x.to_lowercase().next().unwrap_or(x))
        .collect();
    let at = |p: usize, w: &str| {
        c[p..]
            .iter()
            .zip(w.chars())
            .filter(|(a, b)| a == &b)
            .count()
            == w.chars().count()
            && c.len() - p >= w.chars().count()
    };
    for p in 0..c.len() {
        if c[p] == '<' {
            let mut k = p + 1;
            while k < c.len() && c[k].is_whitespace() {
                k += 1;
            }
            for t in TAGS {
                let n = t.len();
                if at(k, t) && c.get(k + n).is_none_or(|&x| !is_w(x)) {
                    return Some(c[p..k + n].iter().collect::<String>().trim().to_string());
                }
            }
        }
        if c[p].is_whitespace() && c.get(p + 1) == Some(&'o') && c.get(p + 2) == Some(&'n') {
            let mut k = p + 3;
            while k < c.len() && is_w(c[k]) {
                k += 1;
            }
            if k > p + 3 {
                let mut m = k;
                while m < c.len() && c[m].is_whitespace() {
                    m += 1;
                }
                if c.get(m) == Some(&'=') {
                    return Some(c[p..=m].iter().collect::<String>().trim().to_string());
                }
            }
        }
        if at(p, "javascript:") {
            return Some("javascript:".to_string());
        }
    }
    None
}

pub(crate) fn word_dash(s: &str) -> bool {
    !s.is_empty() && s.chars().all(|c| is_w(c) || c == '-')
}

pub(crate) fn script_ref(v: &str) -> Option<(bool, &str)> {
    let (before, rest) = if let Some(r) = v.strip_prefix("script-before:") {
        (true, r)
    } else {
        (false, v.strip_prefix("script:")?)
    };
    let r = rest.trim_start_matches(' ');
    (r.len() > 3
        && r.ends_with(".js")
        && r.chars().all(|c| is_w(c) || matches!(c, '.' | '/' | '-')))
    .then_some((before, r))
}
