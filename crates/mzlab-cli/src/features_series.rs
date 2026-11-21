//! The CSV of a series of files (`features`) with the plan (`--dry-run`) and the report (`--report`) of the run: indexes in a folder the caller owns,
//! detection and alignment in parallel by `par`, the rows counted per file for the report.

use std::path::{Path, PathBuf};

use mzlab_core::budget::{sha256_hex, MemBudget};
use mzlab_core::error::Result;
use mzlab_core::json::Out;
use mzlab_ops::run::feat::run;
use mzlab_ops::run::table::core::{samples_json, table_file};

use crate::open;

/// The CSV of the series `files`, indexes kept in `tmp` (the caller owns it: WASI has no temporary folder).
pub fn features(files: &[PathBuf], params: &str, tmp: &Path) -> Result<String> {
    features_of(files, &vec![String::new(); files.len()], params, tmp)
}

/// `features` of the samples `labels` name: `NAME@ID` is the sample `ID` of the dataset `files[k]`, an empty label the first (or only) sample.
pub fn features_of(
    files: &[PathBuf],
    labels: &[String],
    params: &str,
    tmp: &Path,
) -> Result<String> {
    features_full(files, labels, params, tmp).map(|(csv, _)| csv)
}

/// `features-plan/1` of the series (`features --dry-run`): the indexes are made in `tmp`, nothing is detected.
pub fn features_plan_of(
    files: &[PathBuf],
    labels: &[String],
    params: &str,
    tmp: &Path,
) -> Result<String> {
    let idxs: Vec<PathBuf> = (0..files.len())
        .map(|k| tmp.join(format!("{k}.mzidx")))
        .collect();
    let recipe = crate::par::index_only(files, labels, &idxs, params)?;
    Ok(crate::par::features_report::plan_text(&recipe, &idxs, files, labels))
}

/// `features_of` and the `features-report/1` of the run (`--report`).
pub fn features_full(
    files: &[PathBuf],
    labels: &[String],
    params: &str,
    tmp: &Path,
) -> Result<(String, String)> {
    let idxs: Vec<PathBuf> = (0..files.len())
        .map(|k| tmp.join(format!("{k}.mzidx")))
        .collect();
    let (recipe, detected, weak) = crate::par::index_and_detect(files, labels, &idxs, params)?;
    let rows: Vec<usize> = run::read_files(&detected)?.0.iter().map(|f| f.len()).collect();
    let mut series = crate::par::Series(detected, weak);
    let origin = run::origin_of(&mut series)?;
    eprintln!("align {} file(s)", files.len());
    let mut table = Vec::new();
    let budget = MemBudget::mb(mzlab_core::budget_auto::native_auto_mb());
    run::align_to_par(
        &mut series,
        &budget,
        &recipe,
        &|k| open(&idxs[k]),
        &mut table,
    )?;
    let mut list = Out::default();
    list.raw("{\"files\":[");
    for (k, (name, sha)) in origin.inputs.iter().enumerate() {
        list.raw(if k > 0 { ",{\"name\":" } else { "{\"name\":" })
            .str(name)
            .raw(",\"sha256\":")
            .str(sha)
            .raw("}");
    }
    list.raw("]}");
    let samples = samples_json(&list.0)?;
    let polarity = recipe.detect.polarity.map_or(1, i32::from);
    let csv = table_file(
        &mut mzlab_feat::store::Slice(&table),
        table.len() as u64,
        &samples,
        "",
        &sha256_hex(&table),
        polarity,
        "full",
    )?;
    let report = crate::par::features_report::report_text(&recipe, &idxs, files, labels, &rows);
    Ok((csv, report))
}
