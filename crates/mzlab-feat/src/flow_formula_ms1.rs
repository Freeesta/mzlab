//! The text of `formula.ms1@1`: candidate formulas of each feature from the MS1 scan at the apex, in the file where the feature is largest (`untargeted::formula_id`). Takes names,
//! bytes and the parameters and returns the text: `flow run` (`mzlab-cli`) and the browser (`mzlab-feat-wasm`) run these functions. The browser reads one mzML per call (`ms1_rows`) and
//! joins the rows in the order of the table (`ms1_join`); `ms1_text` does the same over all the files.

use mzlab_core::error::{Error, Result};
use mzlab_core::json::{Out, Value};
use mzlab_core::mzml::Run;

use crate::flow_ms2::{num, read_table, Named, Table};
use crate::untargeted::formula_id::{identify, to_json, Opts};

fn param<'a>(params: &'a [(String, Value)], name: &str) -> Option<&'a Value> {
    params.iter().find(|(k, _)| k == name).map(|(_, v)| v)
}

fn opts(params: &[(String, Value)]) -> Opts {
    let mut o = Opts::default();
    if let Some(s) = param(params, "ion").and_then(Value::str) {
        o.ion = s.to_string();
    }
    o.z = param(params, "z").and_then(Value::f64).map(|z| z as i32);
    o.tol_ppm = num(params, "tol_ppm");
    o.elements = param(params, "elements")
        .and_then(Value::str)
        .map(String::from);
    if let Some(s) = param(params, "nitrogen").and_then(Value::str) {
        o.nitrogen = s.to_string();
    }
    o.rdb_min = num(params, "rdb_min");
    o.max_results = num(params, "max_results").max(1.0) as usize;
    o.finder.max_charge = num(params, "max_charge").max(1.0) as i32;
    o.finder.require_c13 = param(params, "require_c13")
        .and_then(Value::bool)
        .unwrap_or(true);
    o
}

fn stem(name: &str) -> &str {
    name.rsplit_once('.').map_or(name, |p| p.0)
}

/// The MS1 scan of `run` closest in RT to `rt` whose polarity is `sign` (or not said).
fn nearest_ms1(run: &Run, rt: f64, sign: i8) -> Option<(usize, Vec<f64>, Vec<f64>)> {
    let s = run
        .spectra
        .iter()
        .filter(|s| s.level == 1 && (s.polarity == 0 || s.polarity == sign))
        .min_by(|a, b| {
            (a.rt - rt)
                .abs()
                .total_cmp(&(b.rt - rt).abs())
                .then(a.index.cmp(&b.index))
        })?;
    Some((
        s.index,
        s.mz.clone(),
        s.intensity.iter().map(|v| *v as f64).collect(),
    ))
}

/// For each row of the table the index in `names` of the file it reads: the one where its area is largest, or the only file.
fn file_of_rows(t: &Table, names: &[&str]) -> Result<Vec<usize>> {
    t.rows
        .iter()
        .map(|row| {
            let area = |k: usize| {
                row.get(k)
                    .map(String::as_str)
                    .unwrap_or("")
                    .parse::<f64>()
                    .unwrap_or(0.0)
            };
            let best = t.areas.iter().fold(None, |b: Option<&(String, usize)>, a| {
                if b.is_none_or(|b| area(a.1) > area(b.1)) {
                    Some(a)
                } else {
                    b
                }
            });
            best.and_then(|b| names.iter().position(|n| stem(n) == b.0))
                .or(if names.len() == 1 { Some(0) } else { None })
                .ok_or_else(|| Error::new("err.flow.formula.file"))
        })
        .collect()
}

/// The object of one feature of the output, from the MS1 scan of its file.
fn row_text(o: &Opts, t: &Table, n: usize, name: &str, run: &Run) -> Result<String> {
    let row = &t.rows[n];
    let cell = |k: usize| row.get(k).map(String::as_str).unwrap_or("");
    let (mz, rt) = (
        cell(t.mz).parse::<f64>().unwrap_or(f64::NAN),
        cell(t.rt).parse::<f64>().unwrap_or(f64::NAN),
    );
    let sign = if o.ion.ends_with('-') { -1 } else { 1 };
    let mut out = Out::default();
    out.raw("{\"id\":")
        .str(cell(t.id))
        .raw(",\"mz\":")
        .num(mz)
        .raw(",\"rt\":")
        .num(rt);
    out.raw(",\"file\":").str(stem(name));
    match nearest_ms1(run, rt, sign) {
        None => {
            out.raw(",\"error\":\"err.flow.formula.scan\"}");
        }
        Some((scan, m, y)) => {
            out.raw(",\"scan\":").int(scan as i64);
            match identify(&m, &y, mz, o) {
                Ok(r) => {
                    out.raw(",\"result\":").raw(&to_json(&r)).raw("}");
                }
                // a wrong parameter stops the step; a feature the search cannot treat is recorded
                Err(e) if e.key == "err.formulaId.param" || e.key == "err.comp.ion" => {
                    return Err(e)
                }
                Err(e) => {
                    out.raw(",\"error\":").str(e.key).raw("}");
                }
            }
        }
    }
    Ok(out.0)
}

/// The objects of the features that read the file `file.0` of `names` (`file.1` are its bytes), with their position in the table. A file no feature reads is not parsed.
pub fn ms1_rows(
    params: &[(String, Value)],
    features: &[u8],
    names: &[&str],
    file: (usize, &[u8]),
) -> Result<Vec<(usize, String)>> {
    let t = read_table(features)?;
    let (o, mine) = (opts(params), file_of_rows(&t, names)?);
    let rows: Vec<usize> = (0..mine.len()).filter(|n| mine[*n] == file.0).collect();
    if rows.is_empty() {
        return Ok(Vec::new());
    }
    let run = mzlab_core::mzml::read(file.1)?;
    rows.into_iter()
        .map(|n| Ok((n, row_text(&o, &t, n, names[file.0], &run)?)))
        .collect()
}

/// The `formulas/1` text from the objects of the features in the order of the table.
pub fn ms1_join(rows: &[String]) -> String {
    let mut out = Out::default();
    out.raw("{\"format\":\"formulas/1\",\"step\":\"formula.ms1@1\",\"algorithm\":\"compose_isotopes\",\"features\":[");
    out.raw(&rows.join(",")).raw("]}");
    out.0
}

/// `formula.ms1`: for each feature the MS1 scan at the apex, in the file where the feature is largest, goes to `formula_id::identify`.
pub fn ms1_text(params: &[(String, Value)], features: &[u8], files: &[Named]) -> Result<String> {
    let names: Vec<&str> = files.iter().map(|f| f.0).collect();
    let t = read_table(features)?;
    file_of_rows(&t, &names)?;
    let mut rows = vec![String::new(); t.rows.len()];
    for (k, f) in files.iter().enumerate() {
        for (n, s) in ms1_rows(params, features, &names, (k, f.1))? {
            rows[n] = s;
        }
    }
    Ok(ms1_join(&rows))
}
