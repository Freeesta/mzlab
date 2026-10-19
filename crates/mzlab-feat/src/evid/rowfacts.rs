//! The measured facts of every row of a table (`Facts`): the isotope pattern against the formula of the hypothesis of the row, the purity of its MS2 and the
//! share of that MS2 the parent explains. Shared by the `prove` example and the proofs of the `.wasm`; the caller chooses which MS2 of a row it hands in.

use super::producers::{isotope_score, network_share, neutral_of, FeatIndex, Peaks};
use super::Facts;
use crate::hypo::Registry;
use crate::table::Aligned;

/// Window (ppm) that matches a row to its hypothesis, RT window (min) that finds the parent row, tolerance (Da) of the shared fragments.
const HYP_PPM: f64 = 5e-6;
const PARENT_RT: f64 = 0.3;
const FRAGMENT_TOL: f64 = 0.01;

/// The row within 5 ppm and `rt_tol` minutes of the ion with the largest area.
pub fn row_near(t: &Aligned, mz: f64, rt: f64, rt_tol: f64) -> Option<usize> {
    let area = |r: usize| {
        t.area[r * t.n_files..(r + 1) * t.n_files]
            .iter()
            .copied()
            .filter(|a| a.is_finite())
            .fold(0.0, f64::max)
    };
    (0..t.len())
        .filter(|&r| (t.mz[r] - mz).abs() <= mz * 5e-6 && (t.rt[r] - rt).abs() <= rt_tol)
        .max_by(|&a, &b| area(a).total_cmp(&area(b)))
}

/// The facts of every row and the MS2 peaks of the parent row (its fragments, for gate 2 of the view of the products).
///
/// `spectra[row]` is the MS2 chosen for the row with its purity. The isotope pattern is scored against the formula of the hypothesis within 5 ppm, in
/// the files of `idx`; at unit resolution (`unit`) it is not computed, the window of an isotopologue being as wide as the isotopologue is far. The share
/// of the MS2 explained by the parent needs the hypothesis of the row, the parent row and both spectra; the parent row itself has none.
pub fn row_facts(
    aligned: &Aligned,
    reg: &Registry,
    parent: Option<(f64, f64)>,
    spectra: &[Option<(Peaks, f32)>],
    idx: &[FeatIndex],
    unit: bool,
) -> (Vec<Facts>, Option<Peaks>) {
    let sign = reg.polarity;
    let parent_row = parent.and_then(|(mz, rt)| row_near(aligned, mz, rt, PARENT_RT));
    let parent_spec =
        parent_row.and_then(|r| spectra.get(r).and_then(|x| x.as_ref().map(|x| x.0.clone())));
    let parent_neutral = parent.map(|p| neutral_of(p.0, sign));
    let facts = (0..aligned.len())
        .map(|row| {
            let mz = aligned.mz[row];
            let hyp = reg.hyps.iter().find(|h| (h.mz - mz).abs() <= mz * HYP_PPM);
            let isotope = hyp
                .filter(|_| !unit)
                .and_then(|h| isotope_score(idx, mz, aligned.rt[row], &h.formula, sign));
            let spec = spectra.get(row).and_then(Option::as_ref);
            let network = match (hyp, &parent_spec, spec, parent_neutral) {
                (Some(h), Some(ps), Some((cs, _)), Some(pn)) if parent_row != Some(row) => {
                    network_share(cs, ps, h.neutral - pn, FRAGMENT_TOL)
                }
                _ => None,
            };
            Facts {
                isotope,
                fragments: None,
                purity: spec.map(|x| f64::from(x.1)),
                network,
            }
        })
        .collect();
    (facts, parent_spec)
}
