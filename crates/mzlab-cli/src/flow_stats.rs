//! The native executor of `table.stats@1`: Welch's t per comparison of the design, the one-way ANOVA, BH q-values and log2FC (`stat::compare`) on the
//! areas of a `.mzfeat`. The conditions come from the design (`disegno/1`) and the samples file (`campioni/1` or `campioni/2`), matched to the columns of
//! the table by file name, else by SHA-256. Writes `stat/1`. Called by `flow.rs`.

use crate::flow::{read_one, write_made, FileRunner};
use mzlab_core::campioni;
use mzlab_core::disegno;
use mzlab_core::error::{Error, Result};
use mzlab_core::flow::{Artifact, Job};
use mzlab_core::json::parse;
use mzlab_core::step::DataType;
use mzlab_feat::samples::{self, Sample};
use mzlab_feat::stat::{compare, comparisons_from_design, layout_from_design, StatsOpts};
use mzlab_feat::store::{origin_from_meta, read_table};

fn number(job: &Job, name: &str) -> Option<f64> {
    let found = job.resolved.params.iter().find(|(k, _)| k == name);
    found.and_then(|(_, v)| v.f64())
}

/// The samples of the file in whichever schema it has, as the engine of the statistics reads them: the role in `kind`, the condition (`serie`) in `group`.
pub(crate) fn samples_of(bytes: &[u8]) -> Result<Vec<Sample>> {
    let doc = parse(bytes)?;
    if doc.get("schema").and_then(|v| v.str()) != Some("campioni/2") {
        return samples::from_json(bytes);
    }
    Ok(campioni::from_json(bytes)?
        .into_iter()
        .map(|c| Sample {
            kind: c.role().to_string(),
            group: c.series.clone(),
            file: c.file,
            sha256: c.sha256,
            time_min: c.time_min,
            replicate: c.replicate,
            conc: c.conc,
            unit: c.unit,
            order: c.order,
            istd: c.istd,
            note: c.note,
        })
        .collect())
}

/// The sample of each column of the table (`inputs` of its meta), by name else by SHA-256; a column the samples do not know is no condition and no blank.
pub(crate) fn columns(inputs: &[(String, String)], all: &[Sample]) -> Vec<Sample> {
    let unknown = |file: &str, sha256: &str| Sample {
        file: file.to_string(),
        sha256: sha256.to_string(),
        kind: "other".into(),
        time_min: None,
        replicate: 1,
        group: String::new(),
        conc: None,
        unit: None,
        order: 1,
        istd: false,
        note: String::new(),
    };
    inputs
        .iter()
        .map(|(name, sha)| {
            let found = all
                .iter()
                .find(|s| s.file == *name)
                .or_else(|| all.iter().find(|s| !sha.is_empty() && s.sha256 == *sha));
            found.cloned().unwrap_or_else(|| unknown(name, sha))
        })
        .collect()
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
