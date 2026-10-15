// Ported from Attributio src/attributio/review/isotopes.py.
//! Theoretical isotope pattern with fine structure and its comparison with the observed MS1 of a feature.
//!
//! The distribution is an exact multinomial expansion per element, pruned below 1e-7 of the largest term and above +6.5 Da, then the elements are
//! convolved. It is kept apart from `crate::isotopes` (isotopologue grouping inside one file) because the review shows every fine-structure peak
//! (13C and 15N at A+1 are separate) and its masses come from the AME2016 table. Evidence for the FORMULA, never for the structure.

use super::adduct::ELECTRON;
use super::explain::Formula;
use std::collections::BTreeMap;

type Iso = &'static [(f64, f64)];

fn isotopes_of(el: &str) -> Option<Iso> {
    Some(match el {
        "H" => &[(1.00782503223, 0.999885), (2.01410177812, 0.000115)],
        "B" => &[(10.0129369, 0.199), (11.0093054, 0.801)],
        "C" => &[(12.0, 0.9893), (13.00335483507, 0.0107)],
        "N" => &[(14.00307400443, 0.99636), (15.00010889888, 0.00364)],
        "O" => &[(15.99491461957, 0.99757), (16.99913175650, 0.00038), (17.99915961286, 0.00205)],
        "F" => &[(18.99840316273, 1.0)],
        "Na" => &[(22.9897692820, 1.0)],
        "Si" => &[(27.97692653465, 0.92223), (28.97649466490, 0.04685), (29.973770136, 0.03092)],
        "P" => &[(30.97376199842, 1.0)],
        "S" => &[(31.9720711744, 0.9499), (32.9714589098, 0.0075), (33.967867004, 0.0425), (35.96708071, 0.0001)],
        "Cl" => &[(34.968852682, 0.7576), (36.965902602, 0.2424)],
        "K" => &[(38.9637064864, 0.932581), (39.963998166, 0.000117), (40.9618252579, 0.067302)],
        "As" => &[(74.9215946, 1.0)],
        "Se" => &[(73.9224759, 0.0089), (75.9192137, 0.0937), (76.9199142, 0.0763), (77.9173091, 0.2377), (79.9165218, 0.4961), (81.9166995, 0.0873)],
        "Br" => &[(78.9183376, 0.5069), (80.9162897, 0.4931)],
        "I" => &[(126.9044719, 1.0)],
        _ => return None,
    })
}

pub const C13_SHIFT: f64 = 1.0033548;
const PRUNE: f64 = 1e-7;
const MAX_SHIFT: f64 = 6.5;

/// A mass rounded to 6 decimals, as an integer key (the mass is `key / 1e6`).
fn key(m: f64) -> i64 {
    (m * 1e6).round() as i64
}

fn mass_of_key(k: i64) -> f64 {
    k as f64 / 1e6
}

type Dist = BTreeMap<i64, f64>;

fn prune(d: Dist) -> Dist {
    let top = d.values().cloned().fold(f64::NEG_INFINITY, f64::max);
    d.into_iter().filter(|(_, p)| *p >= PRUNE * top).collect()
}

/// {mass: probability} of `n` atoms of one element (repeated convolution, pruned).
fn element(iso: Iso, n: i64) -> Dist {
    let m0 = iso.iter().map(|x| x.0).fold(f64::INFINITY, f64::min);
    let mut dist: Dist = BTreeMap::new();
    dist.insert(0, 1.0);
    for step in 0..n {
        let mut new: Dist = BTreeMap::new();
        for (&k, &p) in &dist {
            let m = mass_of_key(k);
            for &(mi, pi) in iso {
                if pi <= 0.0 {
                    continue;
                }
                let kk = key(m + mi);
                if mass_of_key(kk) - m0 * (step + 1) as f64 > MAX_SHIFT {
                    continue;
                }
                *new.entry(kk).or_insert(0.0) += p * pi;
            }
        }
        dist = prune(new);
    }
    dist
}

