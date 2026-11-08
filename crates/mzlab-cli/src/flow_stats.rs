//! The native executor of `table.stats@1`: Welch's t per comparison of the design, the one-way ANOVA, BH q-values and log2FC (`stat::compare`) on the
//! areas of a `.mzfeat`. The conditions come from the design (`disegno/1`) and the samples file (`campioni/1` or `campioni/2`), matched to the columns of
//! the table by file name, else by SHA-256. Writes `stat/1` (`mzlab_feat::flow_stat`, shared with the browser). Called by `flow.rs`.

use crate::flow::{read_one, write_made, FileRunner};
use mzlab_core::error::Result;
use mzlab_core::flow::{Artifact, Job};
use mzlab_core::step::DataType;

/// The `stat/1` text of the comparisons of `design` on `table` (`mzlab_feat::flow_stat::stats_text`, shared with the browser).
pub fn stats_text(job: &Job, table: &[u8], samples: &[u8], design: &[u8]) -> Result<String> {
    mzlab_feat::flow_stat::stats_text(&job.resolved.params, table, samples, design)
}

pub(crate) fn stats_job(r: &FileRunner, job: &Job) -> Result<Artifact> {
    let table = read_one(r, job, "table")?;
    let (samples, design) = (read_one(r, job, "samples")?, read_one(r, job, "design")?);
    let text = stats_text(job, &table.1, &samples.1, &design.1)?;
    write_made(r, job, text, DataType::Report)
}
