//! `flow/liste` for the kind `transitions` (`liste::route` hands it over): the rows a person typed (or the CSV they imported) in, the checked table out
//! (`rows` as written, the `csv` the plan keeps, a `json` to export and its `sha256`), or the errors of the wrong rows. The row order is kept: it is the
//! order of the outputs.

use mzlab_core::budget::sha256_hex;
use mzlab_core::json::{Out, Value};

use super::table::{parse, to_csv, Row, HEADER};
use crate::liste::errors_json;

fn cell(r: &Value, k: &str) -> String {
    match r.get(k) {
        Some(Value::Num(n)) => n.to_string(),
        v => v.and_then(Value::str).unwrap_or("").trim().to_string(),
    }
}

fn quote(s: &str) -> String {
    if s.contains([';', '"', '\n', '\r']) {
        format!("\"{}\"", s.replace('"', "\"\""))
    } else {
        s.to_string()
    }
}

fn csv_from_rows(rows: &[Value]) -> String {
    let mut out = HEADER.join(";") + "\n";
    for r in rows {
        let line: Vec<String> = HEADER.iter().map(|k| quote(&cell(r, k))).collect();
        out += &(line.join(";") + "\n");
    }
    out
}

fn rows_json(o: &mut Out, rows: &[Row]) {
    o.raw("[");
    for (i, r) in rows.iter().enumerate() {
        o.raw(if i > 0 { ",{" } else { "{" });
        for (k, (h, c)) in HEADER.iter().zip(&r.cells).enumerate() {
            o.raw(if k > 0 { "," } else { "" }).str(h).raw(":").str(c);
        }
        o.raw("}");
    }
    o.raw("]");
}

/// The answer to `{kind: "transitions", rows|csv}`; `req` is the parsed request.
pub fn route(req: &Value) -> Option<String> {
    let csv = match (req.get("rows"), req.get("csv").and_then(Value::str)) {
        (Some(r), _) => csv_from_rows(r.arr()),
        (None, Some(t)) => t.to_string(),
        _ => return None,
    };
    let rows = match parse(&csv) {
        Ok(r) => r,
        Err(e) => return Some(errors_json(&e)),
    };
    let mut json = Out::default();
    json.raw("{\"rows\":");
    rows_json(&mut json, &rows);
    json.raw(",\"schema\":\"transitions/1\"}");
    let mut o = Out::default();
    o.raw("{\"rows\":");
    rows_json(&mut o, &rows);
    o.raw(",\"csv\":").str(&to_csv(&rows));
    o.raw(",\"sha256\":")
        .str(&sha256_hex(json.0.as_bytes()))
        .raw(",\"json\":")
        .str(&json.0)
        .raw("}");
    Some(o.0)
}
