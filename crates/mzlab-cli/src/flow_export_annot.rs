//! The native executor of `export.annotations@1`: the file wrapper of `mzlab_feat::flow_export_annot` (the one implementation the browser runs too). It reads the table and the
//! upstream files by name and writes the CSV; rows that cannot be joined to a feature are counted on stderr. Called by `flow.rs`.

use crate::flow::{io, read_all, read_one, Exec, FileRunner};
use mzlab_core::error::Result;
use mzlab_core::flow::{Artifact, Job};
use mzlab_core::step::DataType;
use mzlab_feat::flow_export_annot::{export_made, Inputs, Params};

pub(crate) const EXECUTORS: &[(&str, Exec)] =
    &[("export.annotations", Exec::File("report", annotations_job))];

/// The files of an optional port (none when it is not wired).
fn port(r: &FileRunner, job: &Job, name: &str) -> Result<Vec<(String, Vec<u8>)>> {
    if job.inputs.iter().any(|(p, v)| p == name && !v.is_empty()) {
        read_all(r, job, name)
    } else {
        Ok(Vec::new())
    }
}

fn named(v: &[(String, Vec<u8>)]) -> Vec<(&str, &[u8])> {
    v.iter().map(|f| (f.0.as_str(), f.1.as_slice())).collect()
}

fn annotations_job(r: &FileRunner, job: &Job) -> Result<Artifact> {
    let table = read_one(r, job, "table")?;
    let (c, s, f) = (
        port(r, job, "candidates")?,
        port(r, job, "screen")?,
        port(r, job, "formulas")?,
    );
    let a = port(r, job, "annot")?;
    let (c, s, f) = (named(&c), named(&s), named(&f));
    let inputs = Inputs {
        table: &table.1,
        candidates: &c,
        screen: &s,
        formulas: &f,
        annot: named(&a).first().copied(),
    };
    let made = export_made(&Params::from_params(&job.resolved.params), &inputs)?;
    if made.unjoined > 0 {
        eprintln!(
            "warning: export.annotations: {} rows could not be joined to a feature (empty feature_id, query id in source_file)",
            made.unjoined
        );
    }
    let out = r.work.join(format!("{}.csv", job.instance.id));
    std::fs::write(&out, made.csv).map_err(io)?;
    r.keep(out, DataType::Report)
}

#[cfg(test)]
#[path = "flow_export_annot_tests.rs"]
mod tests;
