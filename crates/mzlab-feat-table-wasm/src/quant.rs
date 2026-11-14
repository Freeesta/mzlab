//! The text of `quant.calibrate@1`: the calibration curves the user imported (slope, intercept, linear range per compound, made in a spreadsheet) applied to the
//! areas of a feature table, as a long CSV of concentrations with a flag per cell. It never fits a line: the standards are not read. Bytes in, text out: `flow run`
//! (`mzlab-cli`) and the browser module `table` (`flow_quant`) both call it, so they write the same bytes.

use mzlab_core::error::{Error, Result};
use mzlab_core::json::{Out, Value};
use mzlab_feat::samples::{self, KINDS};
use mzlab_feat::store::{origin_from_meta, read_table_from, Slice};
use mzlab_feat::table::{Aligned, CELL_ABSENT};
use wasm_bindgen::prelude::*;

pub const DEFAULT_MZ_TOL: f64 = 0.5;
pub const DEFAULT_RT_TOL: f64 = 0.3;

struct Curve {
    name: String,
    mz: f64,
    rt: f64,
    slope: f64,
    intercept: f64,
    unit: String,
    lo: Option<f64>,
    hi: Option<f64>,
    dilution: f64,
}

fn fields(line: &str, d: char) -> Vec<String> {
    let (mut out, mut cur, mut quoted) = (Vec::new(), String::new(), false);
    let mut it = line.chars().peekable();
    while let Some(c) = it.next() {
        match c {
            '"' if quoted && it.peek() == Some(&'"') => {
                cur.push('"');
                it.next();
            }
            '"' if quoted || cur.is_empty() => quoted = !quoted,
            c if c == d && !quoted => out.push(std::mem::take(&mut cur).trim().to_string()),
            c => cur.push(c),
        }
    }
    out.push(cur.trim().to_string());
    out
}

/// Positions of the columns of the curves file.
struct Cols {
    name: usize,
    mz: usize,
    rt: usize,
    slope: usize,
    intercept: usize,
    unit: Option<usize>,
    lo: Option<usize>,
    hi: Option<usize>,
    dilution: Option<usize>,
}

/// One curve from the fields of line `n`; `d` is the delimiter (a decimal comma is read when it is not `,`).
fn curve_row(f: &[String], d: char, n: usize, c: &Cols) -> Result<Curve> {
    let cell = |c: usize| f.get(c).map(String::as_str).unwrap_or("");
    let row = |what: &str| {
        Error::new("err.flow.quant.curveRow").with("detail", format!("line {n}: {what}"))
    };
    let num = |s: &str| -> Option<f64> {
        let s = if d == ',' {
            s.to_string()
        } else {
            s.replace(',', ".")
        };
        s.parse::<f64>().ok().filter(|v| v.is_finite())
    };
    let req = |c: usize, what: &str| num(cell(c)).ok_or_else(|| row(what));
    let opt = |c: Option<usize>, what: &str| match c.map(cell) {
        None | Some("") => Ok(None),
        Some(s) => num(s).map(Some).ok_or_else(|| row(what)),
    };
    let curve = Curve {
        name: cell(c.name).to_string(),
        mz: req(c.mz, "mz")?,
        rt: req(c.rt, "rt")?,
        slope: req(c.slope, "slope")?,
        intercept: req(c.intercept, "intercept")?,
        unit: c.unit.map(cell).unwrap_or("").to_string(),
        lo: opt(c.lo, "range_min")?,
        hi: opt(c.hi, "range_max")?,
        dilution: opt(c.dilution, "dilution")?.unwrap_or(1.0),
    };
    if curve.name.is_empty() {
        return Err(row("name"));
    }
    if curve.slope == 0.0 {
        return Err(row("slope is zero"));
    }
    if curve.dilution <= 0.0 {
        return Err(row("dilution"));
    }
    if matches!((curve.lo, curve.hi), (Some(a), Some(b)) if a > b) {
        return Err(row("range_min above range_max"));
    }
    Ok(curve)
}

