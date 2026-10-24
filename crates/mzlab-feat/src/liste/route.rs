//! The route `liste/edit` of the Flow editor: the rows a person typed (or the CSV they imported) in, the checked list out (`rows`, the `csv` the plan keeps, the
//! canonical `json` to export and its `sha256`), or the errors of the wrong rows. The page keeps no parsing of its own.

use mzlab_core::error::{Error, Result};
use mzlab_core::json::{parse, Out, Value};

use super::{errors_json, from_csv, to_json, Kind, Liste};
use crate::legacy::transf::delta_text;

fn header(kind: Kind) -> &'static str {
    match kind {
        Kind::Adducts => "name;delta;charge;comment",
        Kind::Transformations => "name;delta;max_repeat;comment",
    }
}

fn quote(s: &str) -> String {
    if s.contains([';', '"', '\n', '\r']) {
        format!("\"{}\"", s.replace('"', "\"\""))
    } else {
        s.to_string()
    }
}

/// The four cells of each row, in the order of the header.
fn cells(list: &Liste) -> Vec<[String; 4]> {
    match list {
        Liste::Adducts(v) => v
            .iter()
            .map(|a| {
                [
                    a.name.clone(),
                    delta_text(&a.delta),
                    a.charge.to_string(),
                    a.comment.clone(),
                ]
            })
            .collect(),
        Liste::Transformations(v) => v
            .iter()
            .map(|t| {
                let rep = t.max_repeat.map(|m| m.to_string()).unwrap_or_default();
                [t.name.clone(), delta_text(&t.delta), rep, t.comment.clone()]
            })
            .collect(),
    }
}

fn csv_of(kind: Kind, rows: &[[String; 4]]) -> String {
    let mut out = format!("{}\n", header(kind));
    for r in rows {
        let line: Vec<String> = r.iter().map(|c| quote(c)).collect();
        out.push_str(&line.join(";"));
        out.push('\n');
    }
    out
}

/// The CSV of the rows `[{name, delta, charge|max_repeat, comment}]` of the request.
fn csv_from_rows(kind: Kind, rows: &[Value]) -> String {
    let third = if kind == Kind::Adducts {
        "charge"
    } else {
        "max_repeat"
    };
    let cell = |r: &Value, k: &str| match r.get(k) {
        Some(Value::Num(n)) => n.to_string(),
        v => v.and_then(Value::str).unwrap_or("").trim().to_string(),
    };
    let all: Vec<[String; 4]> = rows
        .iter()
        .map(|r| {
            [
                cell(r, "name"),
                cell(r, "delta"),
                cell(r, third),
                cell(r, "comment"),
            ]
        })
        .collect();
    csv_of(kind, &all)
}

/// `{kind, rows|csv}` -> `{rows, csv, sha256, json}` or `{errors}`; a wrong row is the error of its line in the CSV that `csv` would be.
pub fn route(body: &str) -> Result<String> {
    let req = parse(body.as_bytes()).map_err(|_| Error::new("error.badListe"))?;
    let kind = req
        .get("kind")
        .and_then(Value::str)
        .and_then(Kind::from_name)
        .ok_or_else(|| Error::new("error.badListe"))?;
    let csv = match (req.get("rows"), req.get("csv").and_then(Value::str)) {
        (Some(r), _) => csv_from_rows(kind, r.arr()),
        (None, Some(t)) => t.to_string(),
        _ => return Err(Error::new("error.badListe")),
    };
    let list = match from_csv(kind, &csv) {
        Ok(l) => l,
        Err(e) => return Ok(errors_json(&e)),
    };
    let rows = cells(&list);
    let mut o = Out::default();
    o.raw("{\"rows\":[");
    let third = if kind == Kind::Adducts {
        "charge"
    } else {
        "max_repeat"
    };
    for (i, r) in rows.iter().enumerate() {
        o.raw(if i > 0 { "," } else { "" })
            .raw("{\"name\":")
            .str(&r[0]);
        o.raw(",\"delta\":")
            .str(&r[1])
            .raw(",\"")
            .raw(third)
            .raw("\":")
            .str(&r[2]);
        o.raw(",\"comment\":").str(&r[3]).raw("}");
    }
    o.raw("],\"csv\":").str(&csv_of(kind, &rows));
    o.raw(",\"sha256\":")
        .str(&list.digest())
        .raw(",\"json\":")
        .str(&to_json(&list))
        .raw("}");
    Ok(o.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sha(body: &str) -> String {
        let v = parse(route(body).unwrap().as_bytes()).unwrap();
        v.get("sha256").and_then(Value::str).unwrap().to_string()
    }

    #[test]
    fn rows_and_csv_give_the_same_list_and_a_change_gives_a_new_sha() {
        let rows = r#"{"kind":"adducts","rows":[{"name":"[M+H]+","delta":"+H","charge":"1","comment":""}]}"#;
        let csv = r#"{"kind":"adducts","csv":"name;delta;charge;comment\n[M+H]+;+H;1;\n"}"#;
        let other = r#"{"kind":"adducts","rows":[{"name":"[M+Na]+","delta":"+Na","charge":"1","comment":"x"}]}"#;
        assert_eq!(sha(rows), sha(csv));
        assert_ne!(sha(rows), sha(other));
    }

    #[test]
    fn a_wrong_row_gives_its_error() {
        let bad =
            r#"{"kind":"adducts","rows":[{"name":"","delta":"+H","charge":"1","comment":""}]}"#;
        let out = route(bad).unwrap();
        assert!(
            out.contains("error.liste.noName") && out.contains("\"line\":2"),
            "{out}"
        );
        assert!(route(r#"{"kind":"nope","rows":[]}"#).is_err());
    }

    #[test]
    fn an_empty_list_has_a_sha() {
        assert_eq!(sha(r#"{"kind":"adducts","rows":[]}"#).len(), 64);
    }
}
