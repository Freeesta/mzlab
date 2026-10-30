//! The native executor of `table.stats@1`: Welch's t per comparison of the design, the one-way ANOVA, BH q-values and log2FC (`stat::compare`) on the
//! areas of a `.mzfeat`. The conditions come from the design (`disegno/1`) and the samples file (`campioni/1` or `campioni/2`), matched to the columns of
//! the table by file name, else by SHA-256. Writes `stat/1`. Called by `flow.rs`.

use crate::flow::{read_one, write_made, FileRunner};
use mzlab_core::disegno;
use mzlab_core::error::{Error, Result};
use mzlab_core::flow::{Artifact, Job};
use mzlab_core::step::DataType;
// moved to mzlab-feat (shared with the browser); the other table steps of this crate keep calling them from here
pub(crate) use mzlab_feat::samples::{columns, samples_of};
use mzlab_feat::stat::{compare, comparisons_from_design, layout_from_design, StatsOpts};
use mzlab_feat::store::{origin_from_meta, read_table};

fn number(job: &Job, name: &str) -> Option<f64> {
    let found = job.resolved.params.iter().find(|(k, _)| k == name);
    found.and_then(|(_, v)| v.f64())
}

/// The `stat/1` text of the comparisons of `design` on `table`.
pub fn stats_text(job: &Job, table: &[u8], samples: &[u8], design: &[u8]) -> Result<String> {
    let (t, meta) = read_table(table)?;
    let inputs = origin_from_meta(&meta).inputs;
    if inputs.len() != t.n_files {
        return Err(Error::new("error.badStatInput"));
    }
    let cols = columns(&inputs, &samples_of(samples)?);
    let design = disegno::from_json(design)?;
    let lay = layout_from_design(&cols, &design);
    let pairs = comparisons_from_design(&design, &lay)?;
    let matrix: Vec<Vec<f64>> = (0..t.len())
        .map(|r| t.area[r * t.n_files..(r + 1) * t.n_files].to_vec())
        .collect();
    let defaults = StatsOpts::default();
    let opts = StatsOpts {
        pseudo: number(job, "pseudocount"),
        floor: number(job, "floor").unwrap_or(defaults.floor),
    };
    let out = compare(&matrix, &lay, &pairs, &opts)?;
    Ok(mzlab_feat::stat::write_stats(&out, &lay))
}

pub(crate) fn stats_job(r: &FileRunner, job: &Job) -> Result<Artifact> {
    let table = read_one(r, job, "table")?;
    let (samples, design) = (read_one(r, job, "samples")?, read_one(r, job, "design")?);
    let text = stats_text(job, &table.1, &samples.1, &design.1)?;
    write_made(r, job, text, DataType::Report)
}
