//! The native executor of `table.kmd@1`: the Kendrick mass defect of every row of a `.mzfeat` and the candidate series of a repeating unit
//! (`stat::kmd_json`, the port of `kmd-calc.js`). Only the m/z and the RT of the rows are read. Called by `flow.rs`.

use crate::flow::{read_one, write_made, FileRunner};
use mzlab_core::error::Result;
use mzlab_core::flow::{Artifact, Job};
use mzlab_core::json::Out;
use mzlab_core::step::DataType;
use mzlab_feat::stat::kmd_json;
use mzlab_feat::store::read_table;

/// The `kmd` text of the rows of `table` under the parameters of the job (`unit`, `tol_mda`, `min_members`).
pub fn kmd_text(job: &Job, table: &[u8]) -> Result<String> {
    let (t, _) = read_table(table)?;
    let mut o = Out::default();
    o.raw("{\"features\":[");
    for (i, (mz, rt)) in t.mz.iter().zip(&t.rt).enumerate() {
        o.raw(if i > 0 { "," } else { "" }).raw("{\"mz\":").num(*mz);
        o.raw(",\"rt\":").num(*rt).raw("}");
    }
    o.raw("]");
    for (k, key) in [("tol_mda", "tolMDa"), ("min_members", "minMembers")] {
        let found = job.resolved.params.iter().find(|(n, _)| n == k);
        if let Some(v) = found.and_then(|(_, v)| v.f64()) {
            o.raw(",\"").raw(key).raw("\":").num(v);
        }
    }
    let unit = job.resolved.params.iter().find(|(n, _)| n == "unit");
    if let Some(u) = unit.and_then(|(_, v)| v.str()) {
        o.raw(",\"unit\":").str(u);
    }
    o.raw("}");
    kmd_json(o.0.as_bytes())
}

pub(crate) fn kmd_job(r: &FileRunner, job: &Job) -> Result<Artifact> {
    let table = read_one(r, job, "table")?;
    write_made(r, job, kmd_text(job, &table.1)?, DataType::Report)
}
