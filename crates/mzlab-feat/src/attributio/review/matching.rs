// Ported from Attributio src/attributio/review/matching.py.
//
// Spectrum against spectrum for the review. The cosine itself is `mzlab_lib::scores::gnps_match_prepared`, the GNPS cosine that
// `crates/mzlab-lib/tests/parita.rs` (`check_attributio`) already holds equal to Attributio's `readers/ms2.py::similarity`; this file
// adds what the review needs around it: cleaning that keeps the indices of the input, the lower limit of the scan window (`cos_vis`),
// the matched pairs and the elemental formulas of a mass difference.

use mzlab_lib::scores::gnps_match_prepared;

/// A spectrum as the review holds it: peaks as read and the precursor m/z.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Ms2 {
    pub mz: Vec<f64>,
    pub it: Vec<f64>,
    pub prec: f64,
}

/// Peaks that survive the cleaning, with their index in the input.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Cleaned {
    pub mz: Vec<f64>,
    pub it: Vec<f64>,
    pub idx: Vec<usize>,
}

/// Peaks at or above `prec - 0.5` are dropped, then those below `rel_min` of the base peak that is left.
pub fn clean(mz: &[f64], it: &[f64], prec: f64, rel_min: f64) -> Cleaned {
    let mut out = Cleaned::default();
    for (k, (m, i)) in mz.iter().zip(it).enumerate() {
        if *m < prec - 0.5 {
            out.mz.push(*m);
            out.it.push(*i);
            out.idx.push(k);
        }
    }
    let mx = out.it.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
    let keep: Vec<bool> = out.it.iter().map(|i| *i >= rel_min * mx).collect();
    let pick = |v: &[f64]| -> Vec<f64> {
        v.iter()
            .zip(&keep)
            .filter(|(_, k)| **k)
            .map(|(x, _)| *x)
            .collect()
    };
    let idx = out
        .idx
        .iter()
        .zip(&keep)
        .filter(|(_, k)| **k)
        .map(|(x, _)| *x)
        .collect();
    Cleaned {
        mz: pick(&out.mz),
        it: pick(&out.it),
        idx,
    }
}

/// How a peak pair matched.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    /// Same m/z.
    Exact,
    /// Shifted by the mass difference of the precursors.
    Shifted,
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct Match {
    pub cosine: f64,
    pub reverse: f64,
    pub matched: usize,
    pub n_shifted: usize,
    /// `(index in the input of a, index in the input of b, kind)`, in the order they were taken (highest product first).
    pub pairs: Vec<(usize, usize, Kind)>,
}

