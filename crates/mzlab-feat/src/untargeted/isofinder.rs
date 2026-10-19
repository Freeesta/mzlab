//! «Isotopic peaks finder» of mzmine 4.10.90: from the spectrum of a feature, the isotope pattern with its charge and the reasons for it.
//!
//! After `filter_isotopefinder/engine/IsotopeFinderEngine.java` with `engine/{CarbonLadder,CarbonRatio,IsotopeEnvelope,OffsetSpan}.java`,
//! `signal/CarbonEnvelopeModel.java` and `util/IsotopesUtils.java` (MIT, (c) 2004-2022 The mzmine Development Team); rewritten, no code copied.
//! Unlike the 13C isotope filter of `isotopes.rs` (which works on the ROWS of the table and removes them) it works on the SPECTRUM of each feature and
//! writes a pattern on it. Here: the candidates of the spectrum, the expected envelope (`envelope`), the ladder on the 13C grid (`ladder`) and the
//! score of ONE charge ([`Engine::score_charge`]) and the choice among the charges (`select`: pattern, alternatives, probabilities). The isotope masses come from `mzlab_core::isoscore::isotopes_of`. Not done: the filter of
//! explainable signals (off by default), the refinement across scans (`CrossScanRefiner`), mobility.

pub mod envelope;
pub mod ladder;
mod score;
pub mod select;

use std::collections::BTreeMap;

use envelope::{jround, m1_bounds, CarbonModel, Envelope};
use ladder::{closest_index, CarbonRatio, Ladder};
use mzlab_core::isoscore::isotopes_of;
use score::{
    collect_emitted, coverage, emitted_offsets, intensity_agreement, kept_offsets,
    self_consistency, spacing_consistency,
};

const MIN_LADDER_PEAKS: usize = 2;
const TIE_WEIGHT: f64 = 0.1;
const NEUTRAL_FALLBACK_WEIGHT: f64 = 0.6;

/// `(m/z, intensity)`.
pub type Pt = (f64, f64);

/// Tolerance of mzmine: the larger of an absolute and a relative (ppm) part (defaults 0.005 m/z or 15 ppm).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Tol {
    pub abs: f64,
    pub ppm: f64,
}

impl Default for Tol {
    fn default() -> Self {
        Tol {
            abs: 0.005,
            ppm: 15.0,
        }
    }
}

impl Tol {
    pub fn for_mass(&self, mz: f64) -> f64 {
        self.abs.max(mz / 1e6 * self.ppm)
    }
    /// `b` within the tolerance of `a` (the relative part is taken on `a`).
    pub fn within(&self, a: f64, b: f64) -> bool {
        self.within_factor(a, b, 1.0)
    }
    pub fn within_factor(&self, a: f64, b: f64, factor: f64) -> bool {
        let d = (a - b).abs();
        d <= self.abs * factor || d <= a / 1e6 * self.ppm * factor
    }
}

/// Settings of the finder (mzmine defaults: H C N O S, charge up to 3, «Require 13C isotope peak» on).
#[derive(Clone, Debug, PartialEq)]
pub struct Config {
    pub elements: Vec<String>,
    pub max_charge: i32,
    pub tol: Tol,
    pub require_c13: bool,
    pub model: CarbonModel,
}

impl Default for Config {
    fn default() -> Self {
        Config {
            elements: ["H", "C", "N", "O", "S"].map(String::from).to_vec(),
            max_charge: 3,
            tol: Tol::default(),
            require_c13: true,
            model: CarbonModel::default(),
        }
    }
}

/// The score of one charge, with its parts (all in 0-1 except `raw`).
#[derive(Clone, Debug, PartialEq)]
pub struct ChargeEval {
    pub charge: i32,
    /// Quality x (1 + 0.1 x peaks kept); 0 when the charge does not count. Charges are ranked on it.
    pub raw: f64,
    pub quality: f64,
    pub coverage: f64,
    pub carbon_fit: f64,
    pub self_consistency: f64,
    pub spacing_consistency: f64,
    pub intensity_agreement: f64,
    /// `false` when the ladder had fewer than 2 peaks, so the fit is neutral.
    pub carbon_assessed: bool,
    pub kept: Vec<Pt>,
    pub base_mz: f64,
    pub placement: i64,
}

