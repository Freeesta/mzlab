// Ported from Attributio plugins/msfinder.py (the idea of MS-FINDER's product-ion assignment with hydrogen rearrangements, without Mono or the Windows program).
//! Attribution of the MS/MS peaks to sub-formulas of a candidate's ion, allowing hydrogen rearrangements. A peak takes the closest sub-formula as in the review
//! (`review::explain`); when none fits, the peak is tried again shifted by k = +1, -1, +2, -2 hydrogen atoms and the fragment formula takes k more hydrogens
//! (a transfer between the two sides of a cleavage). Rearranged assignments cost `PENALTY` of the peak's intensity per hydrogen in the score, so a candidate that
//! needs shifts ranks below one that explains the spectrum plainly. Proofs of a FORMULA, not of a structure; never an identity.

use crate::attributio::review::explain::{
    assign, fmt_delta, fmt_formula, match_adduct, subformula_table, Formula,
};
use crate::attributio::review::settings::ReviewSettings;

/// Order in which the shifts are tried: the plain assignment first.
pub const SHIFTS: [i32; 5] = [0, 1, -1, 2, -2];
/// Share of a peak's intensity lost in the score for each hydrogen of rearrangement.
pub const PENALTY: f64 = 0.25;
const H_MASS: f64 = 1.00782503207;

#[derive(Debug, Clone, PartialEq)]
pub struct ShiftedHit {
    /// Hydrogens moved onto the fragment (0 = plain sub-formula).
    pub shift: i32,
    pub formula: String,
    pub mz_th: f64,
    pub ppm: f64,
    /// Neutral loss from the ion (hydrogen counts may be negative when the fragment gained hydrogens).
    pub loss: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Attribution {
    pub adduct: String,
    pub ion_formula: String,
    /// One entry per peak.
    pub hits: Vec<Option<ShiftedHit>>,
    pub n_explained: usize,
    pub n_rearranged: usize,
    pub frac_explained: f64,
    pub score: f64,
}

/// Attribution of the cleaned spectrum `(mz, it)` to the ion of the formula `fd`; `None` when no adduct of the polarity fits the precursor `mz` within
/// `ppm_ion` or the sub-formula table is too large.
pub fn attribute(
    fd: &Formula,
    mz: f64,
    pol: &str,
    exp: (&[f64], &[f64]),
    s: &ReviewSettings,
) -> Option<Attribution> {
    let am = match_adduct(fd, mz, pol)?;
    if am.ppm.abs() > s.ppm_ion {
        return None;
    }
    let table = subformula_table(&am.ion, pol, &am.adduct, s.max_subformulas)?;
    let mut hits = Vec::with_capacity(exp.0.len());
    for &m in exp.0 {
        let mut found = None;
        for k in SHIFTS {
            let Some((row, _)) = assign(&[m - k as f64 * H_MASS], &table, s.ppm_frag)[0] else {
                continue;
            };
            let mut cnt = table.formula(row);
            let h = cnt.get("H").copied().unwrap_or(0) + k as i64;
            if h < 0 {
                continue;
            }
            cnt.insert("H".into(), h);
            let mz_th = table.mass(row) + k as f64 * H_MASS;
            let ppm = (m - mz_th) / mz_th * 1e6;
            if ppm.abs() > s.ppm_frag {
                continue;
            }
            let mut loss: Formula = am.ion.clone();
            for (e, v) in &cnt {
                *loss.entry(e.clone()).or_insert(0) -= v;
            }
            let nonzero: Formula = cnt
                .iter()
                .filter(|(_, v)| **v != 0)
                .map(|(e, v)| (e.clone(), *v))
                .collect();
            let l = fmt_delta(&loss);
            found = Some(ShiftedHit {
                shift: k,
                formula: fmt_formula(&nonzero),
                mz_th,
                ppm,
                loss: if l.is_empty() { "-".into() } else { l },
            });
            break;
        }
        hits.push(found);
    }
    let total: f64 = exp.1.iter().sum();
    let (mut got, mut score) = (0.0, 0.0);
    for (h, y) in hits.iter().zip(exp.1) {
        if let Some(h) = h {
            got += y;
            score += y * (1.0 - PENALTY * h.shift.abs() as f64);
        }
    }
    Some(Attribution {
        adduct: am.adduct.clone(),
        ion_formula: fmt_formula(&am.ion),
        n_explained: hits.iter().flatten().count(),
        n_rearranged: hits.iter().flatten().filter(|h| h.shift != 0).count(),
        frac_explained: got / total,
        score: score / total,
        hits,
    })
}
