//! Formula candidates for the rows that formed but that no rule explains (`ST_UNEXPLAINED`), scored by how much their elements look like the parent's
//! and how much of the row's MS2 their sub-formulas explain. High resolution only. A candidate is a lead for the chemist: it never changes the status,
//! the kinship or the level of a row.
//!
//! Score (ours, after the idea of annotating transformation products by formula, Helmus et al., ES&T 59:3723, 2025):
//! - `element_fit` = max(Σ_e min(p_e, c_e) / Σ_e c_e, Σ_e min(p_e, c_e) / Σ_e p_e), p = parent and c = candidate counts of each element (H included);
//! - `ms2_explained` = share of the MS2 intensity of the row (precursor left out) matched within `FRAG_TOL` Da by a sub-formula of the ion of the
//!   candidate; `None` without an MS2 (or when the candidate has too many sub-formulas to list);
//! - `isotope_fit` = score of the observed isotope pattern against the ion formula (isoscore); `None` without observed pattern;
//! - `total` = `element_fit` + `ms2_explained` (0 when absent) + `isotope_fit` (0 when absent). The parts are kept apart in the output.

use mzlab_core::elements::{mass_of, parse_formula};
use mzlab_core::json::{round, Out, Value};

use super::{TpBlock, ST_UNEXPLAINED};
use crate::evid::producers::neutral_of;

/// Mass window (ppm) of the formulas of a row, most formulas searched per row, most kept per row.
pub const PPM: f64 = 5.0;
pub const SEARCHED: usize = 50;
pub const KEPT: usize = 3;
/// Extra oxygens allowed over the parent; hydrogens go free within the ring-and-double-bond count.
const EXTRA_O: i64 = 6;
const EXTRA_H: i64 = 8;
/// Tolerance (Da) between a peak of the MS2 and a sub-formula.
const FRAG_TOL: f64 = 0.01;

/// A formula candidate of a row.
#[derive(Debug, Clone, PartialEq)]
pub struct FitCandidate {
    pub neutral_formula: String,
    pub ion_formula: String,
    pub ppm: f64,
    pub element_fit: f64,
    pub ms2_explained: Option<f64>,
    pub isotope_fit: Option<f64>,
    pub total: f64,
}

/// Share of the elements two formulas have in common (see the module header); 0 when one of them is empty.
pub fn element_fit(parent: &[(String, u64)], cand: &[(String, u64)]) -> f64 {
    let count = |f: &[(String, u64)], e: &str| f.iter().find(|(s, _)| s == e).map_or(0, |x| x.1);
    let shared: u64 = cand.iter().map(|(e, c)| (*c).min(count(parent, e))).sum();
    let (p, c): (u64, u64) = (
        parent.iter().map(|x| x.1).sum(),
        cand.iter().map(|x| x.1).sum(),
    );
    if p == 0 || c == 0 {
        return 0.0;
    }
    (shared as f64 / c as f64).max(shared as f64 / p as f64)
}

/// Sorted m/z of the ion of every sub-formula of `ion` (a formula without charge, ion charge `z`); None past the combination cap or on a bad formula.
fn sub_masses(ion: &str, z: i32) -> Option<Vec<f64>> {
    mzlab_core::subformulas::submasses_opt(ion, z, 0.0)
}

/// Share of the intensity of the peaks of an MS2 (`mz`, `it`), the precursor `mz0` excluded, that a sub-formula of the ion explains.
fn ms2_share(ion: &str, z: i32, spec: &(Vec<f64>, Vec<f64>), mz0: f64) -> Option<f64> {
    let subs = sub_masses(ion, z)?;
    let (mut hit, mut all) = (0.0, 0.0);
    for (&m, &i) in spec.0.iter().zip(&spec.1) {
        if m >= mz0 - FRAG_TOL || i.partial_cmp(&0.0) != Some(std::cmp::Ordering::Greater) {
            continue;
        }
        all += i;
        let k = subs.partition_point(|&s| s < m - FRAG_TOL);
        if subs.get(k).is_some_and(|&s| s <= m + FRAG_TOL) {
            hit += i;
        }
    }
    (all > 0.0).then(|| hit / all)
}

/// Hill order: C, H, then the other elements alphabetically; a count of 1 is not written.
fn formula_text(counts: &[(String, i64)]) -> String {
    let mut v: Vec<&(String, i64)> = counts.iter().filter(|x| x.1 > 0).collect();
    mzlab_core::sortc::slice_by_key(&mut v, |x| {
        (x.0 != "C", x.0 != "H" || x.0 == "C", x.0.clone())
    });
    v.iter()
        .map(|(e, n)| {
            if *n == 1 {
                e.clone()
            } else {
                format!("{e}{n}")
            }
        })
        .collect()
}