/// Cosine of `a` against `b`. `shift` = 0: the plain cosine; otherwise the modified cosine with delta m = `shift` (peaks of `b` may also match
/// at m/z + `shift`). `low_limit`: peaks of `b` below it are removed first (`cos_vis`); 0 or `None` = none.
pub fn match_spectra(a: &Ms2, b: &Ms2, tol: f64, shift: f64, low_limit: Option<f64>) -> Match {
    let ca = clean(&a.mz, &a.it, a.prec, 0.01);
    let mut cb = clean(&b.mz, &b.it, b.prec, 0.01);
    if let Some(lim) = low_limit.filter(|l| *l != 0.0) {
        let v: Vec<usize> = (0..cb.mz.len()).filter(|k| cb.mz[*k] >= lim).collect();
        cb = Cleaned {
            mz: v.iter().map(|k| cb.mz[*k]).collect(),
            it: v.iter().map(|k| cb.it[*k]).collect(),
            idx: v.iter().map(|k| cb.idx[*k]).collect(),
        };
    }
    if ca.mz.is_empty() || cb.mz.is_empty() {
        return Match::default();
    }
    let (wa, wb): (Vec<f64>, Vec<f64>) = (
        ca.it.iter().map(|v| v.sqrt()).collect(),
        cb.it.iter().map(|v| v.sqrt()).collect(),
    );
    let g = gnps_match_prepared(&ca.mz, &wa, &cb.mz, &wb, shift, tol, shift != 0.0);
    if g.pairs.is_empty() {
        return Match::default();
    }
    let pairs = g
        .pairs
        .iter()
        .map(|(i, j, sh)| {
            (
                ca.idx[*i],
                cb.idx[*j],
                if *sh { Kind::Shifted } else { Kind::Exact },
            )
        })
        .collect();
    Match {
        cosine: g.cosine,
        reverse: g.reverse,
        matched: g.matched,
        n_shifted: g.n_shifted,
        pairs,
    }
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct Comparison {
    pub cos: f64,
    pub rev: f64,
    pub matched: usize,
    pub n_shifted: usize,
    pub pairs: Vec<(usize, usize, Kind)>,
    pub cos_vis: f64,
    pub matched_vis: usize,
}

/// Cosine and matched peaks on all the reference peaks and on the visible ones (m/z >= `low_limit`, the lower limit of the MS2 scan window).
pub fn compare(
    exp: &Ms2,
    reference: &Ms2,
    tol: f64,
    low_limit: Option<f64>,
    shift: f64,
) -> Comparison {
    let a = match_spectra(exp, reference, tol, shift, None);
    let (cos_vis, matched_vis) = match low_limit.filter(|l| *l != 0.0) {
        Some(l) => {
            let b = match_spectra(exp, reference, tol, shift, Some(l));
            (b.cosine, b.matched)
        }
        None => (a.cosine, a.matched),
    };
    Comparison {
        cos: a.cosine,
        rev: a.reverse,
        matched: a.matched,
        n_shifted: a.n_shifted,
        pairs: a.pairs,
        cos_vis,
        matched_vis,
    }
}

// ------------------------------------------------------------------------------ delta m

const MASS_H: f64 = 1.00782503207;
const MASS_C: f64 = 12.0;
const MASS_N: f64 = 14.0030740048;
const MASS_O: f64 = 15.99491461956;
const MASS_P: f64 = 30.97376163;
const MASS_S: f64 = 31.97207100;

/// Common modifications (Da, exact monoisotopic differences); delta m = precursor(feature) - precursor(reference), `+X` = the feature has X more.
pub const COMMON_DELTAS: [(&str, f64); 42] = [
    ("+H2", 2.015650),
    ("-H2", -2.015650),
    ("+CH2", 14.015650),
    ("-CH2", -14.015650),
    ("+C2H4", 28.031300),
    ("-C2H4", -28.031300),
    ("+O", 15.994915),
    ("-O", -15.994915),
    ("+H2O", 18.010565),
    ("-H2O", -18.010565),
    ("+CO", 27.994915),
    ("-CO", -27.994915),
    ("+CO2", 43.989829),
    ("-CO2", -43.989829),
    ("+C2H2O (acetyl)", 42.010565),
    ("-C2H2O (acetyl)", -42.010565),
    ("+NH3", 17.026549),
    ("-NH3", -17.026549),
    ("+NH", 15.010899),
    ("+SO3 (sulfate)", 79.956815),
    ("-SO3", -79.956815),
    ("+C6H8O6 (glucuronide)", 176.032088),
    ("-C6H8O6", -176.032088),
    ("+C6H10O5 (hexose)", 162.052824),
    ("-C6H10O5", -162.052824),
    ("+HPO3", 79.966331),
    ("+CH2O (hydroxymethyl)", 30.010565),
    ("+Cl-H", 33.961028),
    ("-Cl+H", -33.961028),
    ("+Br-H", 77.910511),
    ("+CH2O2 (formic)", 46.005479),
    ("+C3H4O3", 88.016044),
    ("+O-H2 (oxo)", 13.979265),
    ("+C2H3NO (glycine)", 57.021464),
    ("+C5H7NO3 (glutamine)", 129.042593),
    ("+C3H5NO2S (cysteine)", 119.004099),
    ("+C10H15N3O6S (glutathione)", 305.068156),
    ("+C2H4O (ethoxy)", 44.026215),
    ("+C3H6 (propyl)", 42.046950),
    ("+C4H8", 56.062600),
    ("+Na-H", 21.981943),
    ("+K-H", 37.955882),
];

#[derive(Debug, Clone, PartialEq)]
pub struct DeltaFormula {
    pub label: String,
    pub mass: f64,
    pub err_mda: f64,
    pub common: bool,
}

/// Python's `round(x, 2)` for the errors shown in mDa.
fn round2(x: f64) -> f64 {
    (x * 100.0).round_ties_even() / 100.0
}

/// Signed elemental difference in Hill order (C, H, then the rest alphabetical): `{C:1, H:2}` -> `+CH2`, `{H:-2, O:1}` -> `+O-H2`.
fn fmt_delta(d: &[(&str, i32)]) -> String {
    let side = |sign: i32| -> String {
        let mut els: Vec<(&str, i32)> = d
            .iter()
            .filter(|(_, n)| *n * sign > 0)
            .map(|(e, n)| (*e, n * sign))
            .collect();
        els.sort_by_key(|(e, _)| match *e {
            "C" => (0, ""),
            "H" => (1, ""),
            e => (2, e),
        });
        els.iter()
            .map(|(e, n)| {
                if *n > 1 {
                    format!("{e}{n}")
                } else {
                    (*e).to_string()
                }
            })
            .collect()
    };
    let (pos, neg) = (side(1), side(-1));
    let s = format!(
        "{}{}",
        if pos.is_empty() {
            String::new()
        } else {
            format!("+{pos}")
        },
        if neg.is_empty() {
            String::new()
        } else {
            format!("-{neg}")
        }
    );
    if s.is_empty() {
        "0".to_string()
    } else {
        s
    }
}

/// The elemental differences closest to `delta` (Da): the common modifications within `tol_mda`, then a free CHNOPS search within `tol_mda`,
/// simplest first, at most `n` of them.
pub fn delta_formulas(delta: f64, tol_mda: f64, n: usize) -> Vec<DeltaFormula> {
    let mut out: Vec<DeltaFormula> = Vec::new();
    for (lab, m) in COMMON_DELTAS {
        let e = (delta - m) * 1000.0;
        if e.abs() <= tol_mda {
            out.push(DeltaFormula {
                label: lab.to_string(),
                mass: m,
                err_mda: round2(e),
                common: true,
            });
        }
    }
    let found = free_search(delta, tol_mda);
    let mut seen: Vec<String> = out.iter().map(|o| o.label.clone()).collect();
    for (g, h, mass, err) in found.into_iter().take(n * 3) {
        let mut d: Vec<(&str, i32)> = ["C", "N", "O", "P", "S"]
            .iter()
            .zip(g)
            .filter(|(_, v)| *v != 0)
            .map(|(e, v)| (*e, v))
            .collect();
        if h != 0 {
            d.push(("H", h));
        }
        let lab = fmt_delta(&d);
        if seen.contains(&lab) || lab == "0" {
            continue;
        }
        seen.push(lab.clone());
        out.push(DeltaFormula {
            label: lab,
            mass,
            err_mda: round2(err),
            common: false,
        });
        if out.iter().filter(|o| !o.common).count() >= n {
            break;
        }
    }
    out
}

/// Grid of C, N, O, P, S counts with H from the remainder; the candidates within `tol_mda`, simplest first then smallest error
/// (stable, in grid order): `(counts of C N O P S, H, mass, error in mDa)`.
fn free_search(delta: f64, tol_mda: f64) -> Vec<([i32; 5], i32, f64, f64)> {
    let mut found: Vec<([i32; 5], i32, f64, f64, f64)> = Vec::new();
    for c in -4i32..19 {
        for nn in -3i32..6 {
            for o in -5i32..11 {
                for p in -1i32..2 {
                    for s in -2i32..3 {
                        let heavy = c as f64 * MASS_C
                            + nn as f64 * MASS_N
                            + o as f64 * MASS_O
                            + p as f64 * MASS_P
                            + s as f64 * MASS_S;
                        let h = ((delta - heavy) / MASS_H).round_ties_even() as i32;
                        if !(-20..=40).contains(&h) || h.abs() > 2 * c.abs().max(1) + 4 {
                            continue;
                        }
                        let mass = heavy + h as f64 * MASS_H;
                        let err = (delta - mass) * 1000.0;
                        if err.abs() > tol_mda {
                            continue;
                        }
                        let complexity = (c.abs() + nn.abs() + o.abs() + p.abs() + s.abs()) as f64
                            + h.abs() as f64 * 0.25
                            + if p != 0 { 2.0 } else { 0.0 }
                            + if s != 0 { 2.0 } else { 0.0 };
                        found.push(([c, nn, o, p, s], h, mass, err, complexity));
                    }
                }
            }
        }
    }
    found.sort_by(|a, b| a.4.total_cmp(&b.4).then(a.3.abs().total_cmp(&b.3.abs())));
    found
        .into_iter()
        .map(|(g, h, m, e, _)| (g, h, m, e))
        .collect()
}
