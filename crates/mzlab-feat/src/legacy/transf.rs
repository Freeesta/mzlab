//! The list of transformations the user edits in mzFinder (`parse_transformations` of mzFinder) and its rows as extra rules of the hypotheses (L5): the changes
//! of formula that `transformation_table` adds to the known ones and the `lit:` comments of `literature`. `merge` returns the rules
//! file with the new rows appended (a change that a rule already has is not added twice; a `lit:` comment turns the rule of that change into a literature rule).
//! The error texts are those of the Python.

use mzlab_core::json::{self, Out, Value};

use super::Res;
use crate::transf_base::delta_text;

/// The old path of the rows of the list, which live in `crate::transf_base` so that `lists` does not depend on `legacy`.
pub use crate::transf_base::{parse, parse_delta, Delta, Row};

/// The reference of a comment that starts with `lit:` (case does not matter), if it has one.
fn literature(comment: &str) -> Option<String> {
    let c = comment.trim_start();
    let head: String = c.chars().take(3).collect();
    let rest = c.get(3..).filter(|_| head.eq_ignore_ascii_case("lit"))?;
    let reference = rest.trim_start().strip_prefix(':')?.trim();
    (!reference.is_empty()).then(|| reference.to_string())
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
        (
            "passi_max".into(),
            Value::Num(f64::from(r.max_repeat.unwrap_or(2))),
        ),
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

/// The mutable member `key` of an object value.
fn member<'a>(v: &'a mut Value, key: &str) -> Option<&'a mut Value> {
    match v {
        Value::Obj(o) => o.iter_mut().find(|(k, _)| k == key).map(|(_, v)| v),
        _ => None,
    }
}

/// The `regole_tp` file `base` (`/1` list or the default profile of `/2`) with the rows of `text` as rules.
pub fn merge(base: &[u8], text: &str) -> Res<Vec<u8>> {
    let rows = crate::lists::from_legacy_text(text)?.transformation_rows();
    let unreadable = || "the rules of Tabui cannot be read".to_string();
    let mut file = json::parse(base).map_err(|_| unreadable())?;
    let default = file
        .get("predefinito")
        .and_then(Value::str)
        .map(String::from);
    let flat = file.get("regole").is_some();
    let holder = if flat {
        Some(&mut file)
    } else {
        default
            .as_deref()
            .and_then(|d| member(&mut file, "profili").and_then(|p| member(p, d)))
    };
    let Some(Value::Arr(list)) = holder.and_then(|h| member(h, "regole")) else {
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
