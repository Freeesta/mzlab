//! The in-source-fragment verdict of the low-resolution rows of mzFinder and the supposed fragmentation tree: pure functions with parity to
//! `TP_Mine/py/tpmine/isf.py` and `fragtree.py` (`tools/finder_lrisf_riferimento.py`, `tests/golden/finder_lrisf.json`). The verdict (a
//! probability) lives only here, never in `mzlab-core`, whose `ionfamily` ports measure and do not conclude. The classifier is
//! `isf_eval(samples, parent_mz, ions, formula_p) -> Vec<Option<IsfVerdict>>` (`api.classify_ions` with `fam = false`; never panics: on an error
//! the ion gets `None`), `lr_tree(ms2, mz_x, ion_formula, smiles) -> json` (the supposed tree, `fragtree.lr`) and `compare_lr(tree, tp_mz, dm) -> json`.

#[cfg(feature = "chem")]
pub mod compare;
pub mod isf;
pub mod isf_report;
pub mod isf_split;
pub mod isf_text;
#[cfg(feature = "chem")]
pub mod tree;

use mzlab_core::idxread::ReadAt;
use mzlab_core::json::Value;
use mzlab_core::origin::{origin_report, Request, Sample};

#[cfg(feature = "chem")]
pub use compare::compare_lr;
pub use isf_report::{classify_report, Ramp};
#[cfg(feature = "chem")]
pub use tree::{lr_tree, resolve_smiles, ParentMs2};

/// A full-scan file of the series: its label, treatment time (min) and the survey section of its `.mzidx` (what the pipeline has per file).
pub type IsfSample<'a, R> = Sample<'a, R>;

/// What the rows of the Dig LR keep of the verdict of an ion (`row.isf`).
#[derive(Debug, Clone, PartialEq)]
pub struct IsfVerdict {
    pub p_inherited: f64,
    pub isf_ness: f64,
    /// `isf`, `isotope`, `adduct`, `tp`, `other_from_parent`.
    pub probs: Vec<(String, f64)>,
    pub doubtful: bool,
    pub reasons: Vec<String>,
    pub explanation: String,
    pub calibrated: bool,
}

impl IsfVerdict {
    fn from_report(r: &Value) -> Option<IsfVerdict> {
        let probs = match r.get("probs")? {
            Value::Obj(o) => o
                .iter()
                .filter_map(|(k, v)| Some((k.clone(), v.f64()?)))
                .collect(),
            _ => return None,
        };
        Some(IsfVerdict {
            p_inherited: r.get("p_inherited")?.f64()?,
            isf_ness: r.get("isf_ness")?.f64()?,
            probs,
            doubtful: r.get("doubtful")?.bool()?,
            reasons: r
                .get("reasons")?
                .arr()
                .iter()
                .filter_map(|s| s.str().map(str::to_string))
                .collect(),
            explanation: r.get("explanation")?.str()?.to_string(),
            calibrated: r.get("calibrated")?.bool()?,
        })
    }
}

/// One verdict per ion of `ions` (m/z) against the parent, in the order given; `None` where the ion cannot be analysed.
pub fn isf_eval<R: ReadAt>(
    samples: &mut [IsfSample<R>],
    parent_mz: f64,
    ions: &[f64],
    formula_p: Option<&str>,
) -> Vec<Option<IsfVerdict>> {
    ions.iter()
        .map(|&mz| {
            let req = Request {
                mz,
                parent_mz,
                rt0: None,
                rt1: None,
                reference: None,
                formula_p,
                top: 0,
            };
            let rep = origin_report(samples, None, &req).ok()?;
            rep.get("profile")?;
            IsfVerdict::from_report(&classify_report(&rep, formula_p, None)?)
        })
        .collect()
}
