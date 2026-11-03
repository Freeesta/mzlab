//! The native executor of `chrom.transitions@1`: the transition table integrated in each mzML (or its candidate peaks), as one CSV. The text is
//! `mzlab_feat::transitions::transitions_text`, shared with the browser; this file only reads and writes the files. Called by `flow.rs`.

use mzlab_core::error::Result;
use mzlab_core::flow::{Artifact, Job};
use mzlab_core::step::DataType;
use mzlab_feat::transitions::transitions_text;

use crate::flow::{io, read_all, Exec, FileRunner};
use crate::flow_read::named;

pub(crate) const EXECUTORS: &[(&str, Exec)] =
    &[("chrom.transitions", Exec::File("peaks", transitions_job))];

fn transitions_job(r: &FileRunner, job: &Job) -> Result<Artifact> {
    let files = read_all(r, job, "source")?;
    let params = &job.resolved.params;
    let table = params
        .iter()
        .find(|(k, _)| k == "transitions")
        .and_then(|(_, v)| v.str())
        .unwrap_or("");
    let text = transitions_text(params, table, &named(&files))?;
    let out = r.work.join(format!("{}.csv", job.instance.id));
    std::fs::write(&out, text).map_err(io)?;
    r.keep(out, DataType::Report)
}

#[cfg(test)]
#[path = "flow_transitions_tests.rs"]
mod tests;
