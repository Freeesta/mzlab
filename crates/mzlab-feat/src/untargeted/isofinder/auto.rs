//! Automatic detection of heavy elements (Cl, Br, S, Si) in an isotope pattern, after `ElementAutoDetector.java` of mzmine 4.10.90 (MIT, (c) 2004-2022
//! The mzmine Development Team; rewritten, no code copied). From the spacings between pairs of signals near the M+2 defect of each candidate element and
//! the intensity of those signals it lists the elements that cannot be ruled out, best first. Atom counts are only a rough hint. Used by `select`.

use std::collections::BTreeSet;

use super::envelope::jround;
use super::{Pt, Tol};
use mzlab_core::isoscore::isotopes_of;

/// The elements tried when none are given.
pub const DEFAULT_CANDIDATES: [&str; 4] = ["Cl", "Br", "S", "Si"];

/// Distance between 13C and 12C as mzmine writes it in `IsotopePatternUtils.C13_MZ_DELTA`.
const C13_MZ_DELTA: f64 = 1.003355;
const WINDOW_PAD_DA: f64 = 2.5;
const MIN_PEAK_REL: f64 = 0.01;
const STRONG_ABUNDANCE: f64 = 0.5;
const MIN_REACH_WEIGHT: f64 = 0.05;
const REACH_FLOOR_FRACTION: f64 = 0.15;
const MIN_CONFIDENCE: f64 = 0.2;
const MAX_PLAUSIBLE_ATOMS: i64 = 40;
const ATOM_SOFT_CAP: f64 = 8.0;
const MIN_DEFECT_SIGMA: f64 = 0.0009;
const SI_M1_MIN: f64 = 0.02;
const SIGNIFICANT_FRACTION: f64 = 0.3;
const SIGNIFICANT_M1_REL: f64 = 0.02;

/// One element found: symbol, rough atom count and confidence (0-1).
#[derive(Clone, Debug, PartialEq)]
pub struct Found {
    pub symbol: String,
    pub atoms: i64,
    pub confidence: f64,
}

/// Elements that cannot be ruled out, best first; empty when none.
pub type Composition = Vec<Found>;

/// M+2 (and, if any, M+1) isotope of an element: mass difference from the main isotope and abundance relative to it.
struct ElementIsotopes {
    symbol: String,
    m2_delta: f64,
    m2_rel: f64,
    m1: Option<(f64, f64)>,
}

impl ElementIsotopes {
    fn bears_m1(&self) -> bool {
        self.m1.is_some_and(|m| m.1 >= SIGNIFICANT_M1_REL)
    }
}

/// The most abundant isotope within 0.5 of `target` mass units above the main one: `(delta, relative abundance)`.
fn nearest_delta(iso: &[(f64, f64)], main: (f64, f64), target: f64) -> Option<(f64, f64)> {
    let mut best: Option<(f64, f64)> = None;
    for i in iso {
        let (delta, rel) = (i.0 - main.0, i.1 / main.1);
        if (delta - target).abs() <= 0.5 && best.is_none_or(|b| rel > b.1) {
            best = Some((delta, rel));
        }
    }
    best
}

fn element_isotopes(candidates: &[&str]) -> Vec<ElementIsotopes> {
    let mut out = Vec::new();
    for sym in candidates {
        let Some(iso) = isotopes_of(sym) else {
            continue;
        };
        let main = iso
            .iter()
            .fold(iso[0], |m, i| if i.1 > m.1 { *i } else { m });
        let Some(m2) = nearest_delta(iso, main, 2.0) else {
            continue;
        };
        let m1 = nearest_delta(iso, main, 1.0);
        out.push(ElementIsotopes {
            symbol: sym.to_string(),
            m2_delta: m2.0,
            m2_rel: m2.1,
            m1,
        });
    }
    out
}

/// The points of the spectrum within 2.5 / charge of the m/z range of a pattern: the window the detector runs on.
pub fn detection_window(mz: &[f64], y: &[f64], lo: f64, hi: f64, charge: i32) -> Vec<Pt> {
    let pad = WINDOW_PAD_DA / charge.max(1) as f64;
    mz.iter()
        .zip(y)
        .filter(|(m, _)| **m >= lo - pad && **m <= hi + pad)
        .map(|(m, v)| (*m, *v))
        .collect()
}

