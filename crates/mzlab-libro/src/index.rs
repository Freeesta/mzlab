//! The search index of the Teoria (`indice-ricerca.js`), rebuilt for ONE chapter: port of the per-chapter part of tools/genera_indice_teoria.py
//! (titles of h2/h3, of the boxes `.box.gym` and of the simulators, with the normalised search string and the synonyms). Used by the Teoria editor
//! when a heading is retitled; `tests/index.rs` proves it gives back the committed index for every chapter.

/// Words that make an entry searchable by another name: if the normalised text of an entry contains the key, the listed words are added.
const SINONIMI: &[(&str, &[&str])] = &[
    ("esi", &["elettrospray", "electrospray"]),
    ("elettrospray", &["esi", "electrospray"]),
    ("perdit", &["neutral loss", "perdita neutra"]),
    ("addott", &["adduct", "m+h", "m+na"]),
    ("ms2", &["msms", "ms/ms", "tandem", "frammentazione"]),
    ("msms", &["ms2", "tandem"]),
    ("frammentazione", &["frammentare", "fragmentation", "cid"]),
    ("alta risoluzione", &["hrms", "orbitrap", "tof"]),
    ("triplo quadrupolo", &["qqq", "qtrap", "mrm"]),
    ("mrm", &["transizione", "srm"]),
    ("ione molecolare", &["m+.", "molecular ion"]),
    ("isotop", &["cloro", "bromo", "a+2"]),
    ("formula", &["massa esatta", "ppm"]),
    ("ei", &["ionizzazione elettronica", "70 ev"]),
    ("ionizzazione elettronica", &["ei"]),
    ("ionizzazione chimica", &["ci"]),
    ("azoto", &["parita", "nitrogen rule"]),
    ("gradiente", &["fase mobile", "eluizione"]),
    (
        "cromatogramma",
        &["xic", "tic", "rt", "tempo di ritenzione"],
    ),
    ("xic", &["cromatogramma estratto"]),
    ("tic", &["cromatogramma totale"]),
];

/// The synonym table of the search index: normalised key (matched at the start of a word) -> words that make an entry findable by another name.
pub fn sinonimi() -> &'static [(&'static str, &'static [&'static str])] {
    SINONIMI
}

/// The letter a precomposed Latin letter has after NFD (and its combining marks dropped); other characters are unchanged.
fn base_letter(c: char) -> char {
    const FROM: &str =
        "àáâãäåçèéêëìíîïñòóôõöùúûüýÿāăąćĉċčďēĕėęěĝğġģĥĩīĭįĵķĺļľńņňōŏőŕŗřśŝşšţťũūŭůűųŵŷźżž";
    const TO: &str =
        "aaaaaaceeeeiiiinooooouuuuyyaaaccccdeeeeegggghiiiijklllnnnooorrrssssttuuuuuuwyzzz";
    match FROM.chars().position(|f| f == c) {
        Some(p) => TO.chars().nth(p).unwrap_or(c),
        None => c,
    }
}

fn lower_base(s: &str) -> String {
    s.to_lowercase()
        .chars()
        .filter(|c| !('\u{300}'..='\u{36f}').contains(c))
        .map(base_letter)
        .collect()
}

