//! The native executor of `features.pair@1`: the positive and the negative `.mzfeat` paired into the CSV of pairs and its `pair/1` document
//! (`mzlab_feat::pair::pair_text`, the function the `flowtab` `.wasm` calls in the browser). This file only reads and writes the files.
//! Called by `flow.rs`.

use crate::flow::{io, one_input, Exec, FileRunner};
use crate::flow_made::Made;
use mzlab_core::error::Result;
use mzlab_core::flow::Job;
use mzlab_core::step::DataType;

/// The step of this file, chained by `flow::all_executors`.
pub(crate) const EXECUTORS: &[(&str, Exec)] = &[("features.pair", Exec::Made(pair_job))];

fn pair_job(r: &FileRunner, job: &Job) -> Result<Made> {
    // a side is named by the file of its table (`<id>.mzfeat`), the name the page gives it too
    let side = |port: &str| -> Result<(String, Vec<u8>)> {
        let p = one_input(r, job, port)?;
        let name = p
            .file_name()
            .map_or(String::new(), |n| n.to_string_lossy().into_owned());
        Ok((name, std::fs::read(&p).map_err(io)?))
    };
    let (pos, neg) = (side("pos")?, side("neg")?);
    let m = mzlab_feat::pair::pair_text(&job.resolved.params, (&pos.0, &pos.1), (&neg.0, &neg.1))?;
    let csv = r.work.join(format!("{}.csv", job.instance.id));
    std::fs::write(&csv, &m.csv).map_err(io)?;
    let doc = r.work.join(format!("{}.pair.json", job.instance.id));
    std::fs::write(&doc, &m.doc).map_err(io)?;
    let mut made = Made::new("pairs", r.keep(csv, DataType::Report)?);
    made.extra = Some(("pair", r.keep(doc, DataType::Report)?));
    Ok(made)
}

#[cfg(test)]
#[path = "flow_pair_tests.rs"]
mod tests;
