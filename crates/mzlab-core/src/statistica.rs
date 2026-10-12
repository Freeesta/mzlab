//! `mzlab/statistica.py` (part 1): the error type of the statistics routes, the feature matrix, the time profile of one ion and the blank
//! filter of a feature table. Numbers only, never verdicts. The reference difference is in `statref.rs`, the PCA in `pca.rs`, the
//! JSON of the requests in `statroute.rs`.

use std::cmp::Ordering::Equal;

use crate::json::Value;
use crate::jsonb::{floats, int, num, opt, text, Obj};
use crate::stats::psum;

/// What Python raises: a `UserError` (key, parameters, English text) or any other error (only a text).
pub struct StatErr {
    pub key: Option<&'static str>,
    pub params: Vec<(&'static str, f64)>,
    pub text: String,
}

pub type R<T> = Result<T, StatErr>;

/// A `UserError` without parameters.
pub fn user<T>(key: &'static str, text: &str) -> R<T> {
    Err(StatErr { key: Some(key), params: Vec::new(), text: text.to_string() })
}

/// Any other error (a bad or missing parameter): only a text.
pub fn bad<T>(text: &str) -> R<T> {
    Err(StatErr { key: None, params: Vec::new(), text: text.to_string() })
}

/// `_matrix`: features × samples; a missing value (null / negative) is 0 (not detected). Rows of different length are an error.
pub fn clean_matrix(rows: &[Vec<Option<f64>>]) -> R<Vec<Vec<f64>>> {
    let w = rows.first().map_or(0, Vec::len);
    if rows.iter().any(|r| r.len() != w) {
        return bad("the rows of the table must have the same length");
    }
    Ok(rows.iter().map(|r| r.iter().map(|v| v.filter(|x| *x == *x).unwrap_or(0.0).max(0.0)).collect()).collect())
}

pub fn ncols(m: &[Vec<f64>]) -> usize {
    m.first().map_or(0, Vec::len)
}

/// `a.mean()` of a contiguous float64 array (pairwise sum, like numpy).
pub fn mean(a: &[f64]) -> f64 {
    psum(a) / a.len() as f64
}

fn pick(row: &[f64], cols: &[usize]) -> Vec<f64> {
    cols.iter().map(|&c| row[c]).collect()
}

// ------------------------------------------------------------------------------------------------ time profile of one ion
fn number(v: Option<&Value>) -> R<Option<f64>> {
    match v {
        None | Some(Value::Null) => Ok(None),
        Some(Value::Num(n)) => Ok(Some(*n)),
        Some(_) => bad("a number was expected"),
    }
}

/// `time_profile`: the points in the order of their time, normalised on an internal standard when `istd` is given (file → {area, height}).
pub fn time_profile(points: &[Value], metric: &str, istd: Option<&Value>) -> R<Value> {
    if metric != "area" && metric != "height" {
        return user("err.stat.metric", "metric must be 'area' or 'height'");
    }
    let mut rows: Vec<(Option<f64>, String, usize, Value)> = Vec::new();
    for (i, p) in points.iter().enumerate() {
        let file = p.get("file").and_then(Value::str).map(String::from);
        let Some(file) = file else { return bad("missing or invalid parameter: 'file'") };
        let v = number(p.get(metric))?;
        let mut norm = None;
        if let (Some(s), Some(v)) = (istd, v) {
            let std = s.get(&file).and_then(|o| o.get(metric)).and_then(Value::f64).filter(|x| *x != 0.0);
            norm = std.map(|d| v / d);
        }
        let t = number(p.get("time"))?;
        let row = Obj::new().put("file", text(&file)).put("time", opt(t)).put("rt", p.get("rt").cloned().unwrap_or(Value::Null))
            .put("area", p.get("area").cloned().unwrap_or(Value::Null)).put("height", p.get("height").cloned().unwrap_or(Value::Null))
            .put("value", opt(v)).put("norm", opt(norm)).done();
        rows.push((t, file, i, row));
    }
    rows.sort_by(|a, b| {
        let by_time = a.0.unwrap_or(0.0).partial_cmp(&b.0.unwrap_or(0.0)).unwrap_or(Equal);
        a.0.is_none().cmp(&b.0.is_none()).then(by_time).then_with(|| if a.0.is_some() { a.1.cmp(&b.1) } else { Equal }).then(a.2.cmp(&b.2))
    });
    let field = if istd.is_some() { "norm" } else { "value" };
    let series: Vec<f64> = rows.iter().enumerate().filter(|(_, r)| r.0.is_some() && !r.3.get(field).map_or(true, Value::is_null)).map(|(i, _)| i as f64).collect();
    let out: Vec<Value> = rows.into_iter().map(|r| r.3).collect();
    Ok(Obj::new().put("metric", text(metric)).b("normalized", istd.is_some()).put("rows", Value::Arr(out)).put("series", floats(&series)).done())
}

// ------------------------------------------------------------------------------------------------ blank subtraction of a feature table
pub struct BlankRow {
    pub blank: f64,
    pub threshold: f64,
    pub n_above: usize,
    pub n_samples: usize,
    pub max_sample: f64,
}

pub struct BlankOut {
    pub kept: Vec<usize>,
    pub discarded: Vec<usize>,
    pub rows: Vec<BlankRow>,
    pub n_blank: usize,
    pub n_sample: usize,
}

/// `blank_filter`: a feature is kept when at least `min_detect` sample files are above (blank level × factor) and above 0.
pub fn blank_filter(m: &[Vec<f64>], roles: &[String], factor: f64, stat: &str, min_detect: i64) -> R<BlankOut> {
    if stat != "max" && stat != "mean" {
        return user("err.stat.blank.stat", "stat must be 'max' or 'mean'");
    }
    if !(factor >= 0.0) || min_detect < 1 {
        return user("err.stat.blank.param", "factor must be >= 0 and the minimum of detections >= 1");
    }
    let bl: Vec<usize> = (0..roles.len()).filter(|&i| roles[i] == "blank").collect();
    let sm: Vec<usize> = (0..roles.len()).filter(|&i| roles[i] != "blank" && roles[i] != "dark").collect();
    if bl.is_empty() {
        return user("err.stat.blank.none", "no file is marked as a blank");
    }
    if sm.is_empty() {
        return user("err.stat.blank.nosample", "no sample file");
    }
    if bl.iter().chain(&sm).any(|&c| c >= ncols(m)) {
        return bad("index out of bounds: the table has fewer columns than files");
    }
    let (mut kept, mut discarded, mut rows) = (Vec::new(), Vec::new(), Vec::new());
    for (f, row) in m.iter().enumerate() {
        let b = pick(row, &bl);
        let lvl = if stat == "max" { b.iter().cloned().fold(f64::NEG_INFINITY, f64::max) } else { mean(&b) };
        let thr = lvl * factor;
        let s = pick(row, &sm);
        let n_above = s.iter().filter(|&&v| v > thr && v > 0.0).count();
        if n_above as i64 >= min_detect { kept.push(f) } else { discarded.push(f) }
        rows.push(BlankRow { blank: lvl, threshold: thr, n_above, n_samples: sm.len(), max_sample: s.iter().cloned().fold(f64::NEG_INFINITY, f64::max) });
    }
    Ok(BlankOut { kept, discarded, rows, n_blank: bl.len(), n_sample: sm.len() })
}

/// The JSON of `blank_filter` (what `/api/stat/blank` answers).
pub fn blank_value(b: &BlankOut, stat: &str, factor: f64, min_detect: i64) -> Value {
    let idx = |v: &[usize]| Value::Arr(v.iter().map(|&i| int(i as i64)).collect());
    let rows = b.rows.iter().map(|r| {
        Obj::new().n("blank", r.blank).n("threshold", r.threshold).i("n_above", r.n_above as i64).i("n_samples", r.n_samples as i64).n("max_sample", r.max_sample).done()
    });
    Obj::new().put("kept", idx(&b.kept)).put("discarded", idx(&b.discarded)).put("rows", Value::Arr(rows.collect())).put("stat", text(stat))
        .put("factor", num(factor)).i("min_detect", min_detect).i("n_blank", b.n_blank as i64).i("n_sample", b.n_sample as i64).done()
}
