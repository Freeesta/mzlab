//! The modification of a row: the difference of neutral formula from the parent given by its best hypothesis (`ipotesi/1`), the alternatives within
//! the tolerance and the isotope check of the MS1 at the apex (M+1/M, M+2/M, sulfur count). Used by `interp::build_row`.

use std::collections::BTreeMap;

use mzlab_core::composition::isotope_ratios;
use mzlab_core::elements::{parse_formula, Formula};

use crate::legacy::series::Hyp;

/// Relative deviation of the observed M+2/M from the expected one that still counts as consistent with the sulfur count.
pub const M2_TOLERANCE: f64 = 0.30;
/// Hypotheses this close (ppm) to the m/z of a row are candidates for its modification.
pub const MATCH_PPM: f64 = 5.0;

/// The minus sign of the labels (U+2212), never the hyphen.
pub const MINUS: char = '\u{2212}';

fn count_map(f: &Formula) -> BTreeMap<String, i64> {
    let mut m = BTreeMap::new();
    for (e, n) in f {
        *m.entry(e.clone()).or_insert(0) += *n as i64;
    }
    m
}

/// Difference `a - b` of two formulas as signed counts per element (zeros dropped).
pub fn diff(a: &Formula, b: &Formula) -> BTreeMap<String, i64> {
    let mut d = count_map(a);
    for (e, n) in count_map(b) {
        *d.entry(e).or_insert(0) -= n;
    }
    d.retain(|_, n| *n != 0);
    d
}

/// Element order of a label: C, H, N, O, S, then alphabetical.
fn rank(e: &str) -> (usize, String) {
    let k = ["C", "H", "N", "O", "S"]
        .iter()
        .position(|x| *x == e)
        .unwrap_or(5);
    (k, e.to_string())
}

/// `+O`, `−2H`, `+O−2H`: gains first, then losses.
pub fn label(d: &BTreeMap<String, i64>) -> String {
    let mut items: Vec<(&String, &i64)> = d.iter().collect();
    items.sort_by_key(|(e, _)| rank(e));
    let mut out = String::new();
    for sign in [1i64, -1] {
        for (e, &n) in items.iter().filter(|(_, n)| n.signum() == sign) {
            out.push(if sign > 0 { '+' } else { MINUS });
            if n.abs() != 1 {
                out.push_str(&n.abs().to_string());
            }
            out.push_str(e);
        }
    }
    out
}

/// One way to the formula of a row.
#[derive(Debug, Clone, PartialEq)]
pub struct Candidate {
    pub delta: String,
    pub neutral_formula: String,
    pub ion_formula: String,
    pub ppm: f64,
    pub source: &'static str,
}

/// The `source` of a hypothesis from the names the registry gives its sources.
fn source_of(h: &Hyp) -> &'static str {
    let has = |s: &str| h.sources.iter().any(|x| x == s);
    if has("regola") {
        "rule"
    } else if has("rottura") {
        "cleavage"
    } else if has("rete") {
        "network"
    } else {
        "formula"
    }
}

/// The ion formula of a neutral one for the protonated adduct (the polarity of the series is positive in HR).
fn protonated(neutral: &Formula) -> String {
    let mut f = neutral.clone();
    match f.iter_mut().find(|(e, _)| e == "H") {
        Some(h) => h.1 += 1,
        None => f.push(("H".into(), 1)),
    }
    mzlab_core::elements::fmt(&f)
}

/// The hypotheses whose ion m/z is within `MATCH_PPM` of `mz`, best first (fewest steps is not known here: smallest |ppm|, then the label).
/// The first is the modification; the others are the alternatives. `None` formulas that do not parse are skipped.
pub fn candidates(mz: f64, parent: &Formula, hyps: &[&Hyp]) -> Vec<Candidate> {
    let mut out: Vec<Candidate> = Vec::new();
    for h in hyps {
        let ppm = (mz - h.mz) / h.mz * 1e6;
        let Ok(f) = parse_formula(&h.formula) else {
            continue;
        };
        if !ppm.is_finite() || ppm.abs() > MATCH_PPM {
            continue;
        }
        let c = Candidate {
            delta: label(&diff(&f, parent)),
            neutral_formula: mzlab_core::elements::fmt(&f),
            ion_formula: protonated(&f),
            ppm,
            source: source_of(h),
        };
        if !out.iter().any(|o| o.neutral_formula == c.neutral_formula) {
            out.push(c);
        }
    }
    out.sort_by(|a, b| {
        a.ppm
            .abs()
            .total_cmp(&b.ppm.abs())
            .then(a.neutral_formula.cmp(&b.neutral_formula))
    });
    out
}

/// The verdict on the sulfur count and the numbers behind it.
#[derive(Debug, Clone, PartialEq)]
pub struct IsoCheck {
    pub m1_obs: f64,
    pub m1_exp: f64,
    pub m2_obs: f64,
    pub m2_exp: f64,
    /// `consistent`, `inconsistent` or `unknown` (no M+2 measured).
    pub s_count: &'static str,
}

/// Expected and observed isotope ratios of an ion formula: `obs` = intensities of M, M+1 and M+2 at the apex (M = 0 gives `unknown`).
pub fn iso_check(ion_formula: &str, obs: [f64; 3]) -> Option<IsoCheck> {
    let f = parse_formula(ion_formula).ok()?;
    let counts: Vec<(String, i64)> = f.iter().map(|(e, n)| (e.clone(), *n as i64)).collect();
    let (m1_exp, m2_exp) = isotope_ratios(&counts);
    if obs[0] <= 0.0 {
        return None;
    }
    let (m1_obs, m2_obs) = (obs[1] / obs[0], obs[2] / obs[0]);
    let s_count = if obs[2] <= 0.0 {
        "unknown"
    } else if (m2_obs - m2_exp).abs() <= M2_TOLERANCE * m2_exp {
        "consistent"
    } else {
        "inconsistent"
    };
    Some(IsoCheck {
        m1_obs,
        m1_exp,
        m2_obs,
        m2_exp,
        s_count,
    })
}