/// The curves of the CSV; the delimiter is the one of the header (`;` and tab read a decimal comma, as a spreadsheet writes it). The columns come in any order;
/// `range_min`, `range_max`, `dilution` and `unit` may be empty.
fn parse_curves(bytes: &[u8]) -> Result<Vec<Curve>> {
    let text = String::from_utf8_lossy(bytes);
    let mut lines = text
        .lines()
        .enumerate()
        .map(|(i, l)| (i + 1, l.trim_start_matches('\u{feff}')))
        .filter(|(_, l)| !l.trim().is_empty() && !l.starts_with('#'));
    let bad = |detail: String| Error::new("err.flow.quant.curves").with("detail", detail);
    let (_, head) = lines.next().ok_or_else(|| bad("empty file".into()))?;
    let d = [';', '\t', ',']
        .into_iter()
        .find(|d| head.contains(*d))
        .unwrap_or(',');
    let names: Vec<String> = fields(head, d).iter().map(|h| h.to_lowercase()).collect();
    let col = |n: &str| names.iter().position(|h| h == n);
    let need = |n: &str| col(n).ok_or_else(|| bad(format!("no column {n}")));
    let cols = Cols {
        name: need("name")?,
        mz: need("mz")?,
        rt: need("rt")?,
        slope: need("slope")?,
        intercept: need("intercept")?,
        unit: col("unit"),
        lo: col("range_min"),
        hi: col("range_max"),
        dilution: col("dilution"),
    };
    let mut out: Vec<Curve> = Vec::new();
    for (n, line) in lines {
        let curve = curve_row(&fields(line, d), d, n, &cols)?;
        if out.iter().any(|c| c.name == curve.name) {
            return Err(Error::new("err.flow.quant.duplicate").with("name", curve.name));
        }
        out.push(curve);
    }
    if out.is_empty() {
        return Err(bad("no curve".into()));
    }
    Ok(out)
}

fn param<'a>(params: &'a [(String, Value)], name: &str) -> Option<&'a Value> {
    params.iter().find(|(k, _)| k == name).map(|(_, v)| v)
}

fn quoted(s: &str) -> String {
    if s.contains([',', ';', '"', '\n']) {
        format!("\"{}\"", s.replace('"', "\"\""))
    } else {
        s.to_string()
    }
}

/// The row nearest to the curve inside both tolerances, the first one on a tie.
fn nearest_row(t: &Aligned, c: &Curve, mz_tol: f64, rt_tol: f64) -> Option<usize> {
    let mut best: Option<(f64, usize)> = None;
    for row in 0..t.mz.len() {
        let (dm, dr) = ((t.mz[row] - c.mz).abs(), (t.rt[row] - c.rt).abs());
        if dm > mz_tol || dr > rt_tol {
            continue;
        }
        let d = dm / mz_tol.max(1e-9) + dr / rt_tol.max(1e-9);
        if best.is_none_or(|b| d < b.0) {
            best = Some((d, row));
        }
    }
    best.map(|b| b.1)
}

/// The flag of a concentration read from the curve (before the dilution).
fn flag_of(measured: f64, c: &Curve) -> &'static str {
    if measured < 0.0 {
        "negative"
    } else if c.lo.is_some_and(|lo| measured < lo) {
        "below_range"
    } else if c.hi.is_some_and(|hi| measured > hi) {
        "above_range"
    } else {
        "ok"
    }
}

/// The sample types to quantify, each one a type of `campioni/1`.
fn kinds_of(params: &[(String, Value)]) -> Result<Vec<String>> {
    let kinds: Vec<String> = param(params, "kinds")
        .and_then(|v| v.str())
        .unwrap_or("sample")
        .split(',')
        .map(|k| k.trim().to_string())
        .filter(|k| !k.is_empty())
        .collect();
    match kinds.iter().find(|k| !KINDS.contains(&k.as_str())) {
        Some(k) => Err(Error::new("err.flow.quant.kind").with("kind", k.clone())),
        None => Ok(kinds),
    }
}

