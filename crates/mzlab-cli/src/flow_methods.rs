//! The native executor of `report.methods@1`: the methods text of a `corsa/1` run record. The text is `mzlab_feat::methods::methods_text`, shared with the browser; this file only reads and
//! writes the files. Called by `flow.rs`.

use crate::flow::{io, read_one, Exec, FileRunner};
use mzlab_core::error::Result;
use mzlab_core::flow::{Artifact, Job};
use mzlab_core::step::DataType;
use mzlab_feat::methods::methods_text;

pub(crate) const EXECUTORS: &[(&str, Exec)] =
    &[("report.methods", Exec::File("methods", methods_job))];

fn methods_job(r: &FileRunner, job: &Job) -> Result<Artifact> {
    let run = read_one(r, job, "run")?;
    let text = methods_text(&job.resolved.params, &run.1)?;
    let out = r.work.join(format!("{}.md", job.instance.id));
    std::fs::write(&out, text).map_err(io)?;
    r.keep(out, DataType::Report)
}
