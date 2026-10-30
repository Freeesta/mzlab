//! `qc.mass_accuracy`: how well each file was calibrated, measured on ions that are known to be there (the lab background list, the internal standards and the standards of the user),
//! and, if asked, the m/z of a feature table corrected with what was measured. Raw data and indexes are only read; the table is the one thing that can be written, as a copy.
//! Everything here is arithmetic over the peak tables of the indexes, so the native command and the browser run the same code (`run`). The method of the fit is one module per
//! method (`ols`, `huber`, `ransac`; `offset`, `linear_mz`, `linear_mz_rt`), chosen by enum in `fit`. Contract: `docs/schemi/massacc-1.json`, `docs/agenti/qc-massa.md`.

mod analyse;
mod correct;
pub mod fit;
mod huber;
mod istd;
mod linear_mz;
mod linear_mz_rt;
pub mod measure;
mod offset;
mod ols;
mod ransac;
pub mod refs;
mod report;

use mzlab_core::error::{Error, Result};
use mzlab_core::idxread::{Index, ReadAt};
use mzlab_core::json::Value;

pub use analyse::{FileResult, Point};
pub use fit::{Model, Robust};
pub use refs::{RefIon, Role};

use istd::area_rsd;

/// The parameters of the step (defaults of the descriptor).
#[derive(Clone, Debug, PartialEq)]
pub struct Params {
    pub search_ppm: f64,
    pub min_intensity: f64,
    pub min_refs: usize,
    pub robust: Robust,
    pub model: Model,
    pub warn_offset_ppm: f64,
    pub warn_spread_ppm: f64,
    pub warn_is_rt_min: f64,
    pub warn_is_area_rsd: f64,
}

impl Default for Params {
    fn default() -> Params {
        Params {
            search_ppm: 20.0,
            min_intensity: 1e4,
            min_refs: 5,
            robust: Robust::Huber,
            model: Model::None,
            warn_offset_ppm: 3.0,
            warn_spread_ppm: 2.0,
            warn_is_rt_min: 0.1,
            warn_is_area_rsd: 0.2,
        }
    }
}

impl Params {
    /// The parameters from an object of the resolved ones (`[(name, value)]`); one that is missing keeps its default, one that is wrong is `err.flow.param.type`.
    pub fn from_value(v: &Value) -> Result<Params> {
        let mut p = Params::default();
        let bad = |name: &str| Error::new("err.flow.param.type").with("param", name);
        let num = |name: &str, into: &mut f64| -> Result<()> {
            match v.get(name) {
                None => Ok(()),
                Some(x) => x
                    .f64()
                    .filter(|n| n.is_finite() && *n >= 0.0)
                    .map(|n| *into = n)
                    .ok_or_else(|| bad(name)),
            }
        };
        num("search_ppm", &mut p.search_ppm)?;
        num("min_intensity", &mut p.min_intensity)?;
        num("warn_offset_ppm", &mut p.warn_offset_ppm)?;
        num("warn_spread_ppm", &mut p.warn_spread_ppm)?;
        num("warn_is_rt_min", &mut p.warn_is_rt_min)?;
        num("warn_is_area_rsd", &mut p.warn_is_area_rsd)?;
        let mut refs = p.min_refs as f64;
        num("min_refs", &mut refs)?;
        p.min_refs = refs as usize;
        if let Some(x) = v.get("robust") {
            p.robust = x
                .str()
                .and_then(Robust::parse)
                .ok_or_else(|| bad("robust"))?;
        }
        if let Some(x) = v.get("model") {
            p.model = x.str().and_then(Model::parse).ok_or_else(|| bad("model"))?;
        }
        Ok(p)
    }
}

/// A file of the check: the name it is reported under and whether the samples mark it as holding the internal standards.
#[derive(Clone, Debug, PartialEq)]
pub struct FileSpec {
    pub name: String,
    pub istd: bool,
}

/// What a run made.
#[derive(Debug)]
pub struct Outcome {
    /// `massacc/1`.
    pub json: String,
    /// The table with its m/z corrected, only when the model is not `none` and a table was given.
    pub table: Option<Vec<u8>>,
    /// The warning keys of the whole run, sorted, once each.
    pub warnings: Vec<String>,
}

/// The order the warnings are listed in.
const WARNINGS: [&str; 5] = [
    "warn.qc.mass.offset",
    "warn.qc.mass.spread",
    "warn.qc.mass.is_rt",
    "warn.qc.mass.is_area",
    "warn.qc.mass.few_refs",
];

fn sort_warnings(w: &mut Vec<&'static str>) {
    *w = WARNINGS.iter().copied().filter(|k| w.contains(k)).collect();
}

/// `run` from what a caller has: the (name, SHA-256) of each file, the samples document, the lab list, the CSV of the user. The native command and the browser both call this one.
pub fn check<R: ReadAt, T: ReadAt>(
    open: &mut dyn FnMut(usize) -> Result<Index<R>>,
    names: &[(String, String)],
    samples: Option<&[u8]>,
    contaminants: &[u8],
    csv: Option<&[u8]>,
    table: Option<(&mut T, u64)>,
    p: &Params,
) -> Result<Outcome> {
    let flags = istd_flags(samples, names)?;
    let files: Vec<FileSpec> = names
        .iter()
        .zip(flags)
        .map(|((name, _), istd)| FileSpec {
            name: name.clone(),
            istd,
        })
        .collect();
    run(open, &files, refs::references(contaminants, csv)?, table, p)
}

