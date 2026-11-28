//! Unit-resolution isotope grouping and the spill-over flag of `features.companions` (opt-in `iso_group`): the +1 and +2 pairs the search found are judged
//! by the Pearson of the heights and the M+1/M, M+2/M ratio, and the accepted ones put M, M+1 and M+2 in one group. A +1 channel that holds
//! `spill_min`-`spill_max` of M and follows it is flagged as instrument spill-over, never grouped. Every pair keeps its verdict and numbers.

use super::evidence::Evidence;
use mzlab_core::json::Value;

/// What the judgement of a pair says; the name is written in `companions/1`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Verdict {
    Isotope,
    Spillover,
    /// r under `iso_min_corr`: never an isotope (the study rule: an ion at M+1 with r < 0.5 is not one).
    RejectedCorr,
    /// The ratio is outside the bounds of an isotope and, at +1, outside the spill-over band.
    RejectedRatio,
    /// Too few shared files or a constant level: r was not computed.
    Untested,
}

impl Verdict {
    pub fn name(self) -> &'static str {
        match self {
            Verdict::Isotope => "isotope",
            Verdict::Spillover => "spillover",
            Verdict::RejectedCorr => "rejected_corr",
            Verdict::RejectedRatio => "rejected_ratio",
            Verdict::Untested => "untested",
        }
    }
}

/// The parameters of the grouping (`--set`); `on` = false leaves the text of the step as it was.
#[derive(Debug, Clone)]
pub struct IsoParams {
    pub on: bool,
    pub min_corr: f64,
    pub ratio_min: f64,
    pub ratio_max: f64,
    pub m2_max: f64,
    pub spill_min: f64,
    pub spill_max: f64,
}

impl IsoParams {
    pub fn of(params: &[(String, Value)]) -> IsoParams {
        let num = |k: &str, d: f64| {
            params
                .iter()
                .find(|(n, _)| n == k)
                .and_then(|(_, v)| v.f64())
                .unwrap_or(d)
        };
        IsoParams {
            on: params
                .iter()
                .find(|(n, _)| n == "iso_group")
                .and_then(|(_, v)| v.bool())
                .unwrap_or(false),
            min_corr: num("iso_min_corr", 0.5),
            ratio_min: num("iso_ratio_min", 0.01).max(0.0),
            ratio_max: num("iso_ratio_max", 0.5).max(0.0),
            m2_max: num("iso_m2_max", 1.0).max(0.0),
            spill_min: num("spill_min", 0.5).max(0.0),
            spill_max: num("spill_max", 0.8).max(0.0),
        }
    }
}

/// A pair at +1 or +2 Da with its numbers and verdict; `a` is the lighter row, `b` the heavier.
#[derive(Debug, Clone)]
pub struct IsoPair {
    pub a: usize,
    pub b: usize,
    pub step: u8,
    pub verdict: Verdict,
    pub ev: Evidence,
}

/// The verdict of a +`step` pair: the correlation floor first (it decides before any ratio), then the spill-over band at +1, then the ratio bounds.
pub fn judge(p: &IsoParams, step: u8, ev: &Evidence) -> Verdict {
    if !ev.r.is_finite() {
        return Verdict::Untested;
    }
    if ev.r < p.min_corr {
        return Verdict::RejectedCorr;
    }
    let q = ev.ratio_med;
    if !q.is_finite() {
        return Verdict::Untested;
    }
    if step == 1 {
        if q >= p.spill_min && q <= p.spill_max {
            return Verdict::Spillover;
        }
        if q < p.ratio_min || q > p.ratio_max {
            return Verdict::RejectedRatio;
        }
    } else if q > p.m2_max {
        return Verdict::RejectedRatio;
    }
    Verdict::Isotope
}

#[derive(Debug, Clone, Default)]
pub struct IsoCounts {
    pub pairs: usize,
    pub isotope: usize,
    pub spillover: usize,
    pub rejected_corr: usize,
    pub rejected_ratio: usize,
    pub untested: usize,
    pub groups: usize,
    pub grouped_rows: usize,
}

/// The grouping of the whole table: per row the id-free row index of the monoisotopic row (`group_of`), the step (0 for M) and, for a flagged +1
/// channel, the row of the M it spills from.
#[derive(Debug, Clone, Default)]
pub struct IsoFound {
    pub pairs: Vec<IsoPair>,
    pub counts: IsoCounts,
    pub group_of: Vec<Option<usize>>,
    pub step_of: Vec<Option<u8>>,
    pub spill_of: Vec<Option<usize>>,
}

/// Groups the accepted pairs: rows are taken from the lightest m/z up; a row joins the best accepted lighter row (highest r, then the lower row) that is not
/// itself a heavier member, so a chain M, M+1, M+2 never makes an M+3. `mz` and `n` come from the table.
pub fn group(mut pairs: Vec<IsoPair>, mz: &[f64], n: usize) -> IsoFound {
    let key = |p: &IsoPair| (p.a, p.b, p.step);
    crate::sortc::any_by(&mut pairs, |x, y| key(x).cmp(&key(y)));
    let mut out = IsoFound {
        group_of: vec![None; n],
        step_of: vec![None; n],
        spill_of: vec![None; n],
        ..IsoFound::default()
    };
    let c = &mut out.counts;
    c.pairs = pairs.len();
    for p in &pairs {
        match p.verdict {
            Verdict::Isotope => c.isotope += 1,
            Verdict::Spillover => c.spillover += 1,
            Verdict::RejectedCorr => c.rejected_corr += 1,
            Verdict::RejectedRatio => c.rejected_ratio += 1,
            Verdict::Untested => c.untested += 1,
        }
    }
    let mut order: Vec<usize> = (0..n).collect();
    crate::sortc::any_by(&mut order, |&i, &j| {
        crate::sortc::asc_nan_last(mz[i], mz[j]).then(i.cmp(&j))
    });
    let mut parent: Vec<Option<usize>> = vec![None; n];
    for &j in &order {
        let mut best: Option<(f64, usize)> = None;
        let mut spill: Option<(f64, usize)> = None;
        for p in pairs.iter().filter(|p| p.b == j) {
            let slot = match p.verdict {
                Verdict::Isotope if parent[p.a].is_none() => &mut best,
                Verdict::Spillover if parent[p.a].is_none() => &mut spill,
                _ => continue,
            };
            if slot.is_none_or(|(r, a)| p.ev.r > r || (p.ev.r == r && p.a < a)) {
                *slot = Some((p.ev.r, p.a));
            }
        }
        if let Some((_, a)) = best {
            parent[j] = Some(a);
        } else if let Some((_, a)) = spill {
            out.spill_of[j] = Some(a);
        }
    }
    let mut has_member = vec![false; n];
    for &a in parent.iter().flatten() {
        has_member[a] = true;
    }
    for j in 0..n {
        if let Some(a) = parent[j] {
            out.group_of[j] = Some(a);
            out.step_of[j] = Some(libm::round(mz[j] - mz[a]).clamp(1.0, 2.0) as u8);
        } else if has_member[j] {
            out.group_of[j] = Some(j);
            out.step_of[j] = Some(0);
        }
    }
    out.counts.groups = has_member.iter().filter(|h| **h).count();
    out.counts.grouped_rows = out.group_of.iter().flatten().count();
    out.pairs = pairs;
    out
}