/// Every `(measured neutral spacing, intensity of the higher peak)` of a pair of significant signals whose spacing lies in `[lo, hi]`.
fn pairs_in_band(sorted: &[Pt], z: i32, min_peak: f64, lo: f64, hi: f64) -> Vec<(f64, f64)> {
    let mut pairs = Vec::new();
    for (i, a) in sorted.iter().enumerate() {
        if a.1 < min_peak {
            continue;
        }
        for b in &sorted[i + 1..] {
            let d = (b.0 - a.0) * z as f64;
            if d > hi {
                break;
            }
            if d >= lo && b.1 >= min_peak {
                pairs.push((d, b.1));
            }
        }
    }
    pairs
}

/// The silicon fingerprint: strongest base-relative M+1 peak whose spacing sits in the 29Si band, below the 13C M+1 position.
fn strongest_m1_heavy(
    sorted: &[Pt],
    z: i32,
    min_peak: f64,
    base: f64,
    tol_neutral: f64,
    el: &[ElementIsotopes],
) -> f64 {
    let deltas: Vec<f64> = el
        .iter()
        .filter(|e| e.bears_m1())
        .filter_map(|e| e.m1.map(|m| m.0))
        .collect();
    if deltas.is_empty() {
        return 0.0;
    }
    let lo = deltas.iter().copied().fold(f64::INFINITY, f64::min);
    let hi = deltas.iter().copied().fold(f64::NEG_INFINITY, f64::max);
    let band_lo = lo - tol_neutral.min(0.0025);
    let band_hi = (hi + tol_neutral).min(C13_MZ_DELTA - 0.003f64.max(0.5 * tol_neutral));
    let best = pairs_in_band(sorted, z, min_peak, band_lo, band_hi)
        .iter()
        .fold(0.0f64, |m, p| m.max(p.1));
    if base > 0.0 {
        best / base
    } else {
        0.0
    }
}

/// Median as mzmine takes it: the mean of the two middle values; 0 for none.
fn median(values: &[f64]) -> f64 {
    let mut v = values.to_vec();
    v.sort_by(f64::total_cmp);
    match v.len() {
        0 => 0.0,
        n => (v[(n - 1) / 2] + v[n / 2]) / 2.0,
    }
}

/// Population standard deviation (not the sample form).
fn std_dev(values: &[f64]) -> f64 {
    if values.len() < 2 {
        return 0.0;
    }
    let n = values.len() as f64;
    let mean = values.iter().sum::<f64>() / n;
    (values.iter().map(|v| (v - mean) * (v - mean)).sum::<f64>() / n).sqrt()
}

/// What the pairs of signals say: median M+2 spacing, base-relative strength of the strongest one, the silicon fingerprint and the precision.
struct Evidence {
    med_delta: f64,
    max_ratio: f64,
    m1_ratio: f64,
    defect_sigma: f64,
}

/// Elements whose M+2 spacing matches one of the pairs and that one atom could explain in height.
fn spacing_matched(
    el: &[ElementIsotopes],
    pairs: &[(f64, f64)],
    max_ratio: f64,
    tol_neutral: f64,
) -> BTreeSet<usize> {
    let mut out = BTreeSet::new();
    for (i, e) in el.iter().enumerate() {
        if max_ratio < REACH_FLOOR_FRACTION * e.m2_rel {
            continue;
        }
        if pairs
            .iter()
            .any(|p| (p.0 - e.m2_delta).abs() <= tol_neutral)
        {
            out.insert(i);
        }
    }
    out
}