/// The standard of one compound over the files: how much its area varies.
#[derive(Clone, Debug, PartialEq)]
pub struct IstdBatch {
    pub name: String,
    pub n: usize,
    pub median_area: f64,
    pub rsd: Option<f64>,
}

/// The check over every file: `open(k)` gives the index of file `k` (called once each, in order). `refs` are the references before `refs::prepare`. `table` is the `.mzfeat` to correct
/// (read only when `model` is not `none`).
pub fn run<R: ReadAt, T: ReadAt>(
    open: &mut dyn FnMut(usize) -> Result<Index<R>>,
    files: &[FileSpec],
    refs: Vec<RefIon>,
    table: Option<(&mut T, u64)>,
    p: &Params,
) -> Result<Outcome> {
    let refs = refs::prepare(refs, p.search_ppm);
    let mut results: Vec<FileResult> = Vec::with_capacity(files.len());
    for (k, spec) in files.iter().enumerate() {
        let mut index = open(k)?;
        let m = measure::measure(&mut index, &refs, p)?;
        results.push(analyse::analyse_file(spec, &m, &refs, p));
    }
    let batch = batch_of(&mut results, &refs, p);
    let has_table = table.is_some();
    let corrected = match (table, p.model) {
        (Some((t, size)), m) if m != Model::None => Some(correct::correct(t, size, &results)?),
        _ => None,
    };
    let json = report::write(p, &refs, &results, &batch, has_table, corrected.is_some());
    let mut warnings: Vec<&'static str> = results
        .iter()
        .flat_map(|r| r.warnings.iter().copied())
        .collect();
    if area_warning(&batch, p) {
        warnings.push("warn.qc.mass.is_area");
    }
    sort_warnings(&mut warnings);
    Ok(Outcome {
        json,
        table: corrected,
        warnings: warnings.into_iter().map(String::from).collect(),
    })
}

/// The standards over the files: the spread of each area and the files that stand out; their warning is added to the files.
fn batch_of(results: &mut [FileResult], refs: &[RefIon], p: &Params) -> Vec<IstdBatch> {
    let mut ids: Vec<usize> = results
        .iter()
        .flat_map(|r| r.istds.iter().map(|s| s.r))
        .filter(|&r| refs[r].role == Role::Istd)
        .collect();
    crate::sortc::ord(&mut ids);
    ids.dedup();
    let mut out = Vec::new();
    for r in ids {
        let areas: Vec<(usize, f64)> = results
            .iter()
            .enumerate()
            .filter_map(|(k, f)| f.istds.iter().find(|s| s.r == r).map(|s| (k, s.area)))
            .collect();
        let values: Vec<f64> = areas.iter().map(|a| a.1).collect();
        let (rsd, med) = (area_rsd(&values), ols::median(&values));
        if rsd.is_some_and(|x| x > p.warn_is_area_rsd) {
            for &(k, a) in &areas {
                if med > 0.0 && (a / med - 1.0).abs() > p.warn_is_area_rsd {
                    results[k].warnings.push("warn.qc.mass.is_area");
                }
            }
        }
        out.push(IstdBatch {
            name: refs[r].name.clone(),
            n: areas.len(),
            median_area: med,
            rsd,
        });
    }
    for f in results.iter_mut() {
        sort_warnings(&mut f.warnings);
    }
    out
}

/// Whether the batch has a standard whose area varies more than the limit.
pub(crate) fn area_warning(batch: &[IstdBatch], p: &Params) -> bool {
    batch
        .iter()
        .any(|b| b.rsd.is_some_and(|x| x > p.warn_is_area_rsd))
}

/// The `istd` flag of each file of `names` (name, SHA-256) from the samples document (`campioni/1`, `/2` or `/3`): matched by file name, else by SHA-256; no document means every file may hold standards; a document without the file means none.
pub fn istd_flags(samples: Option<&[u8]>, names: &[(String, String)]) -> Result<Vec<bool>> {
    let Some(bytes) = samples else {
        return Ok(vec![true; names.len()]);
    };
    let doc = mzlab_core::json::parse(bytes)?;
    let rows: Vec<(String, String, bool)> =
        if doc.get("schema").and_then(Value::str) == Some("campioni/1") {
            crate::samples::from_json(bytes)?
                .into_iter()
                .map(|s| (s.file, s.sha256, s.istd))
                .collect()
        } else {
            mzlab_core::campioni::from_json(bytes)?
                .into_iter()
                .map(|c| (c.file, c.sha256, c.istd))
                .collect()
        };
    Ok(names
        .iter()
        .map(|(name, sha)| {
            rows.iter()
                .find(|r| r.0 == *name)
                .or_else(|| rows.iter().find(|r| !sha.is_empty() && r.1 == *sha))
                .is_some_and(|r| r.2)
        })
        .collect())
}
