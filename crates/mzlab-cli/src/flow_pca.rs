//! The native executor of `table.pca@1`: the exploratory PCA of the areas of a `.mzfeat` (`stat::pca_json`, the engine of the route `api/stat/pca`), asked
//! as the Pca page asks: roles and replicate groups of the files from the samples file, matched to the columns by name, else by SHA-256. Called by `flow.rs`.

use crate::flow::{read_one, write_made, FileRunner};
use crate::flow_stats::{columns, samples_of};
use mzlab_core::error::{Error, Result};
use mzlab_core::flow::{Artifact, Job};
use mzlab_core::json::{parse, Out};
use mzlab_core::step::DataType;
use mzlab_feat::samples::Sample;
use mzlab_feat::stat::pca_json;
use mzlab_feat::store::{origin_from_meta, read_table};

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

fn number(job: &Job, name: &str) -> Option<f64> {
    let found = job.resolved.params.iter().find(|(k, _)| k == name);
    found.and_then(|(_, v)| v.f64())
}

fn text(job: &Job, name: &str) -> Option<String> {
    let found = job.resolved.params.iter().find(|(k, _)| k == name);
    found.and_then(|(_, v)| v.str()).map(String::from)
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
pub fn pca_text(job: &Job, table: &[u8], samples: &[u8]) -> Result<String> {
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
        if let Some(v) = number(job, k) {
            o.raw(",").str(k).raw(":").num(v);
        }
    }
    for k in ["stat", "transform", "scaling"] {
        if let Some(v) = text(job, k) {
            o.raw(",").str(k).raw(":").str(&v);
        }
    }
    o.raw("}");
    pca_json(o.0.as_bytes()).map_err(|e| engine_error(&e))
}

pub(crate) fn pca_job(r: &FileRunner, job: &Job) -> Result<Artifact> {
    let (table, samples) = (read_one(r, job, "table")?, read_one(r, job, "samples")?);
    let text = pca_text(job, &table.1, &samples.1)?;
    write_made(r, job, text, DataType::Report)
}
