//! The list of transformations the user edits in mzFinder (`tpmine.chem.parse_transformations`) and its rows as extra rules of the hypotheses (L5): the changes
//! of formula that `hr/network.py::transformation_table` adds to the known ones and the `lit:` comments of `hr/rules.py::literature`. `merge` returns the rules
//! file with the new rows appended (a change that a rule already has is not added twice; a `lit:` comment turns the rule of that change into a literature rule).
//! The error texts are those of the Python.

use mzlab_core::elements::parse_formula;
use mzlab_core::json::{self, Out, Value};

use super::formula::formula_text;
use super::Res;

/// Signed counts per element, ordered by symbol (a sorted `Vec`: a map would add its code to the `.wasm`).
pub type Delta = Vec<(String, i64)>;

/// One row `name;change[;comment]`.
pub struct Row {
    pub name: String,
    pub delta: Delta,
    pub comment: String,
}

/// `tpmine.chem.parse_delta`: '+O', '-H2', '-Cl+H', '+C2H4O' -> signed counts (what is not a sign followed by letters and digits is skipped).
pub fn parse_delta(text: &str) -> Res<Delta> {
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
            for (el, n) in parse_formula(&body).map_err(|e| formula_text(&e))? {
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
pub fn parse(text: &str) -> Res<Vec<Row>> {
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
        });
    }
    Ok(rows)
}

/// The reference of a comment that starts with `lit:` (case does not matter), if it has one.
fn literature(comment: &str) -> Option<String> {
    let c = comment.trim_start();
    let head: String = c.chars().take(3).collect();
    let rest = c.get(3..).filter(|_| head.eq_ignore_ascii_case("lit"))?;
    let reference = rest.trim_start().strip_prefix(':')?.trim();
    (!reference.is_empty()).then(|| reference.to_string())
}

/// '+O', '-C-O', ... (`chem.fmt_delta`).
fn delta_text(d: &Delta) -> String {
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

fn set(rule: &mut Value, key: &str, v: Value) {
    if let Value::Obj(o) = rule {
        match o.iter_mut().find(|(k, _)| k == key) {
            Some((_, old)) => *old = v,
            None => o.push((key.to_string(), v)),
        }
    }
}

fn new_rule(r: &Row) -> Value {
    let s = |x: &str| Value::Str(x.to_string());
    Value::Obj(vec![
        ("id".into(), s(&r.name)),
        ("nome".into(), s(&r.name)),
        ("delta_formula".into(), Value::Str(delta_text(&r.delta))),
        ("passi_max".into(), Value::Num(2.0)),
        ("livello_previsione".into(), Value::Num(4.0)),
        ("fonte".into(), s("transformation_table")),
        ("direzione_rt".into(), s("libera")),
    ])
}

fn emit(v: &Value, o: &mut Out) {
    match v {
        Value::Null => o.raw("null"),
        Value::Bool(b) => o.raw(if *b { "true" } else { "false" }),
        Value::Num(n) => o.num(*n),
        Value::Str(s) => o.str(s),
        Value::Arr(a) => {
            o.raw("[");
            for (i, x) in a.iter().enumerate() {
                o.raw(if i > 0 { "," } else { "" });
                emit(x, o);
            }
            o.raw("]")
        }
        Value::Obj(m) => {
            o.raw("{");
            for (i, (k, x)) in m.iter().enumerate() {
                o.raw(if i > 0 { "," } else { "" }).str(k).raw(":");
                emit(x, o);
            }
            o.raw("}")
        }
    };
}

/// The `regole_tp/1` file `base` with the rows of `text` as rules.
pub fn merge(base: &[u8], text: &str) -> Res<Vec<u8>> {
    let rows = parse(text)?;
    let unreadable = || "the rules of mzFinder cannot be read".to_string();
    let mut file = json::parse(base).map_err(|_| unreadable())?;
    let Value::Obj(top) = &mut file else {
        return Err(unreadable());
    };
    let Some(Value::Arr(list)) = top.iter_mut().find(|(k, _)| k == "regole").map(|(_, v)| v) else {
        return Err(unreadable());
    };
    let mut known: Vec<Delta> = list
        .iter()
        .map(|r| {
            let t = r.get("delta_formula").and_then(Value::str).unwrap_or("");
            parse_delta(t).unwrap_or_default()
        })
        .collect();
    for r in rows.iter().filter(|r| !r.delta.is_empty()) {
        let at = known.iter().position(|d| *d == r.delta).unwrap_or_else(|| {
            list.push(new_rule(r));
            known.push(r.delta.clone());
            list.len() - 1
        });
        if let Some(l) = literature(&r.comment) {
            set(&mut list[at], "livello_previsione", Value::Num(2.0));
            set(&mut list[at], "fonte", Value::Str(format!("lit:{l}")));
        }
    }
    let mut o = Out::default();
    emit(&file, &mut o);
    Ok(o.0.into_bytes())
}

/// Class of a change for the table of the page: `halogen` (a halogen changes), `oxidation` (oxygen gained), `loss` (the mass goes down), `gain`.
fn class_of(d: &Delta) -> &'static str {
    let n = |e: &str| d.iter().find(|(x, _)| x == e).map_or(0, |(_, k)| *k);
    if ["Cl", "Br", "F", "I"].iter().any(|e| n(e) != 0) {
        "halogen"
    } else if n("O") > 0 {
        "oxidation"
    } else if mass_delta(d) < 0.0 {
        "loss"
    } else {
        "gain"
    }
}