/// Python `norm`: lower case, accents dropped, white space collapsed.
pub fn norm(s: &str) -> String {
    lower_base(s)
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

/// The id `teoria.js` gives a heading at run time.
pub fn slug(s: &str) -> String {
    let mut out = String::new();
    for ch in lower_base(s).chars() {
        if ch.is_ascii_lowercase() || ch.is_ascii_digit() {
            out.push(ch);
        } else if !out.ends_with('-') {
            out.push('-');
        }
    }
    out.trim_matches('-').chars().take(48).collect()
}

fn entity(name: &str) -> Option<String> {
    if let Some(n) = name.strip_prefix('#') {
        let code = match n.strip_prefix(['x', 'X']) {
            Some(h) => u32::from_str_radix(h, 16).ok()?,
            None => n.parse().ok()?,
        };
        return char::from_u32(code).map(String::from);
    }
    let c = match name {
        "amp" => '&',
        "lt" => '<',
        "gt" => '>',
        "quot" => '"',
        "apos" => '\'',
        "nbsp" => '\u{a0}',
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
    };
    Some(c.to_string())
}

/// Python `plain`: tags removed, entities decoded, white space collapsed. An entity that is not known is an error (the index would differ).
fn plain(h: &str) -> Result<String, String> {
    let mut no_tags = String::new();
    let mut in_tag = false;
    for ch in h.chars() {
        match ch {
            '<' => in_tag = true,
            '>' if in_tag => in_tag = false,
            _ if !in_tag => no_tags.push(ch),
            _ => {}
        }
    }
    let mut out = String::new();
    let mut rest = no_tags.as_str();
    while let Some(a) = rest.find('&') {
        out.push_str(&rest[..a]);
        rest = &rest[a..];
        match rest.find(';').filter(|&e| e <= 32) {
            Some(e) => {
                out.push_str(
                    &entity(&rest[1..e])
                        .ok_or_else(|| format!("unsupported entity {}", &rest[..=e]))?,
                );
                rest = &rest[e + 1..];
            }
            None => {
                out.push('&');
                rest = &rest[1..];
            }
        }
    }
    out.push_str(rest);
    Ok(out.split_whitespace().collect::<Vec<_>>().join(" "))
}

/// One row of the index: `[text, type, url, chapter, normalised string + synonyms]` as a JSON array (Python `json.dumps(ensure_ascii=False)`).
fn entry(text: &str, kind: &str, url: &str, cap: &str) -> String {
    let key = norm(text);
    let mut syn: Vec<&str> = SINONIMI
        .iter()
        .filter(|(k, _)| has_word_start(&key, k))
        .flat_map(|(_, ws)| ws.iter().copied())
        .filter(|w| *w != key)
        .collect();
    syn.sort_unstable();
    syn.dedup();
    let mut all = vec![key.as_str()];
    all.extend(syn);
    let norm_all = all.join(" ");
    let row = [text, kind, url, cap, norm_all.trim()];
    format!(
        "[{}]",
        row.iter()
            .map(|s| json_str(s))
            .collect::<Vec<_>>()
            .join(", ")
    )
}

/// `re.search(r"(^| )" + k, key)`
fn has_word_start(key: &str, k: &str) -> bool {
    key.starts_with(k) || key.contains(&format!(" {k}"))
}

fn json_str(s: &str) -> String {
    let mut o = String::from("\"");
    for c in s.chars() {
        match c {
            '"' => o.push_str("\\\""),
            '\\' => o.push_str("\\\\"),
            '\n' => o.push_str("\\n"),
            '\r' => o.push_str("\\r"),
            '\t' => o.push_str("\\t"),
            '\u{8}' => o.push_str("\\b"),
            '\u{c}' => o.push_str("\\f"),
            c if (c as u32) < 0x20 => o.push_str(&format!("\\u{:04x}", c as u32)),
            c => o.push(c),
        }
    }
    o.push('"');
    o
}

/// The rows a chapter page adds after its own title: sections (h2/h3), simulators and the titles of the `gym` boxes, in document order.
fn section_rows(html: &str, file: &str, cap: &str) -> Result<Vec<String>, String> {
    let mut rows = Vec::new();
    let mut last = String::new();
    let mut at = 0;
    while let Some(p) = html[at..].find('<') {
        let start = at + p;
        let tail = &html[start..];
        if let Some((end, id, inner)) = heading_at(tail) {
            let t = plain(inner)?;
            at = start + end;
            if t.is_empty() {
                continue;
            }
            last = id.map(str::to_string).unwrap_or_else(|| slug(&t));
            rows.push(entry(&t, "sezione", &format!("{file}#{last}"), cap));
        } else if let Some((end, classes, inner)) = box_at(tail) {
            let t = plain(inner)?;
            at = start + end;
            if classes.split_whitespace().any(|c| c == "gym") && !t.is_empty() {
                let url = if last.is_empty() {
                    file.to_string()
                } else {
                    format!("{file}#{last}")
                };
                rows.push(entry(&t, "esercizio", &url, cap));
            }
        } else if let Some((end, id, inner)) = sim_at(tail) {
            rows.push(entry(
                &plain(inner)?,
                "simulazione",
                &format!("{file}#{id}"),
                cap,
            ));
            at = start + end;
        } else {
            at = start + 1;
        }
    }
    Ok(rows)
}

/// `<h2 ...>inner</h2>` / `<h3 ...>`: (end of the match, id attribute, inner).
fn heading_at(t: &str) -> Option<(usize, Option<&str>, &str)> {
    let level = t
        .strip_prefix("<h")?
        .chars()
        .next()
        .filter(|c| matches!(c, '2' | '3'))?;
    let gt = t.find('>')?;
    let attrs = &t[3..gt];
    let id = if attrs.starts_with(' ') {
        attrs.find("id=\"").and_then(|i| {
            let v = &attrs[i + 4..];
            v.find('"').filter(|&e| e > 0).map(|e| &v[..e])
        })
    } else {
        None
    };
    let close = format!("</h{level}>");
    let end = gt + 1 + t[gt + 1..].find(&close)?;
    Some((end + close.len(), id, &t[gt + 1..end]))
}

/// `<div class="box CLASSES"...><b>inner</b>`
fn box_at(t: &str) -> Option<(usize, &str, &str)> {
    let r = t.strip_prefix("<div class=\"box ")?;
    let q = r.find('"')?;
    let gt = r[q..].find('>').map(|g| q + g)?;
    let b = r[gt + 1..].strip_prefix("<b>")?;
    let e = b.find("</b>")?;
    let used = t.len() - b.len() + e + 4;
    Some((used, &r[..q], &b[..e]))
}

/// `<div class="sim" id="ID"...><h4>inner</h4>`
fn sim_at(t: &str) -> Option<(usize, &str, &str)> {
    let r = t.strip_prefix("<div class=\"sim\" id=\"")?;
    let q = r.find('"').filter(|&q| q > 0)?;
    let gt = r[q..].find('>').map(|g| q + g)?;
    let h = r[gt + 1..].strip_prefix("<h4>")?;
    let e = h.find("</h4>")?;
    let used = t.len() - h.len() + e + 5;
    Some((used, &r[..q], &h[..e]))
}

/// The index with the rows of the chapter `file` (`12-dati.html`) rebuilt from its page `html`; its own title row and every other chapter stay as they are.
pub fn reindex(index_js: &str, file: &str, html: &str) -> Result<String, String> {
    let lines: Vec<&str> = index_js.split('\n').collect();
    let url_of = |l: &str| -> Option<String> {
        let row = l.trim_start().strip_prefix('[')?;
        let cols = json_columns(row)?;
        cols.get(2).cloned()
    };
    let mine = |l: &str| {
        url_of(l)
            .is_some_and(|u| u == file || u.strip_prefix(file).is_some_and(|r| r.starts_with('#')))
    };
    let first = lines
        .iter()
        .position(|l| mine(l))
        .ok_or("the chapter is not in the search index")?;
    let last = lines.iter().rposition(|l| mine(l)).unwrap_or(first);
    if lines[first..=last].iter().any(|l| !mine(l)) {
        return Err("the rows of the chapter are not together in the search index".into());
    }
    let cap = json_columns(lines[first].trim_start().trim_start_matches('['))
        .and_then(|c| c.get(3).cloned())
        .ok_or("unreadable row in the search index")?;
    let mut rows: Vec<String> = vec![lines[first].trim_end_matches(',').to_string()];
    rows.extend(section_rows(html, file, &cap)?);
    let trailing = lines[last].ends_with(',');
    let n = rows.len();
    let block: Vec<String> = rows
        .into_iter()
        .enumerate()
        .map(|(i, r)| {
            if i + 1 < n || trailing {
                format!("{r},")
            } else {
                r
            }
        })
        .collect();
    let mut out: Vec<String> = lines[..first].iter().map(|s| s.to_string()).collect();
    out.extend(block);
    out.extend(lines[last + 1..].iter().map(|s| s.to_string()));
    Ok(out.join("\n"))
}

/// The string columns of a row of the index (`"a", "b", ...]`): enough JSON for what the generator writes.
fn json_columns(row: &str) -> Option<Vec<String>> {
    let c: Vec<char> = row.chars().collect();
    let (mut i, mut cols) = (0, Vec::new());
    while i < c.len() {
        match c[i] {
            '"' => {
                let mut s = String::new();
                i += 1;
                while i < c.len() && c[i] != '"' {
                    if c[i] == '\\' {
                        i += 1;
                        s.push(match c.get(i)? {
                            'n' => '\n',
                            't' => '\t',
                            'r' => '\r',
                            'b' => '\u{8}',
                            'f' => '\u{c}',
                            'u' => {
                                let h: String = c.get(i + 1..i + 5)?.iter().collect();
                                i += 4;
                                char::from_u32(u32::from_str_radix(&h, 16).ok()?)?
                            }
                            o => *o,
                        });
                    } else {
                        s.push(c[i]);
                    }
                    i += 1;
                }
                cols.push(s);
                i += 1;
            }
            ']' => return Some(cols),
            _ => i += 1,
        }
    }
    None
}

/// The id a heading with this rendered text gets at run time, for the editor to keep an anchor stable.
pub fn heading_anchor(rendered_inner_html: &str) -> Result<String, String> {
    plain(rendered_inner_html).map(|t| slug(&t))
}