/// Heavy elements possible in the signals `(mz, intensity)` of a pattern of charge `charge`; empty when the spectrum shows none.
pub fn detect(signals: &[Pt], charge: i32, tol: &Tol, candidates: &[&str]) -> Composition {
    if signals.len() < 2 || candidates.is_empty() {
        return Vec::new();
    }
    let z = charge.max(1);
    let mut sorted = signals.to_vec();
    sorted.sort_by(|a, b| a.0.total_cmp(&b.0));
    let base = sorted.iter().fold(0.0f64, |m, p| m.max(p.1));
    let el = element_isotopes(candidates);
    if base <= 0.0 || el.is_empty() {
        return Vec::new();
    }
    let min_peak = base * MIN_PEAK_REL;
    let tol_neutral = (tol.for_mass(sorted[sorted.len() / 2].0) * z as f64).max(1e-4);
    let min_m2 = el.iter().fold(f64::INFINITY, |m, e| m.min(e.m2_delta));
    let max_m2 = el.iter().fold(f64::NEG_INFINITY, |m, e| m.max(e.m2_delta));
    let band_lo = min_m2 - 2.0 * tol_neutral;
    let band_hi =
        (max_m2 + 2.0 * tol_neutral).min(2.0 * C13_MZ_DELTA - 0.004f64.max(0.5 * tol_neutral));
    let pairs = pairs_in_band(&sorted, z, min_peak, band_lo, band_hi);
    if pairs.is_empty() {
        return Vec::new();
    }
    let max_heavy = pairs.iter().fold(0.0f64, |m, p| m.max(p.1));
    let strong: Vec<f64> = pairs
        .iter()
        .filter(|p| p.1 >= SIGNIFICANT_FRACTION * max_heavy)
        .map(|p| p.0)
        .collect();
    let count_sigma = tol_neutral / (strong.len().max(1) as f64).sqrt();
    let ev = Evidence {
        med_delta: median(&strong),
        max_ratio: max_heavy / base,
        m1_ratio: strongest_m1_heavy(&sorted, z, min_peak, base, tol_neutral, &el),
        defect_sigma: MIN_DEFECT_SIGMA.max((1.5 * std_dev(&strong)).max(count_sigma)),
    };
    let matched = spacing_matched(&el, &pairs, ev.max_ratio, tol_neutral);
    if matched.is_empty() {
        return Vec::new();
    }
    classify(&el, &matched, &ev)
}

/// Ranks the spacing-matched candidates by the median M+2 defect, the M+2 intensity and the silicon fingerprint and keeps those with a score of
/// at least 0.2, best first (ties in the order of the candidates).
fn classify(el: &[ElementIsotopes], matched: &BTreeSet<usize>, ev: &Evidence) -> Composition {
    let mut scored: Vec<(f64, Found)> = Vec::new();
    for e in el
        .iter()
        .enumerate()
        .filter(|(i, _)| matched.contains(i))
        .map(|(_, e)| e)
    {
        let atoms = jround(ev.max_ratio / e.m2_rel).clamp(1, MAX_PLAUSIBLE_ATOMS);
        let defect = (ev.med_delta - e.m2_delta) / ev.defect_sigma;
        let prior = if atoms as f64 <= ATOM_SOFT_CAP {
            1.0
        } else {
            ATOM_SOFT_CAP / atoms as f64
        };
        let reach = if e.m2_rel <= 0.0 {
            1.0
        } else {
            (ev.max_ratio / e.m2_rel).min(1.0)
        };
        let mut s = (-defect * defect).exp() * prior * reach.max(MIN_REACH_WEIGHT);
        if e.m2_rel < STRONG_ABUNDANCE {
            if e.bears_m1() {
                s *= if ev.m1_ratio >= SI_M1_MIN { 1.3 } else { 0.7 };
            } else if ev.m1_ratio >= SI_M1_MIN {
                s *= 0.7;
            }
        }
        if s >= MIN_CONFIDENCE {
            scored.push((
                s,
                Found {
                    symbol: e.symbol.clone(),
                    atoms,
                    confidence: s.min(1.0),
                },
            ));
        }
    }
    // stable, so equal scores keep the order of the candidates; ranked on the raw score, the confidence is that score capped at 1
    scored.sort_by(|a, b| b.0.total_cmp(&a.0));
    scored.into_iter().map(|p| p.1).collect()
}
