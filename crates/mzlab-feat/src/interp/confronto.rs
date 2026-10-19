//! The comparison of the MS2 of a row with the MS2 of the parent: the typed matching of `similarity::pair_components` against the DDA spectrum of the
//! parent (equal, shifted and common-loss peaks, modified cosine and entropy) and the fragments of the row that are, by formula, a fragment of the
//! parent (equal) or one of its fragments plus the modification (shifted). Intensities are never compared with the infusion (CID against stepped HCD).

use crate::legacy::similarity::{pair_components, prepare, Components};

use super::albero::LibNode;

/// Tolerance of the matching of a fragment to a formula of the library, ppm.
pub const FORMULA_PPM: f64 = 5.0;

/// The result of the comparison.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct VsParent {
    /// `dda`, `infusion` or `both`: what the row was compared with.
    pub reference: &'static str,
    pub cos_mod: f64,
    pub entropy: f64,
    /// m/z in the row of the peaks that are equal to a peak of the parent, shifted by the modification, or a common neutral loss.
    pub equal: Vec<f64>,
    pub shifted: Vec<f64>,
    pub losses: Vec<f64>,
    /// By formula against the library of the tree: peaks that are a fragment of the parent as it is, or plus the modification.
    pub formula_equal: Vec<f64>,
    pub formula_shifted: Vec<f64>,
}

/// Spectrum of the row against the DDA spectrum of the parent. Spectra are (m/z, intensity) with their precursors.
pub fn against_dda(
    parent: (&[f64], &[f64], f64),
    row: (&[f64], &[f64], f64),
) -> Option<(Components, Vec<f64>, Vec<f64>, Vec<f64>)> {
    let (a, b) = (
        prepare(parent.0, parent.1, parent.2),
        prepare(row.0, row.1, row.2),
    );
    if a.0.is_empty() || b.0.is_empty() {
        return None;
    }
    let c = pair_components(&a, parent.2, &b, row.2);
    let of = |k: usize| {
        c.pairs
            .iter()
            .filter(|p| p.2 == k)
            .map(|p| p.1)
            .collect::<Vec<f64>>()
    };
    let (e, s, l) = (of(0), of(1), of(2));
    Some((c, e, s, l))
}

/// Peaks of `mz` that match a formula of the library: `(equal, shifted)`, where shifted means library m/z plus `delta` (the mass of the modification).
pub fn by_formula(mz: &[f64], lib: &[LibNode], delta: f64) -> (Vec<f64>, Vec<f64>) {
    let near = |m: f64, t: f64| (m - t).abs() <= t * FORMULA_PPM * 1e-6;
    let (mut equal, mut shifted) = (vec![], vec![]);
    for &m in mz {
        if lib.iter().any(|n| near(m, n.mz)) {
            equal.push(m);
        } else if lib.iter().any(|n| near(m, n.mz + delta)) {
            shifted.push(m);
        }
    }
    (equal, shifted)
}
