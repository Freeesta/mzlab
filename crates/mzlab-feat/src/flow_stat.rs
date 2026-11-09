//! The texts of `table.stats@1` (`stat/1`: Welch, ANOVA, BH), `table.pca@1` (the answer of the PCA engine `api/stat/pca`) and `table.kmd@1` (Kendrick mass defect
//! and series). Bytes in, text out: `flow run` (`mzlab-cli`) and the browser module `flowtab` (`mzlab-feat-wasm`) both call them, so they write the same bytes.
//! Columns are matched to the samples file by name, else by SHA-256 (`samples::columns`).

use crate::samples::{columns, samples_of, Sample};
use crate::stat::{
    compare, comparisons_from_design, kmd_json, layout_from_design, pca_json, write_stats,
    StatsOpts,
};
use crate::store::{origin_from_meta, read_table};
use mzlab_core::drawing;
use mzlab_core::error::{Error, Result};
use mzlab_core::json::{parse, Out, Value};

fn number(params: &[(String, Value)], name: &str) -> Option<f64> {
    let found = params.iter().find(|(k, _)| k == name);
    found.and_then(|(_, v)| v.f64())
}

fn text(params: &[(String, Value)], name: &str) -> Option<String> {
    let found = params.iter().find(|(k, _)| k == name);
    found.and_then(|(_, v)| v.str()).map(String::from)
}

/// The `stat/1` text of the comparisons of `design` on `table`.
pub fn stats_text(
    params: &[(String, Value)],
    table: &[u8],
    samples: &[u8],
    design: &[u8],
) -> Result<String> {
    let (t, meta) = read_table(table)?;
    let inputs = origin_from_meta(&meta).inputs;
    if inputs.len() != t.n_files {
        return Err(Error::new("error.badStatInput"));
    }
    let cols = columns(&inputs, &samples_of(samples)?);
    let design = drawing::from_json(design)?;
    let lay = layout_from_design(&cols, &design);
    let pairs = comparisons_from_design(&design, &lay)?;
    let matrix: Vec<Vec<f64>> = (0..t.len())
        .map(|r| t.area[r * t.n_files..(r + 1) * t.n_files].to_vec())
        .collect();
    let defaults = StatsOpts::default();
    let opts = StatsOpts {
        pseudo: number(params, "pseudocount"),
        floor: number(params, "floor").unwrap_or(defaults.floor),
    };
    let out = compare(&matrix, &lay, &pairs, &opts)?;
    Ok(write_stats(&out, &lay))
}

/// The role the page gives a file (`qaqc.js`, `FROM_CAMPIONI`); a file the samples do not know is a sample.
fn page_role(kind: &str) -> &'static str {
    match kind {
        "blank" => "blank",
        "dark" => "dark",
        "qc" => "qc",
        "std" | "calibration" => "standard",
        _ => "sample",
    }
}

/// The replicate group of the page (`repKey`): files of the same role, time and concentration; a file with neither time nor concentration is alone.
fn group_of(s: &Sample, role: &str) -> String {
    if s.time_min.is_none() && s.conc.is_none() {
        return format!("#{}", s.file);
    }
    let time = s.time_min.map(|t| t.to_string()).unwrap_or_default();
    let conc = s.conc.map(|c| c.to_string()).unwrap_or_default();
    let unit = match (s.conc, &s.unit) {
        (Some(_), Some(u)) => format!("|{u}"),
        (Some(_), None) => "|".to_string(),
        _ => String::new(),
    };
    format!("{role}|{time}|{conc}{unit}")
}

/// The engine's error text (`{error, error_key, params}`) as a keyed error.
fn engine_error(body: &str) -> Error {
    let doc = parse(body.as_bytes()).ok();
    let key = doc.as_ref().and_then(|d| d.get("error_key")?.str());
    let known = [
        "err.stat.pca.few",
        "err.stat.pca.nofeatures",
        "err.stat.pca.param",
        "err.stat.blank.param",
    ];
    let key = known.into_iter().find(|k| Some(*k) == key);
    Error::new(key.unwrap_or("error.badStatInput")).with("detail", body)
}

/// The answer of the PCA engine for the areas of `table` and the roles of `samples`.
pub fn pca_text(params: &[(String, Value)], table: &[u8], samples: &[u8]) -> Result<String> {
    let (t, meta) = read_table(table)?;
    let inputs = origin_from_meta(&meta).inputs;
    if inputs.len() != t.n_files {
        return Err(Error::new("error.badStatInput"));
    }
    let cols = columns(&inputs, &samples_of(samples)?);
    let roles: Vec<&str> = cols.iter().map(|s| page_role(&s.kind)).collect();
    let mut o = Out::default();
    o.raw("{\"matrix\":[");
    for r in 0..t.len() {
        o.raw(if r > 0 { "," } else { "" }).raw("[");
        for (f, a) in t.area[r * t.n_files..(r + 1) * t.n_files]
            .iter()
            .enumerate()
        {
            o.raw(if f > 0 { "," } else { "" });
            o.num(if a.is_finite() && *a > 0.0 { *a } else { 0.0 });
        }
        o.raw("]");
    }
    o.raw("],\"roles\":[");
    for (i, r) in roles.iter().enumerate() {
        o.raw(if i > 0 { "," } else { "" }).str(r);
    }
    o.raw("],\"groups\":[");
    for (i, (s, r)) in cols.iter().zip(&roles).enumerate() {
        o.raw(if i > 0 { "," } else { "" }).str(&group_of(s, r));
    }
    o.raw("]");
    for k in [
        "factor",
        "min_detect",
        "min_intensity",
        "min_samples",
        "ncomp",
    ] {
        if let Some(v) = number(params, k) {
            o.raw(",").str(k).raw(":").num(v);
        }
    }
    for k in ["stat", "transform", "scaling"] {
        if let Some(v) = text(params, k) {
            o.raw(",").str(k).raw(":").str(&v);
        }
    }
    o.raw("}");
    pca_json(o.0.as_bytes()).map_err(|e| engine_error(&e))
}

/// The `kmd` text of the rows of `table` under the parameters of the job (`unit`, `tol_mda`, `min_members`).
pub fn kmd_text(params: &[(String, Value)], table: &[u8]) -> Result<String> {
    let (t, _) = read_table(table)?;
    let mut o = Out::default();
    o.raw("{\"features\":[");
    for (i, (mz, rt)) in t.mz.iter().zip(&t.rt).enumerate() {
        o.raw(if i > 0 { "," } else { "" }).raw("{\"mz\":").num(*mz);
        o.raw(",\"rt\":").num(*rt).raw("}");
    }
    o.raw("]");
    for (k, key) in [("tol_mda", "tolMDa"), ("min_members", "minMembers")] {
        let found = params.iter().find(|(n, _)| n == k);
        if let Some(v) = found.and_then(|(_, v)| v.f64()) {
            o.raw(",\"").raw(key).raw("\":").num(v);
        }
    }
    let unit = params.iter().find(|(n, _)| n == "unit");
    if let Some(u) = unit.and_then(|(_, v)| v.str()) {
        o.raw(",\"unit\":").str(u);
    }
    o.raw("}");
    kmd_json(o.0.as_bytes())
}
