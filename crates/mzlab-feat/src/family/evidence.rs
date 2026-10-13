//! Evidence on the link between two co-eluting ions: port of `TP_Mine/py/tpmine/hr/edge_evidence.py::edge_evidence`.
//!
//! For the pair (X lighter, Y heavier) the readings of the step (`rel`) are checked against the MS2 of the two ions and the ratio of the
//! areas over the samples, and the pair gets a label that is a signal for the chemist, never a verdict: adduct, in-source fragment (only
//! with the proof of the MS2 of Y: the m/z of X is among its fragments) or not distinguishable. Every piece of evidence is a `Proof`.
//! A file alone has no ratio over samples: `Ratio::Unavailable` does not oppose the fragment, `Ratio::Unstable` and too few samples do.

use super::rel::{RKind, Reading, Tables};

impl Default for Rules {
    fn default() -> Self {
        Rules {
            ppm: 5.0,
            abs_da: 0.002,
            coelution_min: 0.05,
            cv_stable: 0.30,
            min_samples: 3,
            skeleton_min: 0.5,
            max_peaks: 40,
            unlisted: "ms2",
            fragment_without_ratio: true,
        }
    }
}

impl Rules {
    /// Unit resolution: the tolerance is in tenths of a dalton, the steps are nominal and an edge needs an MS2 to be more than "not determined".
    pub fn nominal(&self) -> bool {
        self.abs_da >= 0.1
    }

    pub fn tol(&self, mz: f64) -> f64 {
        self.abs_da.max(mz.abs() * self.ppm * 1e-6)
    }

    /// The strongest `max_peaks` peaks (`_spec`), in ascending m/z: None for an empty spectrum.
    pub fn strongest(&self, mz: &[f64], it: &[f64]) -> Option<Spec> {
        if mz.is_empty() || mz.len() != it.len() {
            return None;
        }
        let mut k: Vec<usize> = (0..mz.len()).collect();
        if mz.len() > self.max_peaks {
            crate::sortc::by(&mut k, |&a, &b| it[b].total_cmp(&it[a]));
            k.truncate(self.max_peaks);
        }
        crate::sortc::by(&mut k, |&a, &b| mz[a].total_cmp(&mz[b]).then(a.cmp(&b)));
        Some(Spec {
            mz: k.iter().map(|&i| mz[i]).collect(),
            it: k.iter().map(|&i| it[i]).collect(),
        })
    }

    /// The peak of `s` nearest to `mz` within the tolerance, the precursor's own excluded: its relative intensity in %.
    pub(crate) fn near(&self, s: &Spec, mz: f64, prec: f64) -> Option<f64> {
        let t = 1.5 * self.tol(prec);
        let mut best: Option<(f64, f64)> = None;
        let mut top = f64::MIN;
        for (&a, &i) in s.mz.iter().zip(&s.it) {
            if (a - prec).abs() <= t {
                continue;
            }
            top = top.max(i);
            if best.is_none_or(|(d, _)| (a - mz).abs() < d) {
                best = Some(((a - mz).abs(), i));
            }
        }
        let (d, i) = best?;
        (d <= self.tol(mz)).then(|| 100.0 * i / top.max(1e-300))
    }

    fn peaks_besides(&self, s: &Spec, prec: f64) -> usize {
        s.mz.iter()
            .filter(|&&a| (a - prec).abs() > 1.5 * self.tol(prec))
            .count()
    }

    /// Share (by square-root intensity) of the fragments of X that Y also has, at the same m/z or shifted by the step: (n, shared, fraction).
    fn skeleton(
        &self,
        lo: &Spec,
        up: &Spec,
        mz_lo: f64,
        mz_up: f64,
    ) -> (usize, usize, Option<f64>) {
        let shift = mz_up - mz_lo;
        let (mut n, mut sh, mut tot, mut shared) = (0usize, 0usize, 0.0f64, 0.0f64);
        for (&m, &i) in lo.mz.iter().zip(&lo.it) {
            if (m - mz_lo).abs() <= 1.5 * self.tol(mz_lo) {
                continue;
            }
            n += 1;
            let w = i.sqrt();
            tot += w;
            let t = self.tol(m);
            if up
                .mz
                .iter()
                .any(|&a| (a - m).abs() <= t || (a - (m + shift)).abs() <= t)
            {
                shared += w;
                sh += 1;
            }
        }
        (n, sh, (tot > 0.0).then(|| shared / tot))
    }
}

/// Version of the rules that give the proofs.
pub const PROOF_VERSION: u32 = 1;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProofKind {
    /// The step is in a table (information).
    Step,
    Coelution,
    /// The m/z of X among the fragments of the MS2 of Y.
    FragmentInMs2,
    /// The ratio Y/X over the samples.
    Ratio,
    /// The fragments of X also in Y.
    Skeleton,
    /// The typical loss of the adduct in the MS2 of Y.
    TypicalLoss,
    NoFragmentation,
}

