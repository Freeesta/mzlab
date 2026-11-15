//! The rows of the transformation list of mzFinder (`name;change[;comment]`, `parse_transformations`), the signed changes of formula and their texts,
//! with the error texts of the Python (`str(e)`). `lists` reads the list through them and `legacy` (`transf`, `formula`) adds them to the rules;
//! they depend only on the elements, so `lists` does not depend on `legacy`.

use mzlab_core::elements::parse_term;
use mzlab_core::error::Error;

/// Python's `repr` of a string (quotes and the usual escapes; enough for the texts of the formulas).
pub fn pyrepr(s: &str) -> String {
    let q = if s.contains('\'') && !s.contains('"') {
        '"'
    } else {
        '\''
    };
    let mut o = String::from(q);
    for c in s.chars() {
        match c {
            '\\' => o.push_str("\\\\"),
            '\n' => o.push_str("\\n"),
            '\t' => o.push_str("\\t"),
            '\r' => o.push_str("\\r"),
            c if c == q => {
                o.push('\\');
                o.push(c);
            }
            c => o.push(c),
        }
    }
    o.push(q);
    o
}

/// `str(e)` of the `UserError` of `elements.parse_formula`.
pub(crate) fn formula_text(e: &Error) -> String {
    let p = |k: &str| {
        e.params
            .iter()
            .find(|(n, _)| *n == k)
            .map(|(_, v)| v.clone())
            .unwrap_or_default()
    };
    match e.key {
        "err.formula.empty" => "empty formula".into(),
        "err.formula.element" => format!(
            "unknown element {} in {}",
            pyrepr(&p("element")),
            pyrepr(&p("formula"))
        ),
        "err.formula.parens" => format!("unbalanced parentheses in {}", pyrepr(&p("formula"))),
        _ => format!("cannot read the formula {}", pyrepr(&p("formula"))),
    }
}

/// Signed counts per element, ordered by symbol (a sorted `Vec`: a map would add its code to the `.wasm`).
pub type Delta = Vec<(String, i64)>;

/// One row `name;change[;comment]`.
pub struct Row {
    pub name: String,
    pub delta: Delta,
    pub comment: String,
    /// Times the change may repeat; `None` = the default of the rules (the text of mzFinder has no such column).
    pub max_repeat: Option<u32>,
}

/// `parse_delta` of mzFinder: '+O', '-H2', '-Cl+H', '+C2H4O' -> signed counts (what is not a sign followed by letters and digits is skipped).
pub fn parse_delta(text: &str) -> Result<Delta, String> {
    let mut c: Vec<char> = text.chars().filter(|&c| c != ' ').collect();
    if c.first().is_some_and(|&x| x != '+' && x != '-') {
        c.insert(0, '+');
    }
    let mut out = Delta::new();
    let mut i = 0;
    while i < c.len() {
        if (c[i] == '+' || c[i] == '-') && c.get(i + 1).is_some_and(char::is_ascii_alphanumeric) {
            let end = (i + 1..c.len())
                .find(|&j| !c[j].is_ascii_alphanumeric())
                .unwrap_or(c.len());
            let body: String = c[i + 1..end].iter().collect();
            for (el, n) in parse_term(&body).map_err(|e| formula_text(&e))? {
                let k = if c[i] == '+' { n as i64 } else { -(n as i64) };
                match out.iter_mut().find(|(e, _)| *e == el) {
                    Some((_, v)) => *v += k,
                    None => out.push((el, k)),
                }
            }
            i = end;
        } else {
            i += 1;
        }
    }
    out.retain(|(_, n)| *n != 0);
    crate::sortc::any_by(&mut out, |a, b| a.0.cmp(&b.0));
    Ok(out)
}

/// A line break of Python's `str.splitlines` (the empty pieces of `\r\n` are skipped like any empty line).
fn is_break(c: char) -> bool {
    matches!(
        c,
        '\n' | '\r' | '\x0b' | '\x0c' | '\x1c'..='\x1e' | '\u{85}' | '\u{2028}' | '\u{2029}'
    )
}

/// `parse_transformations`: one per line, empty lines and `#` lines ignored.
pub fn parse(text: &str) -> Result<Vec<Row>, String> {
    let mut rows = Vec::new();
    for ln in text.split(is_break) {
        let ln = ln.trim();
        if ln.is_empty() || ln.starts_with('#') {
            continue;
        }
        let parts: Vec<&str> = ln.split(';').map(str::trim).collect();
        if parts.len() < 2 || parts[1].is_empty() {
            return Err(format!("invalid line (expected «name;change»): {ln}"));
        }
        rows.push(Row {
            name: parts[0].to_string(),
            delta: parse_delta(parts[1])?,
            comment: parts.get(2).map_or("", |s| s).to_string(),
            max_repeat: None,
        });
    }
    Ok(rows)
}

/// '+O', '-C-O', ... (`chem.fmt_delta`).
pub(crate) fn delta_text(d: &Delta) -> String {
    let mut o = String::new();
    for (e, n) in d {
        o.push(if *n > 0 { '+' } else { '-' });
        o.push_str(e);
        if n.abs() != 1 {
            o.push_str(&n.abs().to_string());
        }
    }
    o
}
