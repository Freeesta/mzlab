//! The supposed fragmentation tree of a low-resolution experiment (`fragtree.lr`): the parent and its strongest MS2 fragments, each with every
//! nominal sub-formula of the ion within 0.5 Da and the common neutral losses as alternatives. Nothing is a verdict: the tree says
//! "supposed: unit resolution". The tree itself is `fragtree::two_level` (shared with the high-resolution mode); this module is the
//! low-resolution entry point.

use crate::legacy::edge::Iimn;
use crate::legacy::fragtree::{elements_of, two_level, vec_of, TwoLevel};

/// Unit resolution, Da.
pub const LR_TOL: f64 = 0.5;

/// The product-ion spectrum of the parent as the Dig LR has it: the fragments (m/z and intensity, already limited to the strongest 12 above 5 %),
/// the number of scans averaged and the precursor m/z of the scans (if it is known).
pub struct ParentMs2<'a> {
    pub mz: &'a [f64],
    pub rel: &'a [f64],
    pub scans: usize,
    pub precursor: Option<f64>,
}

/// `smiles` of the parent as the page gives it: the SMILES field, else the neutral text when it is not a plain formula (it has symbols).
pub fn resolve_smiles(smiles: Option<&str>, neutral: Option<&str>) -> Option<String> {
    let s = smiles.map(str::trim).filter(|s| !s.is_empty());
    if let Some(s) = s {
        return Some(s.to_string());
    }
    let n = neutral.filter(|n| !n.is_empty())?;
    let plain = n.trim().replace(' ', "");
    (plain.is_empty() || !plain.chars().all(char::is_alphanumeric))
        .then(|| n.trim().to_string())
        .filter(|s| !s.is_empty())
}

/// `fragtree.lr(ex)`: `mz_x` is the m/z of the parent in the experiment, `ion_formula` the formula of the [M+H]+ ion (`None` for another adduct or
/// no formula), `smiles` from `resolve_smiles`. JSON text; without MS2 of the parent `{"ok": false, ...}`.
pub fn lr_tree(
    ms2: Option<&ParentMs2>,
    mz_x: f64,
    ion_formula: Option<&str>,
    smiles: Option<&str>,
) -> String {
    let Some(m) = ms2.filter(|m| m.scans > 0 && !m.mz.is_empty() && m.mz.len() == m.rel.len())
    else {
        return r#"{"ok":false,"mode":"lr","note":"no MS2 of the parent in the files","nodes":[]}"#
            .into();
    };
    let els = ion_formula.and_then(elements_of).unwrap_or_default();
    let ion = ion_formula
        .and_then(|f| vec_of(f, &els))
        .filter(|_| !els.is_empty());
    let t = TwoLevel {
        ion: ion.as_deref(),
        els: &els,
        mz0: m.precursor.filter(|p| *p != 0.0).unwrap_or(mz_x),
        mzs: m.mz,
        rel: m.rel,
        smiles: smiles.filter(|s| !s.is_empty()),
        ppm: 0.0,
        abs_da: LR_TOL,
        supposed: true,
        mode: "lr",
        n_scans: Some(m.scans as i64),
    };
    two_level(&t, &Iimn::new().losses)
}