/// A piece of evidence: type, measured value, threshold, points (signed weight), sign (+1 supports the reading, -1 opposes it, 0 information
/// or hint) and source. The version and the parameters belong to the whole set of families (`Families`).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Proof {
    pub kind: ProofKind,
    pub value: f32,
    pub threshold: f32,
    pub points: f32,
    pub sign: i8,
    pub source: &'static str,
}

const SOURCE: &str = "edge_evidence";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EdgeKind {
    Adduct,
    Isf,
    Undetermined,
    /// The step is none of the tables.
    Other,
}

/// What is known of the ratio Y/X over the samples.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Ratio {
    Unavailable,
    /// `n` common samples, coefficient of variation (None: not finite).
    Over {
        n: u32,
        cv: Option<f64>,
    },
}

/// Strongest peaks of a spectrum (m/z, intensity), in the order `edge_evidence._spec` keeps them.
#[derive(Debug, Clone, Default)]
pub struct Spec {
    pub mz: Vec<f64>,
    pub it: Vec<f64>,
}

#[derive(Debug, Clone)]
pub struct Rules {
    pub ppm: f64,
    pub abs_da: f64,
    pub coelution_min: f64,
    pub cv_stable: f64,
    pub min_samples: u32,
    pub skeleton_min: f64,
    pub max_peaks: usize,
    /// Name of the reading of a loss that is in none of the tables but whose fragment is in the MS2 of the heavier ion.
    pub unlisted: &'static str,
    /// Declare a fragment on the proof of the MS2 alone when the ratio is `Unavailable` (one file).
    pub fragment_without_ratio: bool,
}

/// Coefficient of variation (sample deviation over |mean|) of the ratios up/lo over the samples where both are positive, as `ratio_stats`.
pub fn ratio_of(lo: &[f64], up: &[f64], r: &Rules) -> Ratio {
    let v: Vec<f64> = lo
        .iter()
        .zip(up)
        .filter(|(a, b)| **a > 0.0 && **b > 0.0)
        .map(|(a, b)| b / a)
        .collect();
    let n = v.len() as u32;
    if n < r.min_samples {
        return Ratio::Over { n, cv: None };
    }
    let mean = v.iter().sum::<f64>() / v.len() as f64;
    let var = v.iter().map(|x| (x - mean) * (x - mean)).sum::<f64>() / (v.len() - 1) as f64;
    let cv = (mean != 0.0)
        .then(|| var.sqrt() / mean.abs())
        .filter(|c| c.is_finite());
    Ratio::Over { n, cv }
}

fn proof(kind: ProofKind, value: f64, threshold: f64, points: f32, sign: i8) -> Proof {
    Proof {
        kind,
        value: value as f32,
        threshold: threshold as f32,
        points: points * sign as f32,
        sign,
        source: SOURCE,
    }
}

/// Pearson-free part of the label: the evidence that the heavier ion is an adduct (or dimer) of the lighter one, from the two MS2 and the
/// typical loss. Returns `(supports, opposes, typical loss seen)`.
fn adduct_evidence(
    t: &Tables,
    r: &Rules,
    p: &Pair,
    name: &str,
    ev: &mut Vec<Proof>,
) -> (bool, bool, bool) {
    let (mut pos, mut neg, mut tl_hit) = (false, false, false);
    if let (Some(lo), Some(up)) = (p.ms2_lo, p.ms2_up) {
        if let (_, _, Some(f)) = r.skeleton(lo, up, p.mz_lo, p.mz_up) {
            let ok = f >= r.skeleton_min;
            (pos, neg) = (ok, !ok);
            ev.push(proof(
                ProofKind::Skeleton,
                f,
                r.skeleton_min,
                1.0,
                if ok { 1 } else { -1 },
            ));
        }
    }
    if let (true, Some(up)) = (name == super::rel::NH4_NAME, p.ms2_up) {
        let lm = t.nh3();
        let hit = r.near(up, p.mz_up - lm, p.mz_up);
        if hit.is_some() {
            (pos, neg, tl_hit) = (true, false, true);
        } else {
            neg = neg || p.ms2_lo.is_none();
        }
        ev.push(proof(
            ProofKind::TypicalLoss,
            hit.unwrap_or(0.0),
            lm,
            2.0,
            if hit.is_some() { 1 } else { -1 },
        ));
    }
    if let (true, Some(up)) = (
        matches!(name, "[M+Na]+" | "[M+K]+" | "[M+Na-2H]-"),
        p.ms2_up,
    ) {
        if r.peaks_besides(up, p.mz_up) == 0 {
            ev.push(proof(ProofKind::NoFragmentation, 0.0, 0.0, 0.5, 0));
        }
    }
    (pos, neg, tl_hit)
}

#[derive(Debug, Clone)]
pub struct Edge {
    pub kind: EdgeKind,
    pub readings: Vec<Reading>,
    pub proofs: Vec<Proof>,
}