/// Isotopologues of a singly charged ion: (m/z, abundance %), the monoisotopic = 100. Peaks below `min_rel` % are dropped; peaks closer than 0.00001 Da are merged.
pub fn pattern(ion: &Formula, pol: &str, min_rel: f64) -> Vec<(f64, f64)> {
    if ion.is_empty() || ion.keys().any(|e| isotopes_of(e).is_none()) {
        return Vec::new();
    }
    let mut dist: Dist = BTreeMap::new();
    dist.insert(0, 1.0);
    let mut mono = 0.0;
    for (el, &n) in ion {
        if n <= 0 {
            continue;
        }
        let iso = isotopes_of(el).unwrap();
        mono += n as f64 * iso.iter().map(|x| x.0).fold(f64::INFINITY, f64::min);
        let ed = element(iso, n);
        let mut new: Dist = BTreeMap::new();
        for (&k, &p) in &dist {
            let m = mass_of_key(k);
            for (&ke, &pi) in &ed {
                let kk = key(m + mass_of_key(ke));
                if mass_of_key(kk) - mono > MAX_SHIFT {
                    continue;
                }
                *new.entry(kk).or_insert(0.0) += p * pi;
            }
        }
        dist = prune(new);
    }
    let e = if pol == "POS" { -ELECTRON } else { ELECTRON };
    let p_mono = *dist.values().next().unwrap_or(&1.0);
    let mut out: Vec<(f64, f64)> = Vec::new();
    for (&k, &p) in &dist {
        let rel = p / p_mono * 100.0;
        if rel < min_rel {
            continue;
        }
        let mz = mass_of_key(k) + e;
        match out.last_mut() {
            Some(last) if (mz - last.0).abs() < 1e-5 => last.1 += rel,
            _ => out.push((mz, rel)),
        }
    }
    out.into_iter().map(|(m, r)| (round_to(m, 6), round_to(r, 5))).collect()
}

/// Python-like `round(x, nd)`: correctly rounded on the exact binary value, ties to even (the decimal formatting of Rust does the same).
pub fn round_to(x: f64, nd: i32) -> f64 {
    if !x.is_finite() || nd < 0 {
        return x;
    }
    format!("{:.*}", nd as usize, x).parse().unwrap_or(x)
}

/// One nominal cluster A, A+1, ...
#[derive(Clone, Debug)]
pub struct Cluster {
    pub k: i64,
    pub mz: f64,
    pub lo: f64,
    pub hi: f64,
    pub rel: f64,
    pub main: f64,
}

/// The sticks summed per nominal cluster, for A, A+1, ... A+(n-1).
pub fn clusters(sticks: &[(f64, f64)], n: i64) -> Vec<Cluster> {
    let Some(&(m0, _)) = sticks.first() else {
        return Vec::new();
    };
    (0..n)
        .map(|k| {
            let sel: Vec<(f64, f64)> = sticks.iter().cloned().filter(|(m, _)| py_round((m - m0) / C13_SHIFT) == k).collect();
            let base = m0 + k as f64 * C13_SHIFT;
            if sel.is_empty() {
                return Cluster { k, mz: base, lo: base, hi: base, rel: 0.0, main: base };
            }
            let tot: f64 = sel.iter().map(|t| t.1).sum();
            let mut sig: Vec<f64> = sel.iter().filter(|t| t.1 >= 0.01 * tot).map(|t| t.0).collect();
            if sig.is_empty() {
                sig = sel.iter().map(|t| t.0).collect();
            }
            let main = sel.iter().fold(sel[0], |b, &t| if t.1 > b.1 { t } else { b }).0;
            Cluster {
                k,
                mz: sel.iter().map(|t| t.0 * t.1).sum::<f64>() / tot,
                lo: sig.iter().cloned().fold(f64::INFINITY, f64::min),
                hi: sig.iter().cloned().fold(f64::NEG_INFINITY, f64::max),
                rel: tot,
                main,
            }
        })
        .collect()
}

