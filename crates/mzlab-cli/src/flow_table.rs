//! The native executor of `table.filter@1`: the view (`view_json`, the one the page asks of the table `.wasm`) of a `.mzfeat` under the rules the
//! parameters of the step spell, in the language of the engine (`filtra`/`normalizza`). The roles come from the samples file, `campioni/1` or `campioni/2`.

use crate::flow::{read_one, write_made, FileRunner};
use mzlab_core::campioni;
use mzlab_core::error::Result;
use mzlab_core::flow::Artifact;
use mzlab_core::flow::Job;
use mzlab_core::json::{parse, Out};
use mzlab_core::step::DataType;
use mzlab_feat::filters::QC_MIN_FILES;
use mzlab_feat::samples::{self, Sample};
use mzlab_feat::store::Slice;
use mzlab_feat_table_wasm::core::view_json;

fn param<'a>(job: &'a Job, name: &str) -> Option<&'a mzlab_core::json::Value> {
    job.resolved
        .params
        .iter()
        .find(|(k, _)| k == name)
        .map(|(_, v)| v)
}

fn number(job: &Job, name: &str) -> Option<f64> {
    param(job, name).and_then(|v| v.f64())
}

/// `{"normalizza":..., "filtra":[...]}`: the rules in the order of the descriptor, each one only when its parameter is set.
pub fn spec_text(job: &Job) -> String {
    let mut rules: Vec<String> = Vec::new();
    let mut rule = |name: &str, value: Option<f64>, extra: &str| {
        if let Some(v) = value {
            let mut o = Out::default();
            o.raw("{\"regola\":").str(name).raw(",\"valore\":").num(v);
            rules.push(format!("{}{extra}}}", o.0));
        }
    };
    rule("min_rilevati", number(job, "min_detected"), "");
    rule("min_area", number(job, "min_area"), "");
    rule("min_altezza", number(job, "min_height"), "");
    rule("bianco", number(job, "blank_ratio"), "");
    let qc_files = number(job, "qc_min_files").unwrap_or(QC_MIN_FILES as f64);
    let extra = format!(",\"qc_min_files\":{qc_files}");
    rule("qc_cv_max", number(job, "qc_cv_max"), &extra);
    if param(job, "require_ms2").and_then(|v| v.bool()) == Some(true) {
        rules.push("{\"regola\":\"ms2\"}".to_string());
    }
    let method = match param(job, "normalize").and_then(|v| v.str()) {
        Some("sum") => "somma",
        Some("median") => "mediana",
        _ => "nessuna",
    };
    format!(
        "{{\"normalizza\":{{\"metodo\":\"{method}\"}},\"filtra\":[{}]}}",
        rules.join(",")
    )
}

/// The samples as `campioni/1` text, whichever of the two schemas the file has.
pub(crate) fn samples_text(bytes: &[u8]) -> Result<String> {
    let doc = parse(bytes)?;
    if doc.get("schema").and_then(|v| v.str()) != Some("campioni/2") {
        return Ok(String::from_utf8_lossy(bytes).into_owned());
    }
    let mut out: Vec<Sample> = Vec::new();
    for c in campioni::from_json(bytes)? {
        let kind = match c.kind.as_str() {
            "infusion" => campioni::tipo_v1(&c.file).to_string(),
            k => k.to_string(),
        };
        let group = if matches!(kind.as_str(), "sample" | "std" | "calibration") {
            String::new()
        } else {
            kind.clone()
        };
        out.push(Sample {
            file: c.file,
            sha256: c.sha256,
            kind,
            time_min: c.time_min,
            replicate: c.replicate,
            group,
            conc: c.conc,
            unit: c.unit,
            order: c.order,
            istd: c.istd,
            note: c.note,
        });
    }
    Ok(samples::to_json(&out))
}

/// The `view` text of the table under the rules of the job.
pub fn filter_text(job: &Job, table: &[u8], samples: &[u8]) -> Result<String> {
    let samples = samples_text(samples)?;
    let spec = spec_text(job);
    view_json(&mut Slice(table), table.len() as u64, &samples, &spec)
}

pub(crate) fn filter_job(r: &FileRunner, job: &Job) -> Result<Artifact> {
    let (table, samples) = (read_one(r, job, "table")?, read_one(r, job, "samples")?);
    let text = filter_text(job, &table.1, &samples.1)?;
    write_made(r, job, text, DataType::Table)
}
