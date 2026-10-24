//! Formula candidates for the rows that formed but that no rule explains (`ST_UNEXPLAINED`), scored by how much their elements look like the parent's
//! and how much of the row's MS2 their sub-formulas explain. High resolution only. A candidate is a lead for the chemist: it never changes the status,
//! the kinship or the level of a row.
//!
//! Score (ours, after the idea of annotating transformation products by formula, Helmus et al., ES&T 59:3723, 2025):
//! - `element_fit` = max(Σ_e min(p_e, c_e) / Σ_e c_e, Σ_e min(p_e, c_e) / Σ_e p_e), p = parent and c = candidate counts of each element (H included);
//! - `ms2_explained` = share of the MS2 intensity of the row (precursor left out) matched within `FRAG_TOL` Da by a sub-formula of the ion of the
//!   candidate; `None` without an MS2 (or when the candidate has too many sub-formulas to list);
//! - `total` = `element_fit` + `ms2_explained` (0 when absent). The two parts are kept apart in the output.

use mzlab_core::composition::{compose, Opts, Ranges};
use mzlab_core::elements::parse_formula;
use mzlab_core::json::{parse, round, Out, Value};
use mzlab_core::subformulas::submasses;

use super::{TpBlock, ST_UNEXPLAINED};

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

/// Share of the intensity of the peaks of an MS2 (`mz`, `it`), the precursor `mz0` excluded, that a sub-formula of the ion explains.
fn ms2_share(ion: &str, z: i32, spec: &(Vec<f64>, Vec<f64>), mz0: f64) -> Option<f64> {
    let subs = submasses(ion, z, 0.0).ok()?;
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

/// Counts the formulas of a row may have: 0 to the parent's, oxygens and hydrogens a few more, no element the parent lacks.
fn element_ranges(pf: &[(String, u64)]) -> Ranges {
    pf.iter()
        .map(|(e, n)| {
            let n = *n as i64;
            let hi = match e.as_str() {
                "O" => n + EXTRA_O,
                "H" => n + EXTRA_H,
                _ => n,
            };
            (e.clone(), (0, hi))
        })
        .collect()
}

/// A formula of the result of `compose` with its scores.
fn scored(
    r: &Value,
    pf: &[(String, u64)],
    spec: Option<&(Vec<f64>, Vec<f64>)>,
    z: i32,
    mz: f64,
) -> Option<FitCandidate> {
    let neutral = r.get("neutral").and_then(Value::str)?.to_string();
    let ion_formula = r.get("formula").and_then(Value::str)?.to_string();
    let fit = element_fit(pf, &parse_formula(&neutral).ok()?);
    let ms2 = spec.and_then(|s| ms2_share(&ion_formula, z, s, mz));
    Some(FitCandidate {
        neutral_formula: neutral,
        ion_formula,
        ppm: r.get("delta_ppm").and_then(Value::f64).unwrap_or(0.0),
        element_fit: fit,
        ms2_explained: ms2,
        total: fit + ms2.unwrap_or(0.0),
    })
}

/// The best `KEPT` candidates of a row at `mz` for the parent `parent` (neutral formula): ordered by total, then by |ppm|. Empty when the parent
/// cannot be read or no formula within the elements of the parent fits the mass.
pub fn candidates(
    parent: &str,
    mz: f64,
    polarity: i8,
    spec: Option<(&[f64], &[f64])>,
) -> Vec<FitCandidate> {
    let Ok(pf) = parse_formula(parent) else {
        return Vec::new();
    };
    let o = Opts {
        elements: Some(element_ranges(&pf)),
        ion: if polarity < 0 { "[M-H]-" } else { "[M+H]+" },
        tol: PPM,
        unit: "ppm",
        rdb: (0.0, 100.0),
        max_results: SEARCHED,
        ..Opts::default()
    };
    let Ok(json) = compose(mz, &o) else {
        return Vec::new();
    };
    let Ok(v) = parse(json.as_bytes()) else {
        return Vec::new();
    };
    let spec = spec.map(|(m, i)| (m.to_vec(), i.to_vec()));
    let z = if polarity < 0 { -1 } else { 1 };
    let mut out: Vec<FitCandidate> = v
        .get("results")
        .map(Value::arr)
        .unwrap_or_default()
        .iter()
        .filter_map(|r| scored(r, &pf, spec.as_ref(), z, mz))
        .collect();
    out.sort_by(|a, b| {
        b.total
            .total_cmp(&a.total)
            .then(a.ppm.abs().total_cmp(&b.ppm.abs()))
    });
    out.truncate(KEPT);
    out
}

impl TpBlock {
    /// Fill `fit_candidates` for every row formed and not explained. `rows` gives (m/z, MS2 peaks if any) of a row; nothing is done at unit resolution,
    /// without a parent formula or without the block.
    pub fn add_fit_candidates<'a>(
        &mut self,
        parent: &str,
        polarity: i8,
        unit: bool,
        rows: impl Fn(usize) -> (f64, Option<(&'a [f64], &'a [f64])>),
    ) {
        self.fit_candidates.clear();
        if unit || parent.is_empty() {
            return;
        }
        for r in 0..self.status.len() {
            if self.status[r] != ST_UNEXPLAINED {
                continue;
            }
            let (mz, spec) = rows(r);
            let c = candidates(parent, mz, polarity, spec);
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

/// The array of candidates as JSON (`[{neutral_formula, ion_formula, ppm, element_fit, ms2_explained, total}]`).
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
            total: f(x, "total").unwrap_or(0.0),
        })
        .collect()
}