/// The columns to quantify, in the order of the table, and whether every file of the table has a role.
fn quantified_columns<'a>(
    files: &'a [(String, String)],
    roles: &[samples::Sample],
    kinds: &[String],
) -> (Vec<(usize, &'a str)>, bool) {
    let cols = files
        .iter()
        .enumerate()
        .filter_map(|(i, (name, _))| {
            let role = roles.iter().find(|s| &s.file == name)?;
            kinds.contains(&role.kind).then_some((i, name.as_str()))
        })
        .collect();
    let listed = files
        .iter()
        .all(|(n, _)| roles.iter().any(|s| &s.file == n));
    (cols, listed)
}

/// The `warn.flow.quant.*` keys of the conditions met: no row for a curve, a row shared by two curves, a concentration out of its range, a negative one.
fn warn_keys(on: [bool; 4]) -> Vec<String> {
    let keys = [
        "warn.flow.quant.noRow",
        "warn.flow.quant.sharedRow",
        "warn.flow.quant.range",
        "warn.flow.quant.negative",
    ];
    keys.iter()
        .zip(on)
        .filter(|(_, on)| *on)
        .map(|(k, _)| k.to_string())
        .collect()
}

/// The CSV `compound,mz,rt,sample,signal,concentration,unit,flag` and the warnings. `flag` is `ok`, `not_detected`, `below_range`, `above_range` or `negative`;
/// the range is checked on the concentration read from the curve, the dilution factor is applied after it.
pub fn quant_text(
    params: &[(String, Value)],
    table: &[u8],
    curves: &[u8],
    samples: &[u8],
) -> Result<(String, Vec<String>)> {
    let curves = parse_curves(curves)?;
    let mz_tol = param(params, "mz_tol")
        .and_then(|v| v.f64())
        .unwrap_or(DEFAULT_MZ_TOL);
    let rt_tol = param(params, "rt_tol")
        .and_then(|v| v.f64())
        .unwrap_or(DEFAULT_RT_TOL);
    let by_height = param(params, "signal").and_then(|v| v.str()) == Some("height");
    let kinds = kinds_of(params)?;
    let (t, meta) = read_table_from(&mut Slice(table), table.len() as u64)?;
    let files = origin_from_meta(&meta).inputs;
    let roles = samples::from_json(crate::pol::samples_v1(samples)?.as_bytes())?;
    let mut warnings: Vec<String> = Vec::new();
    let (cols, listed) = quantified_columns(&files, &roles, &kinds);
    if !listed {
        warnings.push("warn.flow.quant.unlisted".into());
    }
    let mut out = String::from("compound,mz,rt,sample,signal,concentration,unit,flag\n");
    let (mut no_row, mut shared, mut range, mut negative) = (false, false, false, false);
    let mut taken: Vec<usize> = Vec::new();
    for c in &curves {
        let best = nearest_row(&t, c, mz_tol, rt_tol);
        let Some(row) = best else {
            no_row = true;
            continue;
        };
        if taken.contains(&row) {
            shared = true;
        }
        taken.push(row);
        for &(file, name) in &cols {
            let cell = row * t.n_files + file;
            let head = format!(
                "{},{},{},{},",
                quoted(&c.name),
                t.mz[row],
                t.rt[row],
                quoted(name)
            );
            if t.state[cell] == CELL_ABSENT {
                out += &format!("{head},,{},not_detected\n", quoted(&c.unit));
                continue;
            }
            let signal = if by_height {
                t.height[cell] as f64
            } else {
                t.area[cell]
            };
            let measured = (signal - c.intercept) / c.slope;
            let flag = flag_of(measured, c);
            negative |= flag == "negative";
            range |= flag.ends_with("_range");
            out += &format!(
                "{head}{signal},{:.6e},{},{flag}\n",
                measured * c.dilution,
                quoted(&c.unit)
            );
        }
    }
    warnings.extend(warn_keys([no_row, shared, range, negative]));
    Ok((out, warnings))
}

/// `quant_text` for the browser: `params_json` is the step's parameters merged with its defaults; the answer is the JSON `{"text": <the CSV>, "warnings": [<warn.* keys>]}`.
/// Errors are JSON `{"error_key","params"}` texts.
#[wasm_bindgen]
pub fn flow_quant(
    params_json: &str,
    table: &[u8],
    curves: &[u8],
    samples: &[u8],
) -> std::result::Result<String, JsValue> {
    let params = match mzlab_core::json::parse(params_json.as_bytes()).map_err(crate::js_err)? {
        Value::Obj(o) => o,
        _ => Vec::new(),
    };
    let (text, warnings) = quant_text(&params, table, curves, samples).map_err(crate::js_err)?;
    let mut o = Out::default();
    o.raw("{\"text\":");
    o.str(&text);
    o.raw(",\"warnings\":[");
    for (k, w) in warnings.iter().enumerate() {
        if k > 0 {
            o.raw(",");
        }
        o.str(w);
    }
    o.raw("]}");
    Ok(o.0)
}