/// Port of `edge_evidence.edge_evidence` (without the texts). A step in none of the tables whose lighter ion is a fragment in the MS2 of the
/// heavier one is read as a loss named `Rules::unlisted`: the neutral loss does not matter when the MS2 shows the fragment.
pub struct Pair<'a> {
    pub mz_lo: f64,
    pub mz_up: f64,
    pub ms2_lo: Option<&'a Spec>,
    pub ms2_up: Option<&'a Spec>,
    pub ratio: Ratio,
    pub drt_min: Option<f64>,
}

/// Readings of the step between the ends, isotopes apart; an unlisted loss when the lighter ion is a fragment in the MS2 of the heavier.
fn readings_of(t: &Tables, r: &Rules, p: &Pair) -> Vec<Reading> {
    let mut readings: Vec<Reading> = t
        .readings(p.mz_lo, p.mz_up, r.ppm, r.abs_da)
        .into_iter()
        .filter(|x| x.kind != RKind::Isotope)
        .collect();
    if readings.is_empty()
        && p.ms2_up
            .is_some_and(|s| r.near(s, p.mz_lo, p.mz_up).is_some())
    {
        let step = p.mz_up - p.mz_lo;
        readings.push(Reading {
            kind: RKind::Loss,
            name: r.unlisted.to_string(),
            step,
            ppm: 0.0,
        });
    }
    readings
}

/// Proofs of an ISF reading (fragment in the MS2 of the heavier ion, ratio); true when it stands.
fn loss_proofs(r: &Rules, p: &Pair, ev: &mut Vec<Proof>) -> bool {
    let frag = p.ms2_up.and_then(|s| r.near(s, p.mz_lo, p.mz_up));
    if p.ms2_up.is_some() {
        ev.push(proof(
            ProofKind::FragmentInMs2,
            frag.unwrap_or(0.0),
            0.0,
            2.0,
            if frag.is_some() { 1 } else { -1 },
        ));
    }
    let ratio_ok = match p.ratio {
        Ratio::Unavailable => r.fragment_without_ratio,
        Ratio::Over { cv, .. } => cv.is_some_and(|c| c <= r.cv_stable),
    };
    frag.is_some() && ratio_ok
}

/// Label of an edge from the kinds of reading and the adduct (pos, neg) and ISF proofs.
fn decide(has_adduct: bool, has_loss: bool, (pos, neg, isf_ok): (bool, bool, bool)) -> EdgeKind {
    match (has_adduct, has_loss) {
        (true, false) => {
            if neg && !pos {
                EdgeKind::Undetermined
            } else {
                EdgeKind::Adduct
            }
        }
        (false, true) => {
            if isf_ok {
                EdgeKind::Isf
            } else {
                EdgeKind::Undetermined
            }
        }
        _ => {
            if pos && !isf_ok {
                EdgeKind::Adduct
            } else if isf_ok && !pos {
                EdgeKind::Isf
            } else {
                EdgeKind::Undetermined
            }
        }
    }
}

pub fn edge_evidence(t: &Tables, r: &Rules, p: &Pair) -> Edge {
    let readings = readings_of(t, r, p);
    let adduct_rels: Vec<&Reading> = readings
        .iter()
        .filter(|x| matches!(x.kind, RKind::Adduct | RKind::Dimer))
        .collect();
    let loss_rels: Vec<&Reading> = readings.iter().filter(|x| x.kind == RKind::Loss).collect();
    let mut ev: Vec<Proof> = Vec::new();
    if adduct_rels.is_empty() && loss_rels.is_empty() {
        return Edge {
            kind: EdgeKind::Other,
            readings,
            proofs: ev,
        };
    }
    for x in &readings {
        ev.push(proof(ProofKind::Step, x.ppm, r.ppm, 0.0, 0));
    }
    if let Some(d) = p.drt_min {
        let ok = d <= r.coelution_min;
        ev.push(proof(
            ProofKind::Coelution,
            d,
            r.coelution_min,
            1.0,
            if ok { 1 } else { -1 },
        ));
    }
    let isf_ok = !loss_rels.is_empty() && loss_proofs(r, p, &mut ev);
    if let Ratio::Over { cv: Some(c), .. } = p.ratio {
        ev.push(proof(
            ProofKind::Ratio,
            c,
            r.cv_stable,
            2.0,
            if c <= r.cv_stable { 1 } else { -1 },
        ));
    }
    let mut isf_ok = isf_ok;
    let (pos, neg, tl_hit) = match adduct_rels.first() {
        Some(a) => adduct_evidence(t, r, p, &a.name, &mut ev),
        None => (false, false, false),
    };
    if tl_hit && loss_rels.iter().all(|x| x.name == super::rel::NH3_NAME) {
        isf_ok = false;
    }
    let mut kind = decide(
        !adduct_rels.is_empty(),
        !loss_rels.is_empty(),
        (pos, neg, isf_ok),
    );
    if r.nominal()
        && p.ms2_lo.is_none()
        && p.ms2_up.is_none()
        && matches!(kind, EdgeKind::Adduct | EdgeKind::Isf)
    {
        kind = EdgeKind::Undetermined;
    }
    Edge {
        kind,
        readings,
        proofs: ev,
    }
}