pub struct Engine {
    cfg: Config,
    elements: Vec<String>,
    diffs: Vec<Vec<f64>>,
    max_diff: Vec<f64>,
}

impl Engine {
    pub fn new(cfg: Config) -> Result<Engine, String> {
        if cfg.max_charge < 1 {
            return Err("the maximum charge must be at least 1".into());
        }
        let diffs = isotope_diffs(&cfg.elements, cfg.max_charge)?;
        if diffs[0].is_empty() {
            return Err("the elements have no natural isotopes".into());
        }
        let max_diff = diffs
            .iter()
            .map(|d| d.last().map_or(0.0, |m| m + 10.0 * cfg.tol.for_mass(*m)))
            .collect();
        let elements = cfg.elements.clone();
        Ok(Engine {
            cfg,
            elements,
            diffs,
            max_diff,
        })
    }

    /// Candidate peaks of the spectrum for the charge `z` (1-based); empty if the charge has no isotope differences.
    pub fn candidates(&self, z: i32, mz: &[f64], y: &[f64], target_mz: f64) -> Vec<Pt> {
        let i = z as usize - 1;
        if self.diffs[i].is_empty() {
            return Vec::new();
        }
        find_candidates(
            &self.diffs[i],
            self.max_diff[i],
            &self.cfg.tol,
            mz,
            y,
            target_mz,
        )
    }

    fn user_heavies(&self) -> Vec<&str> {
        self.elements.iter().map(String::as_str).collect()
    }

    /// Score of charge `z` for the feature at `feature_mz` with `candidates` from [`Engine::candidates`]; `sign` is the polarity (+1, -1, 0 unknown).
    /// `None` when there are too few candidates or the charge does not pass the gates.
    pub fn score_charge(
        &self,
        z: i32,
        feature_mz: f64,
        candidates: &[Pt],
        sign: i32,
    ) -> Option<ChargeEval> {
        if candidates.len() < min_signals(z) {
            return None;
        }
        let model = &self.cfg.model;
        let env = model.build(feature_mz, z, sign, &self.user_heavies(), None, true);
        let bounds = m1_bounds(model, feature_mz, z, sign);
        self.score_with(z, feature_mz, candidates, &env, bounds)
    }
}

/// Peaks needed to accept a charge.
pub fn min_signals(z: i32) -> usize {
    match z {
        10.. => 5,
        6..=9 => 4,
        4..=5 => 3,
        _ => 2,
    }
}

/// Differences `(m_j - m_i) / z` between the natural isotopes of each element, per charge `z = 1..=max_charge` (index `z - 1`), sorted.
fn isotope_diffs(elements: &[String], max_charge: i32) -> Result<Vec<Vec<f64>>, String> {
    let mut out = vec![Vec::new(); max_charge.max(0) as usize];
    for el in elements {
        let iso =
            isotopes_of(el).ok_or_else(|| format!("element {el} is not in the isotope table"))?;
        for i in 0..iso.len() {
            for j in i + 1..iso.len() {
                for z in 1..=max_charge {
                    out[z as usize - 1].push((iso[j].0 - iso[i].0) / z as f64);
                }
            }
        }
    }
    out.iter_mut().for_each(|d| d.sort_by(f64::total_cmp));
    Ok(out)
}

/// Is there a known m/z at `mz + d` (`dir` = +1: `d` added) or `mz - d` (`dir` = -1) for some `d`? `known` is scanned from its end, as in mzmine.
fn possible_isotope(mz: f64, known: &[Pt], diffs: &[f64], tol: &Tol, up: bool) -> bool {
    for d in diffs {
        let theo = if up { mz - d } else { mz + d };
        let t = tol.for_mass(theo);
        for k in known.iter().rev() {
            if up && theo - t > k.0 || !up && k.0 > theo + t {
                break;
            }
            if k.0 >= theo - t && k.0 <= theo + t {
                return true;
            }
        }
    }
    false
}

fn insert_if_new(list: &mut Vec<Pt>, p: Pt) {
    for c in (0..list.len()).rev() {
        if list[c].0 == p.0 {
            return;
        }
        if list[c].0 < p.0 {
            list.insert(c + 1, p);
            return;
        }
    }
}