/// Ring and double bond count of a neutral formula (C, H, N, P and the halogens count; any other element is left out).
fn rdbe2(counts: &[(String, i64)]) -> i64 {
    let at = |e: &str| counts.iter().find(|x| x.0 == e).map_or(0, |x| x.1);
    let hal: i64 = ["F", "Cl", "Br", "I"].iter().map(|e| at(e)).sum();
    2 * at("C") + 2 + at("N") + at("P") - at("H") - hal
}

/// The neutral formulas within `PPM` of the neutral mass `neutral` whose counts stay in `limits` (0 to the parent's, oxygens and hydrogens a few
/// more; no element the parent lacks), with a whole ring-and-double-bond count of 0 or more. Sorted by |error|, at most `SEARCHED`.
fn formulas_near(
    neutral: f64,
    limits: &[(String, i64)],
    ion_mz: f64,
) -> Vec<(Vec<(String, i64)>, f64)> {
    let tol = ion_mz * PPM * 1e-6;
    let m = |e: &str| mass_of(e).unwrap_or(0.0);
    let others: Vec<&(String, i64)> = limits.iter().filter(|x| x.0 != "C" && x.0 != "H").collect();
    let (cmax, hmax) = limits.iter().fold((0, 0), |a, x| match x.0.as_str() {
        "C" => (x.1, a.1),
        "H" => (a.0, x.1),
        _ => a,
    });
    let mut out = Vec::new();
    let mut cur = vec![0i64; others.len()];
    loop {
        let rest = neutral
            - others
                .iter()
                .zip(&cur)
                .map(|(o, n)| m(&o.0) * *n as f64)
                .sum::<f64>();
        for c in 0..=cmax {
            let h = ((rest - 12.0 * c as f64) / m("H")).round() as i64;
            let err = rest - 12.0 * c as f64 - h as f64 * m("H");
            if h < 0 || h > hmax || err.abs() > tol {
                continue;
            }
            let mut counts = vec![("C".to_string(), c), ("H".to_string(), h)];
            counts.extend(others.iter().zip(&cur).map(|(o, n)| (o.0.clone(), *n)));
            let r2 = rdbe2(&counts);
            if r2 >= 0 && r2 % 2 == 0 && counts.iter().any(|x| x.1 > 0) {
                out.push((counts, err / ion_mz * 1e6));
            }
        }
        let Some(k) = (0..cur.len()).rev().find(|&k| cur[k] < others[k].1) else {
            break;
        };
        cur[k] += 1;
        cur[k + 1..].iter_mut().for_each(|x| *x = 0);
    }
    mzlab_core::sortc::slice_by(&mut out, |a, b| a.1.abs().total_cmp(&b.1.abs()));
    out.truncate(SEARCHED);
    out
}

/// A formula found for a row with its scores.
fn scored(
    counts: &[(String, i64)],
    ppm: f64,
    pf: &[(String, u64)],
    spec: Option<&(Vec<f64>, Vec<f64>)>,
    iso: Option<&[(f64, f64)]>,
    mz: f64,
    z: i32,
) -> FitCandidate {
    let neutral_formula = formula_text(counts);
    let ion: Vec<(String, i64)> = counts
        .iter()
        .map(|(e, n)| (e.clone(), if e == "H" { n + i64::from(z) } else { *n }))
        .collect();
    let ion_formula = formula_text(&ion);
    let fit = element_fit(pf, &parse_formula(&neutral_formula).unwrap_or_default());
    let ms2 = spec.and_then(|s| ms2_share(&ion_formula, z, s, mz));
    let iso_fit = iso.and_then(|obs| {
        if obs.is_empty() {
            return None;
        }
        let io = mzlab_core::isoscore::IsoOpts {
            r200: 0.0,
            sigma_ppm: PPM,
            sigma_int: 0.1,
            min_rel: 1.0,
            noise_rel: 0.0,
            norm: "base",
        };
        mzlab_core::isoscore::isotope_score(&ion, z, obs, &io)
            .ok()
            .map(|s| round(s.score, 4))
    });
    FitCandidate {
        neutral_formula,
        ion_formula,
        ppm,
        element_fit: fit,
        ms2_explained: ms2,
        isotope_fit: iso_fit,
        total: fit + ms2.unwrap_or(0.0) + iso_fit.unwrap_or(0.0),
    }
}

