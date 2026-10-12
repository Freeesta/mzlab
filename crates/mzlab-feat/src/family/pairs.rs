//! Candidate pairs of one file: two good, non-isotopic peaks that co-elute (apex within `coelution_min` minutes) and whose difference of
//! m/z is a step of the tables of `rel` (an adduct, a neutral loss, the dimer). Features are sorted by m/z, so each step is a binary search:
//! O(n log n) for a fixed number of steps.

use super::evidence::{Rules, Spec};
use super::rel::Tables;
use crate::sortc;
use crate::table::{FileFeatures, FLAG_BACKGROUND, FLAG_FT_LOBE, FLAG_GOOD, FLAG_ISOTOPE};

/// Peaks that can be members: good quality, neither background nor a lobe of the Fourier transform, not already an isotopologue.
pub fn eligible(f: &FileFeatures, i: usize) -> bool {
    let fl = f.flags[i];
    fl & FLAG_GOOD != 0 && fl & (FLAG_BACKGROUND | FLAG_FT_LOBE | FLAG_ISOTOPE) == 0
}

/// Do the two peaks co-elute? Apex within `coelution_min` minutes, or within half of the narrower FWHM (a saturated peak has a flat top and its
/// apex wanders; the shape correlation that follows is what decides).
pub fn coelute(f: &FileFeatures, i: usize, j: usize, coelution_min: f64) -> bool {
    let fw_min = f.fwhm[i].min(f.fwhm[j]) as f64 * f.dt / 60.0;
    ((f.rt[i] - f.rt[j]).abs() as f64) <= coelution_min.max(0.5 * fw_min)
}

/// `coelute` on the `gate` of `candidate_pairs`; false when either row is not eligible.
fn gate_coelutes(gate: &[[f32; 2]], dt: f64, i: usize, j: usize, coelution_min: f64) -> bool {
    let fw_min = gate[i][1].min(gate[j][1]) as f64 * dt / 60.0;
    ((gate[i][0] - gate[j][0]).abs() as f64) <= coelution_min.max(0.5 * fw_min)
}

/// `(lighter, heavier)` row pairs, ascending by heavier then lighter, each once.
pub fn candidate_pairs(
    f: &FileFeatures,
    t: &Tables,
    ppm: f64,
    abs_da: f64,
    coelution_min: f64,
) -> Vec<(u32, u32)> {
    let mut out: Vec<(u32, u32)> = Vec::new();
    let steps: Vec<f64> = t.steps().map(|(_, _, d)| d).collect();
    // (rt, fwhm) of every row side by side, NaN for the rows that cannot be members: the window loop touches one cache line per hit
    let gate: Vec<[f32; 2]> = (0..f.len())
        .map(|i| {
            if eligible(f, i) {
                [f.rt[i], f.fwhm[i]]
            } else {
                [f32::NAN; 2]
            }
        })
        .collect();
    let ok = |i: usize| !gate[i][0].is_nan();
    // `want = up - step` and its tolerance grow with `up`: one pair of moving pointers per step instead of two binary searches
    let mut at = vec![(0usize, 0usize); steps.len() + 1];
    for j in (0..f.len()).filter(|&j| ok(j)) {
        let up = f.mz[j];
        let wants = steps.iter().map(|d| up - d).chain([t.dimer_base(up)]);
        for (want, ptr) in wants.zip(at.iter_mut()) {
            let tol = (want * ppm * 1e-6).max(abs_da) * 1.01 + 1e-9;
            while ptr.0 < f.len() && f.mz[ptr.0] < want - tol {
                ptr.0 += 1;
            }
            ptr.1 = ptr.1.max(ptr.0);
            while ptr.1 < f.len() && f.mz[ptr.1] <= want + tol {
                ptr.1 += 1;
            }
            for i in ptr.0..ptr.1.min(j) {
                if gate_coelutes(&gate, f.dt, i, j, coelution_min)
                    && t.has_reading(f.mz[i], up, ppm, abs_da)
                {
                    out.push((i as u32, j as u32));
                }
            }
        }
    }
    sortc::by_key(&mut out, |&(i, j)| (j, i));
    out.dedup();
    out
}

/// Pairs from the MS2: the lighter peak is a fragment in the MS2 of the heavier one, whatever the neutral loss. `spec(row)` is the spectrum of
/// the best MS2 of a row. Same order as `candidate_pairs`.
pub fn ms2_pairs<'a>(
    f: &FileFeatures,
    rows: &[u32],
    spec: impl Fn(usize) -> Option<&'a Spec>,
    r: &Rules,
) -> Vec<(u32, u32)> {
    let mut out: Vec<(u32, u32)> = Vec::new();
    for &j in rows {
        let j = j as usize;
        let Some(s) = spec(j) else { continue };
        for &m in &s.mz {
            if (m - f.mz[j]).abs() <= 1.5 * r.tol(f.mz[j]) || m > f.mz[j] - 0.5 {
                continue;
            }
            let tol = r.tol(m);
            let a = f.mz.partition_point(|&x| x < m - tol);
            let b = f.mz.partition_point(|&x| x <= m + tol);
            for i in a..b.min(j) {
                if eligible(f, i) && coelute(f, i, j, r.coelution_min) {
                    out.push((i as u32, j as u32));
                }
            }
        }
    }
    sortc::by_key(&mut out, |&(i, j)| (j, i));
    out.dedup();
    out
}
