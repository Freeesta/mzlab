//! The POST routes `api/stat/profile`, `api/stat/blank`, `api/stat/reference` and `api/stat/pca` (`_p_stat_*` of `mzlab/api.py`): the body
//! is the JSON the page sends, the answer the JSON of Python, an error the JSON of `api._fail` (`{error, error_key, params}`).

use crate::json::{parse, Value};
use crate::jsonb::{text, write, Obj};
use crate::pca::{pca_analysis, PcaOpts};
use crate::statistica::{bad, blank_filter, blank_value, clean_matrix, time_profile, StatErr, R};
use crate::statref::{identity_links, reference_difference, RefIn, RefOpts, FLOOR};

fn num(b: &Value, k: &str, d: f64) -> R<f64> {
    match b.get(k) {
        None => Ok(d),
        Some(Value::Num(n)) => Ok(*n),
        _ => bad(&format!("missing or invalid parameter: '{k}'")),
    }
}

fn int(b: &Value, k: &str, d: i64) -> R<i64> {
    num(b, k, d as f64).map(|n| n as i64)
}

fn string(b: &Value, k: &str, d: &str) -> R<String> {
    match b.get(k) {
        None => Ok(d.to_string()),
        Some(Value::Str(s)) => Ok(s.clone()),
        _ => bad(&format!("missing or invalid parameter: '{k}'")),
    }
}

fn matrix(b: &Value) -> R<Vec<Vec<f64>>> {
    let mut rows = Vec::new();
    for r in b.get("matrix").map_or(&[][..], Value::arr) {
        let cells = r.arr().iter().map(|v| match v {
            Value::Num(n) => Ok(Some(*n)),
            Value::Null => Ok(None),
            _ => bad("a number was expected in the table"),
        });
        rows.push(cells.collect::<R<Vec<_>>>()?);
    }
    clean_matrix(&rows)
}

fn roles(b: &Value) -> R<Vec<String>> {
    b.get("roles")
        .map_or(&[][..], Value::arr)
        .iter()
        .map(|v| {
            v.str()
                .map(String::from)
                .map_or_else(|| bad("a role must be a text"), Ok)
        })
        .collect()
}

/// A group label as `str(g)` makes it (null stays None; an integer has no decimals).
fn label(v: &Value) -> Option<String> {
    match v {
        Value::Null => None,
        Value::Str(s) => Some(s.clone()),
        Value::Num(n) if n.fract() == 0.0 => Some(format!("{}", *n as i64)),
        other => Some(write(other)),
    }
}

fn groups(b: &Value) -> Option<Vec<Option<String>>> {
    b.get("groups")
        .filter(|g| !g.is_null())
        .map(|g| g.arr().iter().map(label).collect())
}

fn times(b: &Value) -> R<Vec<Option<f64>>> {
    b.get("times")
        .map_or(&[][..], Value::arr)
        .iter()
        .map(|v| match v {
            Value::Num(n) => Ok(Some(*n)),
            Value::Null => Ok(None),
            _ => bad("a time must be a number"),
        })
        .collect()
}

fn floats(b: &Value, k: &str) -> R<Vec<f64>> {
    b.get(k)
        .map_or(&[][..], Value::arr)
        .iter()
        .map(|v| v.f64().map_or_else(|| bad("a number was expected"), Ok))
        .collect()
}

fn profile(b: &Value) -> R<Value> {
    let istd = b.get("istd").filter(|v| !v.is_null());
    time_profile(
        b.get("points").map_or(&[][..], Value::arr),
        &string(b, "metric", "area")?,
        istd,
    )
}

fn blank(b: &Value) -> R<Value> {
    let (factor, stat, min_detect) = (
        num(b, "factor", 3.0)?,
        string(b, "stat", "max")?,
        int(b, "min_detect", 1)?,
    );
    let out = blank_filter(&matrix(b)?, &roles(b)?, factor, &stat, min_detect)?;
    Ok(blank_value(&out, &stat, factor, min_detect))
}

fn reference(b: &Value) -> R<Value> {
    let (m, r, t, g) = (matrix(b)?, roles(b)?, times(b)?, groups(b));
    let chosen: Option<Vec<i64>> = match b.get("reference").filter(|v| !v.is_null()) {
        Some(v) => Some(
            v.arr()
                .iter()
                .map(|x| {
                    x.f64()
                        .map_or_else(|| bad("a column index was expected"), |n| Ok(n as i64))
                })
                .collect::<R<Vec<_>>>()?,
        ),
        None => None,
    };
    let pseudo = match b.get("pseudo").filter(|v| !v.is_null()) {
        Some(_) => Some(num(b, "pseudo", 0.0)?),
        None => None,
    };
    let o = RefOpts {
        factor: num(b, "factor", 3.0)?,
        ref_stat: string(b, "stat", "max")?,
        min_detect: int(b, "min_detect", 1)?,
        pseudo,
        floor: num(b, "floor", FLOOR)?,
        ..RefOpts::default()
    };
    let inp = RefIn {
        matrix: &m,
        roles: &r,
        times: &t,
        groups: g.as_deref(),
        reference: chosen.as_deref(),
    };
    let mut out = reference_difference(&inp, &o)?;
    let (mz, rt) = (floats(b, "mz")?, floats(b, "rt")?);
    if !mz.is_empty() && mz.len() == rt.len() {
        // shown beside the numbers, never merged
        if let Value::Obj(o) = &mut out {
            o.push(("identity".to_string(), identity_links(&mz, &rt)?));
        }
    }
    Ok(out)
}

fn pca(b: &Value) -> R<Value> {
    let o = PcaOpts {
        factor: num(b, "factor", 3.0)?,
        stat: string(b, "stat", "max")?,
        min_detect: int(b, "min_detect", 1)?,
        min_intensity: num(b, "min_intensity", 0.0)?,
        min_samples: int(b, "min_samples", 2)?,
        transform: string(b, "transform", "log")?,
        scaling: string(b, "scaling", "pareto")?,
        ncomp: int(b, "ncomp", 5)?.max(1) as usize,
    };
    let g = groups(b);
    pca_analysis(&matrix(b)?, &roles(b)?, g.as_deref(), &o)
}

/// The JSON of `api._fail` for an error.
fn error_json(e: &StatErr) -> String {
    let mut o = Obj::new().put("error", text(&e.text));
    if let Some(k) = e.key {
        o = o.put("error_key", text(k)).put(
            "params",
            e.params
                .iter()
                .fold(Obj::new(), |o, (k, v)| o.n(k, *v))
                .done(),
        );
    }
    write(&o.done())
}

/// The answer to a POST of `path`: None = not a statistics route; Ok = the JSON (status 200); Err = the JSON of the error (status 400).
pub fn stat_route(path: &str, body: &str) -> Option<Result<String, String>> {
    let run: fn(&Value) -> R<Value> = match path.trim_start_matches("./").trim_start_matches('/') {
        "api/stat/profile" => profile,
        "api/stat/blank" => blank,
        "api/stat/reference" => reference,
        "api/stat/pca" => pca,
        _ => return None,
    };
    let b = match parse(body.as_bytes()) {
        Ok(b @ Value::Obj(_)) => b,
        _ => {
            return Some(Err(error_json(&StatErr {
                key: None,
                params: Vec::new(),
                text: "the request must be a JSON object".into(),
            })))
        }
    };
    Some(run(&b).map(|v| write(&v)).map_err(|e| error_json(&e)))
}
