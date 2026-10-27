//! The native executors of `export.table_csv` and `export.mztab`: the table file (`mzlab_feat_table_wasm::core::table_file`, the one the page
//! downloads) of a `.mzfeat` with the roles of the samples file, as CSV or as mzTab-M, so the plan and the page write the same bytes.

use std::path::PathBuf;

use crate::flow::{io, read_one, FileRunner};
use crate::flow_base::open;
use mzlab_core::budget::sha256_hex;
use mzlab_core::error::{Error, Result};
use mzlab_core::flow::{Artifact, Job};
use mzlab_core::step::DataType;
use mzlab_feat::store::{read_table_from, Slice};
use mzlab_feat_table_wasm::core::table_file;
use mzlab_feat_table_wasm::pol;

/// Polarity the detection of the table ran in: the recipe in its meta is text (`detect=Params { .. polarity: Some(-1) .. }`); positive when it names none.
pub(crate) fn polarity(table: &[u8]) -> Result<i32> {
    let (_, meta) = read_table_from(&mut Slice(table), table.len() as u64)?;
    let recipe = meta.get("recipe").and_then(|r| r.str()).unwrap_or("");
    Ok(if recipe.contains("polarity: Some(-1)") {
        -1
    } else {
        1
    })
}

/// Polarity of the table: the rule of `mzlab_feat_table_wasm::pol` (the recipe, else the files of its lineage), the one the browser applies.
pub(crate) fn polarity_of(table: &[u8], files: &[PathBuf]) -> Result<i32> {
    let ix: Vec<_> = files.iter().filter_map(|p| open(p).ok()).collect();
    pol::of_table(&mut Slice(table), table.len() as u64, &ix)
}

/// The text of the file `kind` of the table, whole (no view: a filter view keeps no reference to its table).
pub(crate) fn export_text(
    table: &[u8],
    samples: &[u8],
    kind: &str,
    files: &[PathBuf],
) -> Result<String> {
    if table.first() == Some(&b'{') {
        return Err(Error::new("err.flow.export.view"));
    }
    let samples = crate::flow_table::samples_text(samples)?;
    let sha = sha256_hex(table);
    table_file(
        &mut Slice(table),
        table.len() as u64,
        &samples,
        "",
        &sha,
        polarity_of(table, files)?,
        kind,
    )
}

fn export_job(r: &FileRunner, job: &Job, kind: &str, ext: &str) -> Result<Artifact> {
    let (table, samples) = (read_one(r, job, "table")?, read_one(r, job, "samples")?);
    let files = job
        .inputs
        .iter()
        .find(|(p, _)| p == "table")
        .and_then(|(_, v)| v.first())
        .and_then(|a| r.lineage(a))
        .map(|l| l.index)
        .unwrap_or_default();
    let text = export_text(&table.1, &samples.1, kind, &files)?;
    let out = r.work.join(format!("{}.{ext}", job.instance.id));
    std::fs::write(&out, text).map_err(io)?;
    r.keep(out, DataType::Report)
}

/// `export.table_csv`: the file of the parameter `kind` (`full` when it is left out).
pub(crate) fn csv_job(r: &FileRunner, job: &Job) -> Result<Artifact> {
    let kind = job
        .resolved
        .params
        .iter()
        .find(|(k, _)| k == "kind")
        .and_then(|(_, v)| v.str());
    export_job(r, job, kind.unwrap_or("full"), "csv")
}

/// `export.mztab`: the table as mzTab-M 2.0.
pub(crate) fn mztab_job(r: &FileRunner, job: &Job) -> Result<Artifact> {
    export_job(r, job, "mztab", "mzTab")
}