/// Python `round(x)` for a float: half to even.
fn py_round(x: f64) -> i64 {
    round_to(x, 0) as i64
}

/// One row of the isotope comparison.
#[derive(Clone, Debug, PartialEq)]
pub struct IsoRow {
    pub k: i64,
    pub label: String,
    pub obs_mz: Option<f64>,
    pub ppm: Option<f64>,
    pub obs: f64,
    pub th: f64,
    pub diff: Option<f64>,
    pub verdict: String,
}

/// The comparison of the observed MS1 peaks with the pattern of an ion.
#[derive(Clone, Debug)]
pub struct IsoComparison {
    pub rows: Vec<IsoRow>,
    pub verdict: String,
    pub rmse: Option<f64>,
    pub a_ppm: Option<f64>,
    pub strict_m2: bool,
    pub flags: Vec<String>,
    pub n_judged: usize,
}

/// Tolerances of the isotope verdict.
#[derive(Clone, Copy, Debug)]
pub struct IsoTol {
    pub abs: f64,
    pub rel: f64,
    pub ppm: f64,
}

impl Default for IsoTol {
    fn default() -> Self {
        IsoTol { abs: 0.06, rel: 0.35, ppm: 10.0 }
    }
}

fn has(ion: &Formula, els: &[&str]) -> bool {
    els.iter().any(|e| ion.get(*e).copied().unwrap_or(0) > 0)
}

fn argmax_of(idx: &[usize], it: &[f64]) -> usize {
    let mut best = idx[0];
    for &i in idx {
        if it[i] > it[best] {
            best = i;
        }
    }
    best
}

fn verdict_k2(diff: f64, obs: f64, th: f64, strict: bool, tol: IsoTol) -> String {
    if th < 4.0 && !strict {
        return if diff.abs() <= f64::max(10.0, 0.5 * th) { "ok" } else { "off (weak rule)" }.into();
    }
    if obs < (1.0 - tol.rel) * th && obs < th - 100.0 * tol.abs {
        return if obs < 0.25 * th { "MISSING" } else { "too low" }.into();
    }
    if diff.abs() <= f64::max(100.0 * tol.abs, tol.rel * th) { "ok" } else { "off" }.into()
}