/// Candidate isotope peaks of the spectrum `(mz, y)` around `target_mz` for one charge: the point nearest to the target, then a walk down from the top
/// of the spectrum (a point is kept if adding a difference of `diffs` lands on a candidate) and a walk up (a point is kept if subtracting one lands on
/// one), both stopping `max_diff` beyond the last accepted point. Ascending in m/z.
pub fn find_candidates(
    diffs: &[f64],
    max_diff: f64,
    tol: &Tol,
    mz: &[f64],
    y: &[f64],
    target_mz: f64,
) -> Vec<Pt> {
    let Some(t) = closest_index(mz, target_mz) else {
        return Vec::new();
    };
    let n = mz.len() as i64;
    let mut cands = vec![(mz[t], y[t])];
    let (mut m, mut last) = (mz[t], mz[t]);
    let mut dp = n - 1;
    while dp >= 0 && m >= last - max_diff {
        m = mz[dp as usize];
        if possible_isotope(m, &cands, diffs, tol, false) {
            cands.push((m, y[dp as usize]));
            last = m;
        }
        dp -= 1;
    }
    cands.sort_by(|a, b| a.0.total_cmp(&b.0));
    m = mz[t];
    let mut max_mz = m;
    dp += 1;
    while dp < n && m <= max_mz + max_diff {
        m = mz[dp as usize];
        if possible_isotope(m, &cands, diffs, tol, true) {
            let p = (m, y[dp as usize]);
            if m > max_mz {
                cands.push(p);
                max_mz = m;
            } else {
                insert_if_new(&mut cands, p);
            }
        }
        dp += 1;
    }
    cands.sort_by(|a, b| a.0.total_cmp(&b.0));
    cands
}

fn most_intense(c: &[Pt]) -> Pt {
    c.iter().fold(c[0], |b, p| if p.1 > b.1 { *p } else { b })
}

fn slide_carbon_fit(ladder: &BTreeMap<i64, f64>, env: &Envelope) -> CarbonFit {
    if ladder.len() < MIN_LADDER_PEAKS {
        return CarbonFit {
            score: 1.0,
            placement: env.base_offset(),
            assessed: false,
        };
    }
    let obs: f64 = ladder.values().map(|v| v * v).sum();
    let pred: f64 = (0..=env.max_offset())
        .map(|o| env.expected_at(o).powi(2))
        .sum();
    if obs <= 0.0 || pred <= 0.0 {
        return CarbonFit {
            score: 0.0,
            placement: env.base_offset(),
            assessed: true,
        };
    }
    let norm = obs.sqrt() * pred.sqrt();
    let (mut best, mut place) = (f64::NEG_INFINITY, env.base_offset());
    for p in 0..=env.max_offset() {
        let dot: f64 = ladder.iter().map(|(k, v)| v * env.expected_at(k + p)).sum();
        if dot > best {
            best = dot;
            place = p;
        }
    }
    CarbonFit {
        score: (best / norm).max(0.0),
        placement: place,
        assessed: true,
    }
}

/// Base of the ladder: the most intense candidate, or — when the chain around the searched peak does not contain it — the most intense of that chain.
struct Setup {
    base: Pt,
    ladder: Ladder,
    cands: Vec<Pt>,
    anchor_offset: i64,
    step: i64,
}

