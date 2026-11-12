//! The native executor of `structure.moiety@1`: the rows of a candidate table that contain (or lack) a moiety. The table is
//! `mzlab_feat::chem::moiety::moiety_text`, shared with the browser (`molkit`, `flow_moiety`); this file only reads and writes the files. Called by `flow.rs`.

use crate::flow::{io, read_one, Exec, FileRunner};
use mzlab_core::error::Result;
use mzlab_core::flow::{Artifact, Job};
use mzlab_core::json::Value;
use mzlab_core::step::DataType;
use mzlab_feat::chem::moiety::moiety_text;

pub(crate) const EXECUTORS: &[(&str, Exec)] =
    &[("structure.moiety", Exec::File("candidates", moiety_job))];

fn moiety_job(r: &FileRunner, job: &Job) -> Result<Artifact> {
    let table = read_one(r, job, "candidates")?;
    let text = moiety_text(&Value::Obj(job.resolved.params.clone()), &table.1)?;
    let out = r.work.join(format!("{}.csv", job.instance.id));
    std::fs::write(&out, text).map_err(io)?;
    r.keep(out, DataType::Candidates)
}