/// The observed MS1 peaks (`mz`, `it`) against the pattern of `ion`; `None` when the precursor peak (A) is not in the scan. `floor` is the smallest
/// intensity the scan recorded in the window: an absent isotopologue expected below it is 'below detection', never judged.
pub fn compare(ion: &Formula, pol: &str, mz: &[f64], it: &[f64], prec: f64, tol: IsoTol, floor: Option<f64>) -> Option<IsoComparison> {
    let sticks = pattern(ion, pol, 1e-3);
    if sticks.is_empty() {
        return None;
    }
    let cl = clusters(&sticks, 4);
    let tol0 = f64::max(0.003, prec * tol.ppm * 1e-6);
    let near: Vec<usize> = (0..mz.len()).filter(|&i| (mz[i] - prec).abs() <= tol0).collect();
    if near.is_empty() {
        return None;
    }
    let ia = argmax_of(&near, it);
    let a_int = it[ia];
    let th_a = cl[0].main;
    let strict = has(ion, &["Cl", "Br", "S"]);
    let mut rows: Vec<IsoRow> = Vec::new();
    for c in &cl {
        if c.k == 0 {
            rows.push(IsoRow {
                k: 0,
                label: "A".into(),
                obs_mz: Some(round_to(mz[ia], 5)),
                ppm: Some(round_to((mz[ia] - th_a) / th_a * 1e6, 2)),
                obs: 100.0,
                th: 100.0,
                diff: Some(0.0),
                verdict: "reference".into(),
            });
            continue;
        }
        let t = f64::max(0.003, c.main * tol.ppm * 1e-6);
        let sel: Vec<usize> = (0..mz.len()).filter(|&i| mz[i] >= c.lo - t && mz[i] <= c.hi + t).collect();
        let obs = if sel.is_empty() { 0.0 } else { sel.iter().map(|&i| it[i]).sum::<f64>() / a_int * 100.0 };
        let (obs_mz, e_ppm) = if sel.is_empty() {
            (None, None)
        } else {
            let j = argmax_of(&sel, it);
            let th_near = sticks
                .iter()
                .filter(|s| py_round((s.0 - sticks[0].0) / C13_SHIFT) == c.k)
                .map(|s| s.0)
                .fold(None, |b: Option<f64>, m| match b {
                    Some(x) if (x - mz[j]).abs() <= (m - mz[j]).abs() => Some(x),
                    _ => Some(m),
                })
                .unwrap_or(c.main);
            (Some(round_to(mz[j], 5)), Some(round_to((mz[j] - th_near) / th_near * 1e6, 2)))
        };
        let th = c.rel;
        let diff = obs - th;
        let label = format!("A+{}", c.k);
        if obs == 0.0 && floor.map_or(false, |f| f != 0.0 && a_int * th / 100.0 < 1.5 * f) {
            rows.push(IsoRow {
                k: c.k,
                label,
                obs_mz: None,
                ppm: None,
                obs: 0.0,
                th: round_to(th, 2),
                diff: None,
                verdict: "below detection".into(),
            });
            continue;
        }
        let verdict = match c.k {
            1 => {
                if diff.abs() <= f64::max(10.0, 0.5 * th) {
                    "ok".to_string()
                } else {
                    "off".to_string()
                }
            }
            2 => verdict_k2(diff, obs, th, strict, tol),
            _ => "shown only".to_string(),
        };
        rows.push(IsoRow { k: c.k, label, obs_mz, ppm: e_ppm, obs: round_to(obs, 2), th: round_to(th, 2), diff: Some(round_to(diff, 2)), verdict });
    }
    Some(summarise(rows, strict))
}

fn summarise(rows: Vec<IsoRow>, strict: bool) -> IsoComparison {
    let bad2 = rows.iter().any(|r| r.k == 2 && matches!(r.verdict.as_str(), "off" | "MISSING" | "too low"));
    let bad1 = rows.iter().any(|r| r.k == 1 && r.verdict == "off");
    let n_judged = rows.iter().filter(|r| (r.k == 1 || r.k == 2) && r.verdict != "below detection").count();
    let verdict = if bad2 {
        "inconsistent"
    } else if bad1 {
        "M+1 off"
    } else if n_judged > 0 {
        "consistent"
    } else {
        "not judged"
    };
    let flags = rows
        .iter()
        .filter(|r| r.k == 2 && matches!(r.verdict.as_str(), "MISSING" | "too low" | "off") && strict)
        .map(|r| format!("A+2 {} (Cl/Br/S expected {:.1} %, observed {:.1} %)", r.verdict, r.th, r.obs))
        .collect();
    let d3: Vec<f64> = rows.iter().filter(|r| (1..=3).contains(&r.k)).filter_map(|r| r.diff).collect();
    let rmse = (!d3.is_empty()).then(|| round_to((d3.iter().map(|d| d * d).sum::<f64>() / d3.len() as f64).sqrt(), 2));
    let a_ppm = rows[0].ppm;
    IsoComparison { rows, verdict: verdict.into(), rmse, a_ppm, strict_m2: strict, flags, n_judged }
}

/// Orbitrap peak width (FWHM, Da) at `mz` for a resolving power given at m/z `at` (R scales as 1/sqrt(m/z)).
pub fn fwhm(mz: f64, resolution: f64, at: f64) -> f64 {
    let r = resolution * (at / mz).sqrt();
    mz / r
}