impl Engine {
    /// Base peak, ladder and candidates of one charge; `None` if «Require 13C» finds no gap-free ladder through the base.
    fn setup(&self, candidates: &[Pt], searched_mz: f64, spacing: f64) -> Option<Setup> {
        let tol = self.cfg.tol;
        let searched = candidates
            .iter()
            .filter(|p| tol.within(searched_mz, p.0))
            .fold(None::<Pt>, |b, p| match b {
                Some(q) if (q.0 - searched_mz).abs() <= (p.0 - searched_mz).abs() => Some(q),
                _ => Some(*p),
            });
        let anchor_mz = searched.unwrap_or_else(|| most_intense(candidates)).0;
        let mut base = most_intense(candidates);
        let mut ladder = Ladder::build(candidates, base.0, spacing, tol);
        let mut anchor_off = jround((anchor_mz - base.0) / spacing);
        let cluster = ladder.cluster_span_around(anchor_off, anchor_mz);
        if !cluster.contains(0) && cluster.size() >= 2 {
            base = candidates
                .iter()
                .filter(|p| cluster.contains(jround((p.0 - base.0) / spacing)))
                .fold(None::<Pt>, |b, p| match b {
                    Some(q) if q.1 >= p.1 => Some(q),
                    _ => Some(*p),
                })
                .unwrap_or_else(|| most_intense(candidates));
            ladder = Ladder::build(candidates, base.0, spacing, tol);
            anchor_off = jround((anchor_mz - base.0) / spacing);
        }
        if !self.cfg.require_c13 {
            return Some(Setup {
                base,
                ladder,
                cands: candidates.to_vec(),
                anchor_offset: anchor_off,
                step: 1,
            });
        }
        let span = ladder.require_c13_span()?;
        let crop = span.extend_to(anchor_off);
        let cands: Vec<Pt> = candidates
            .iter()
            .copied()
            .filter(|p| crop.contains(jround((p.0 - base.0) / spacing)))
            .collect();
        let ladder = Ladder::build(&cands, base.0, spacing, tol);
        Some(Setup {
            base,
            ladder,
            cands,
            anchor_offset: anchor_off,
            step: span.step,
        })
    }
}

/// Best fit of the ladder against the expected envelope slid over it: cosine of the best placement (0 or more) and that placement.
struct CarbonFit {
    score: f64,
    placement: i64,
    assessed: bool,
}

impl Engine {
    /// As [`Engine::score_charge`] with an envelope given (the elements found in the spectrum rebuild it).
    pub fn score_with(
        &self,
        z: i32,
        searched_mz: f64,
        candidates: &[Pt],
        env: &Envelope,
        bounds: [f64; 2],
    ) -> Option<ChargeEval> {
        let s = self.setup(candidates, searched_mz, env.spacing)?;
        let observed = s.ladder.collapsed();
        if observed.is_empty() {
            return None;
        }
        let carbon = s.ladder.on_grid_intensities(1.0);
        let fit = slide_carbon_fit(&carbon, env);
        let ratio = CarbonRatio::measure(&carbon, fit.placement);
        if self.cfg.require_c13 && s.step == 1 && ratio.fails_require_c13(bounds) {
            return None;
        }
        let coverage = coverage(env, &observed.keys().copied().collect(), fit.placement);
        let self_c = self_consistency(
            z,
            &observed.iter().map(|(k, p)| (*k, p.intensity)).collect(),
            env,
            fit.placement,
            s.base.1,
        );
        let kept_off = if self.cfg.require_c13 {
            observed.keys().copied().collect()
        } else {
            kept_offsets(&observed, env, fit.placement, s.base.1)
        };
        let emit = emitted_offsets(&kept_off, &observed, s.anchor_offset);
        let kept = collect_emitted(&s.cands, &emit, s.base.0, env.spacing);
        let agreement = intensity_agreement(&kept_off, &observed, env, fit.placement, s.base.1);
        let mut quality = fit.score * coverage;
        if z > 1 {
            quality *= self_c;
        }
        quality *= if fit.assessed {
            ratio.plausibility(bounds)
        } else {
            NEUTRAL_FALLBACK_WEIGHT
        };
        let n_kept = kept_off.len();
        let mut raw = quality * (1.0 + TIE_WEIGHT * n_kept as f64);
        let enough = carbon.len() >= min_signals(z) || (min_signals(z) <= 2 && n_kept >= 2);
        if coverage <= 0.0 || !enough || (z > 1 && self_c <= 0.0) {
            raw = 0.0;
        }
        Some(ChargeEval {
            charge: z,
            raw,
            quality,
            coverage,
            carbon_fit: fit.score,
            self_consistency: self_c,
            spacing_consistency: spacing_consistency(&s.ladder, s.base.0, &self.cfg.tol),
            intensity_agreement: agreement,
            carbon_assessed: fit.assessed,
            kept,
            base_mz: s.base.0,
            placement: fit.placement,
        })
    }
}