fn mass_delta(d: &Delta) -> f64 {
    d.iter()
        .map(|(e, k)| mzlab_core::elements::mass_of(e).unwrap_or(0.0) * *k as f64)
        .sum()
}

/// One line of the list as a row for the table: `{"line","on","name","change","dm","class","note","lit"}`, or `{"line","on","error"}` for an active line that is wrong.
/// A `#` line that reads as a row is an inactive row (`on` false); any other comment gives no row. Lines are those of `text.split('\n')`.
pub fn describe(text: &str) -> String {
    let mut o = Out::default();
    o.raw("{\"rows\":[");
    let mut first = true;
    for (i, raw) in text.split('\n').enumerate() {
        let ln = raw.trim();
        let (on, body) = match ln.strip_prefix('#') {
            Some(rest) => (false, rest.trim_start_matches('#').trim()),
            None => (true, ln),
        };
        if body.is_empty() {
            continue;
        }
        let parsed = parse(body).and_then(|mut r| match r.pop() {
            Some(row) if !row.delta.is_empty() => Ok(row),
            _ => Err("no change".to_string()),
        });
        let (sep, ok) = (if first { "" } else { "," }, parsed.is_ok() || on);
        if !ok {
            continue;
        }
        first = false;
        o.raw(sep);
        match parsed {
            Ok(r) => {
                o.raw("{\"line\":")
                    .int(i as i64)
                    .raw(",\"on\":")
                    .raw(if on { "true" } else { "false" });
                o.raw(",\"name\":")
                    .str(&r.name)
                    .raw(",\"change\":")
                    .str(&delta_text(&r.delta));
                o.raw(",\"dm\":")
                    .num(mzlab_core::elements::round_half_up(mass_delta(&r.delta), 4));
                o.raw(",\"class\":")
                    .str(class_of(&r.delta))
                    .raw(",\"note\":")
                    .str(&r.comment);
                o.raw(",\"lit\":").raw(if literature(&r.comment).is_some() {
                    "true}"
                } else {
                    "false}"
                });
            }
            Err(e) => {
                o.raw("{\"line\":")
                    .int(i as i64)
                    .raw(",\"on\":true,\"error\":")
                    .str(&e)
                    .raw("}");
            }
        }
    }
    o.raw("]}");
    o.0
}
