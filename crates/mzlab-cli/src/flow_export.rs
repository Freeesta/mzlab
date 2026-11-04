//! The native executors of `export.table_csv` and `export.mztab`: the table file (`mzlab_feat_table_wasm::core::table_file`, the one the page
//! downloads) of a `.mzfeat` with the roles of the samples file, as CSV or as mzTab-M, so the plan and the page write the same bytes.

use std::path::PathBuf;

use crate::flow::{io, read_one, FileRunner};
use crate::flow_base::open;
use mzlab_core::budget::sha256_hex;
use mzlab_core::error::{Error, Result};
use mzlab_core::flow::{Artifact, Job};
use mzlab_core::step::DataType;
use mzlab_feat::store::Slice;
use mzlab_feat_table_wasm::core::{table_file, view_origin};
use mzlab_feat_table_wasm::pol;

/// Polarity of the table: the rule of `mzlab_feat_table_wasm::pol` (the recipe, else the files of its lineage), the one the browser applies.
pub(crate) fn polarity_of(table: &[u8], files: &[PathBuf]) -> Result<i32> {
    let ix: Vec<_> = files.iter().filter_map(|p| open(p).ok()).collect();
    pol::of_table(&mut Slice(table), table.len() as u64, &ix)
}

/// The `.mzfeat` of this run (made by a step, a plan input, or in the working folder) whose SHA-256 is `sha`: the table a filter view was made from.
fn table_by_sha(r: &FileRunner, sha: &str) -> Option<PathBuf> {
    let made: Vec<PathBuf> = {
        let m = r.made.lock().unwrap_or_else(|e| e.into_inner());
        m.values().cloned().collect()
    };
    let near = std::fs::read_dir(&r.dir)
        .into_iter()
        .flatten()
        .flatten()
        .map(|e| e.path());
    made.into_iter()
        .chain(r.inputs.values().cloned())
        .chain(near)
        .filter(|p| p.extension().is_some_and(|x| x == "mzfeat"))
        .find(|p| std::fs::read(p).is_ok_and(|b| sha256_hex(&b) == sha))
}

/// The text of the file `kind` of the table under the view `spec` (empty: the whole table).
pub(crate) fn export_text(
    table: &[u8],
    samples: &[u8],
    kind: &str,
    files: &[PathBuf],
    spec: &str,
) -> Result<String> {
    let samples = crate::flow_table::samples_text(samples)?;
    let sha = sha256_hex(table);
    table_file(
        &mut Slice(table),
        table.len() as u64,
        &samples,
        spec,
        &sha,
        polarity_of(table, files)?,
        kind,
    )
}

fn export_job(r: &FileRunner, job: &Job, kind: &str, ext: &str) -> Result<Artifact> {
    let (view, samples) = (read_one(r, job, "table")?, read_one(r, job, "samples")?);
    let mut origin = job
        .inputs
        .iter()
        .find(|(p, _)| p == "table")
        .and_then(|(_, v)| v.first())
        .cloned();
    let (mut table, mut spec) = (view.1, String::new());
    if table.first() == Some(&b'{') {
        // a `table.filter` view: the table it was made from is a file of this run, and the rows kept are the view
        let (sha, rules) = view_origin(&String::from_utf8_lossy(&table))?;
        let a = Artifact {
            sha256: sha,
            bytes: 0,
            ty: DataType::Table,
            label: None,
        };
        let at = table_by_sha(r, &a.sha256).ok_or_else(|| Error::new("err.flow.export.view"))?;
        table = std::fs::read(at).map_err(|_| Error::new("err.flow.export.view"))?;
        (spec, origin) = (rules, Some(a));
    }
    let files = origin
        .and_then(|a| r.lineage(&a))
        .map(|l| l.index)
        .unwrap_or_default();
    let text = export_text(&table, &samples.1, kind, &files, &spec)?;
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
