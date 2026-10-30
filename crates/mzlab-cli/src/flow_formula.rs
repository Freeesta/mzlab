//! The native executors of `formula.ms1@1` and `formula.ms2@1`: the file wrappers of the texts in `mzlab-feat` (`flow_formula_ms1`, `flow_formula_ms2`, the one implementation the
//! browser runs too). Both write JSON `formulas/1`.

use mzlab_core::error::Result;
use mzlab_core::flow::Job;
use mzlab_core::json::{parse, Value};
use mzlab_core::step::DataType;
use mzlab_feat::flow_formula_ms1::WARN_ION;
use mzlab_feat::flow_ms2::Named;

use crate::flow::{read_all, read_one, write_made, FileRunner};
use crate::flow_made::Made;
use crate::flow_nature::written;
use crate::flow_read::named;

/// `formula.ms1`: for each feature the MS1 scan at the apex, in the file where the feature is largest, goes to `formula_id::identify`. An `ion` the plan
/// does not write is left to the files (the descriptor default is not a choice of the plan), as the browser does.
pub fn ms1_text(job: &Job, features: &[u8], files: &[Named]) -> Result<String> {
    let params: Vec<(String, Value)> = job
        .resolved
        .params
        .iter()
        .filter(|(k, _)| k != "ion" || written(job, "ion"))
        .cloned()
        .collect();
    mzlab_feat::flow_formula_ms1::ms1_text(&params, features, files)
}

/// The executor of `formula.ms1`: the text, the ions the files gave as an estimate when the plan leaves `ion` out, `WARN_ION` when a written ion is of the other polarity.
pub(crate) fn ms1_job(r: &FileRunner, job: &Job) -> Result<Made> {
    let (table, index) = (read_one(r, job, "features")?, read_all(r, job, "index")?);
    let text = ms1_text(job, &table.1, &named(&index))?;
    let doc = parse(text.as_bytes())?;
    let rows = doc.get("features").map_or(&[][..], Value::arr);
    let mut ions: Vec<&str> = rows
        .iter()
        .filter_map(|f| f.get("ion").and_then(Value::str))
        .collect();
    ions.sort_unstable();
    ions.dedup();
    let warn = rows
        .iter()
        .any(|f| f.get("warning").and_then(Value::str) == Some(WARN_ION));
    let mut made = Made::new(
        "formulas",
        write_made(r, job, text.clone(), DataType::Formulas)?,
    );
    if !written(job, "ion") && !ions.is_empty() {
        made.estimates.push(mzlab_feat::nature::estimate(
            "ion",
            Value::Str(ions.join(", ")),
            "scan polarity of the files",
        ));
    }
    if warn {
        made.warnings.push(WARN_ION.into());
    }
    Ok(made)
}

/// `formula.ms2`: every candidate formula whose ion fits the precursor of a spectrum explains its cleaned peaks with sub-formulas.
pub fn ms2_text(job: &Job, spectra: Named, formulas: &[&[u8]]) -> Result<String> {
    let alg = job.resolved.algorithm.as_ref().map_or("", |a| a.0.as_str());
    mzlab_feat::flow_formula_ms2::ms2_text(alg, &job.resolved.params, spectra, formulas)
}