/// The best `KEPT` candidates of a row at `mz` for the parent `parent` (neutral formula): ordered by total, then by |ppm|. Empty when the parent
/// cannot be read or no formula within the elements of the parent fits the mass.
pub fn candidates(
    parent: &str,
    mz: f64,
    polarity: i8,
    spec: Option<(&[f64], &[f64])>,
    iso: Option<&[(f64, f64)]>,
) -> Vec<FitCandidate> {
    let Ok(pf) = parse_formula(parent) else {
        return Vec::new();
    };
    let limits: Vec<(String, i64)> = pf
        .iter()
        .map(|(e, n)| {
            let n = *n as i64;
            let hi = match e.as_str() {
                "O" => n + EXTRA_O,
                "H" => n + EXTRA_H,
                _ => n,
            };
            (e.clone(), hi)
        })
        .collect();
    let spec = spec.map(|(m, i)| (m.to_vec(), i.to_vec()));
    let z = if polarity < 0 { -1 } else { 1 };
    let mut out: Vec<FitCandidate> = formulas_near(neutral_of(mz, polarity), &limits, mz)
        .into_iter()
        .map(|(counts, ppm)| scored(&counts, ppm, &pf, spec.as_ref(), iso, mz, z))
        .collect();
    mzlab_core::sortc::slice_by(&mut out, |a, b| {
        b.total
            .total_cmp(&a.total)
            .then(a.ppm.abs().total_cmp(&b.ppm.abs()))
    });
    out.truncate(KEPT);
    out
}

impl TpBlock {
    /// Fill `fit_candidates` for every row formed and not explained. `rows` gives (m/z, MS2 peaks if any, isotope pattern if any) of a row;
    /// nothing is done at unit resolution, without a parent formula or without the block.
    pub fn add_fit_candidates<'a>(
        &mut self,
        parent: &str,
        polarity: i8,
        unit: bool,
        rows: impl Fn(
            usize,
        ) -> (
            f64,
            Option<(&'a [f64], &'a [f64])>,
            Option<&'a [(f64, f64)]>,
        ),
    ) {
        self.fit_candidates.clear();
        if unit || parent.is_empty() {
            return;
        }
        for r in 0..self.status.len() {
            if self.status[r] != ST_UNEXPLAINED {
                continue;
            }
            let (mz, spec, iso) = rows(r);
            let c = candidates(parent, mz, polarity, spec, iso);
            if !c.is_empty() {
                self.fit_candidates.push((r as u32, c));
            }
        }
    }

    /// The candidates of a row (empty when it has none).
    pub fn fit_of(&self, row: usize) -> &[FitCandidate] {
        self.fit_candidates
            .binary_search_by_key(&(row as u32), |x| x.0)
            .map_or(&[][..], |i| &self.fit_candidates[i].1)
    }
}

/// The array of candidates as JSON (`[{neutral_formula, ion_formula, ppm, element_fit, ms2_explained, isotope_fit, total}]`).
pub fn candidates_json(o: &mut Out, c: &[FitCandidate]) {
    o.raw("[");
    for (i, x) in c.iter().enumerate() {
        o.raw(if i > 0 { ",{" } else { "{" });
        o.raw("\"neutral_formula\":")
            .str(&x.neutral_formula)
            .raw(",\"ion_formula\":")
            .str(&x.ion_formula)
            .raw(",\"ppm\":")
            .num(round(x.ppm, 2))
            .raw(",\"element_fit\":")
            .num(round(x.element_fit, 4))
            .raw(",\"ms2_explained\":")
            .opt(x.ms2_explained.map(|v| round(v, 4)))
            .raw(",\"isotope_fit\":")
            .opt(x.isotope_fit.map(|v| round(v, 4)))
            .raw(",\"total\":")
            .num(round(x.total, 4))
            .raw("}");
    }
    o.raw("]");
}

/// Candidates back from the array written by `candidates_json`.
pub fn candidates_from(a: &[Value]) -> Vec<FitCandidate> {
    let s = |x: &Value, k: &str| x.get(k).and_then(Value::str).unwrap_or("").to_string();
    let f = |x: &Value, k: &str| x.get(k).and_then(Value::f64);
    a.iter()
        .map(|x| FitCandidate {
            neutral_formula: s(x, "neutral_formula"),
            ion_formula: s(x, "ion_formula"),
            ppm: f(x, "ppm").unwrap_or(0.0),
            element_fit: f(x, "element_fit").unwrap_or(0.0),
            ms2_explained: f(x, "ms2_explained"),
            isotope_fit: f(x, "isotope_fit"),
            total: f(x, "total").unwrap_or(0.0),
        })
        .collect()
}
