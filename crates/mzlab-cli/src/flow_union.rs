//! The native executor of `features.union@1`: the `.mzfeat` tables of the series joined into one target list and its `union/1` document
//! (`mzlab_feat::union::union_text`, the function the `flowtab` `.wasm` calls in the browser). This file only reads and writes the files.
//! Called by `flow.rs`.

use crate::flow::{all_inputs, io, Exec, FileRunner};
use crate::flow_made::Made;
use mzlab_core::error::Result;
use mzlab_core::flow::Job;
use mzlab_core::step::DataType;

/// The step of this file, chained by `flow::all_executors`.
pub(crate) const EXECUTORS: &[(&str, Exec)] = &[("features.union", Exec::Made(union_job))];

fn union_job(r: &FileRunner, job: &Job) -> Result<Made> {
    // a series is named by the file of its table (`<id>.mzfeat`), the name the page gives it too
    let tables = all_inputs(r, job, "table")
        .into_iter()
        .map(|p| {
            let name = p.file_name().map_or(String::new(), |n| n.to_string_lossy().into_owned());
            Ok((name, std::fs::read(&p).map_err(io)?))
        })
        .collect::<Result<Vec<_>>>()?;
    let u = mzlab_feat::union::union_text(&job.resolved.params, &tables)?;
    let csv = r.work.join(format!("{}.csv", job.instance.id));
    std::fs::write(&csv, &u.csv).map_err(io)?;
    let doc = r.work.join(format!("{}.union.json", job.instance.id));
    std::fs::write(&doc, &u.doc).map_err(io)?;
    let mut made = Made::new("targets", r.keep(csv, DataType::Report)?);
    made.extra = Some(("union", r.keep(doc, DataType::Report)?));
    Ok(made)
}
