//! `qc.mass_accuracy@1` natively: the mass error of every file measured on the lab background ions, the internal standards and the standards of the user (`massacc::run`), and the
//! feature table with its m/z corrected when a model is chosen. Raw data and indexes are only read. Called by `flow.rs`; the browser runs the same `massacc::run` in the `table` module.

use crate::flow::{all_inputs, io, read_one, Exec, FileRunner, Made};
use mzlab_core::error::{Error, Result};
use mzlab_core::flow::Job;
use mzlab_core::json::Value;
use mzlab_core::step::{DataType, Param, Port, RunsIn, Step};
use mzlab_feat::massacc::{self, Params};
use mzlab_feat::store::Slice;

/// The lab list of background ions (one file for the page and the program).
const CONTAMINANTS: &[u8] = include_bytes!("../../../mzlab/web/contaminanti.json");
const CEILING: u64 = 737_280;

pub(crate) const EXECUTORS: &[(&str, Exec)] = &[("qc.mass_accuracy", Exec::Made(mass_job))];

/// The descriptor of `qc.mass_accuracy@1`.
pub fn massacc_step() -> Step {
    let d = Params::default();
    let optional = |p: Port| Port {
        optional: true,
        ..p
    };
    Step {
        ins: vec![
            Port::new("raw", DataType::Mzidx, "Scan index of each file (.mzidx)").many(),
            optional(Port::new("table", DataType::Table, "Feature table (.mzfeat) whose m/z is corrected when a model is chosen")),
            optional(Port::new("samples", DataType::Samples, "Roles of the files; the files marked as internal standard files are checked for their standards")),
            optional(Port::new("references", DataType::Report, "CSV of reference ions: name, ion (formula with its charge or adduct, or m/z), optional RT window, role (reference, istd or standard)")),
        ],
        outs: vec![
            Port::new("massacc", DataType::Qc, "Mass accuracy of each file (massacc/1)"),
            Port::new("table", DataType::Table, "Feature table with the m/z corrected (only with a model)"),
        ],
        params: vec![
            Param::number("search_ppm", "Search window around each reference ion").default_num(d.search_ppm).min(1.0).unit("ppm"),
            Param::number("min_intensity", "Lowest peak intensity taken for a reference").default_num(d.min_intensity).min(0.0).unit("counts"),
            Param::integer("min_refs", "References a file needs to be assessed").default_num(d.min_refs as f64).min(1.0),
            Param::choice("robust", "How outlying references are handled", &["huber", "ransac", "none"]).default_str(d.robust.name()),
            Param::choice("model", "Correction of the table m/z", &["none", "offset", "linear_mz", "linear_mz_rt"]).default_str(d.model.name()),
            Param::number("warn_offset_ppm", "Offset above which the file is flagged").default_num(d.warn_offset_ppm).min(0.0).unit("ppm").advanced(),
            Param::number("warn_spread_ppm", "Spread above which the file is flagged").default_num(d.warn_spread_ppm).min(0.0).unit("ppm").advanced(),
            Param::number("warn_is_rt_min", "RT shift limit of internal standards").default_num(d.warn_is_rt_min).min(0.0).unit("min").advanced(),
            Param::number("warn_is_area_rsd", "Area spread limit of internal standards").default_num(d.warn_is_area_rsd).min(0.0).advanced(),
        ],
        runs_in: RunsIn {
            wasm: Some("table"),
            wasm_ceiling_bytes: Some(CEILING),
            native: true,
        },
        errors: vec!["err.flow.massacc.references", "err.flow.massacc.mrm", "err.flow.param.type", "error.badMzfeat"],
        ..Step::core(
            "qc.mass_accuracy",
            "Mass accuracy",
            "Measures the mass error of each file on ions that are known to be there (the lab background list, internal standards and standards of the user) and reports offset, spread, standards and warnings; with a model it also writes a copy of the feature table with its m/z corrected. Numbers only, no verdict.",
        )
    }
}

/// The parameters of the job, with the defaults of the descriptor for the missing ones.
pub fn params_of(job: &Job) -> Result<Params> {
    Params::from_value(&Value::Obj(job.resolved.params.clone()))
}

fn mass_job(r: &FileRunner, job: &Job) -> Result<Made> {
    let p = params_of(job)?;
    let has = |port: &str| job.inputs.iter().any(|(n, v)| n == port && !v.is_empty());
    let raw = job
        .inputs
        .iter()
        .find(|(n, _)| n == "raw")
        .map(|(_, v)| v.as_slice())
        .unwrap_or_default();
    let paths = all_inputs(r, job, "raw");
    let table = if has("table") {
        Some(read_one(r, job, "table")?.1)
    } else {
        None
    };
    let samples = if has("samples") {
        Some(read_one(r, job, "samples")?.1)
    } else {
        None
    };
    let csv = if has("references") {
        Some(read_one(r, job, "references")?.1)
    } else {
        None
    };
    let names: Vec<(String, String)> = raw
        .iter()
        .map(|a| (r.source_name(a), String::new()))
        .collect();
    let mut open = |k: usize| match paths.get(k) {
        Some(path) => crate::flow_base::open(path),
        None => Err(Error::new("err.flow.port.missing").with("port", "raw")),
    };
    let mut slice = table.as_deref().map(Slice);
    let table_in = slice
        .as_mut()
        .zip(table.as_ref())
        .map(|(s, t)| (s, t.len() as u64));
    let out = massacc::check(
        &mut open,
        &names,
        samples.as_deref(),
        CONTAMINANTS,
        csv.as_deref(),
        table_in,
        &p,
    )?;
    let json = r.work.join(format!("{}.json", job.instance.id));
    std::fs::write(&json, &out.json).map_err(io)?;
    let mut made = Made::new("massacc", r.keep(json, DataType::Qc)?);
    made.warnings = out.warnings;
    if let Some(bytes) = out.table {
        let path = r.work.join(format!("{}.mzfeat", job.instance.id));
        std::fs::write(&path, bytes).map_err(io)?;
        made.extra = Some(("table", r.keep(path, DataType::Table)?));
    }
    Ok(made)
}
