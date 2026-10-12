//! The kinetics of one identity as a piece of evidence for the graph: controls (absent at t0, in the dark and in the blank), shape, stability,
//! coupling with the decay of the parent and generation. The evidence supports the kinship parent -> product, never the structure of the
//! product. A coherent kinetics is a weak proof (type C): `POINTS_MAX` is the weight until measurements show how much it separates.

use super::forms::{choice, Form, FormBank};
use super::generation::{generation, ChainBank, Gen, Generation, MARGIN};
use super::resample::{Resample, Stability};
use super::saturation::parent_saturation;

/// Points a kinetic proof adds to the graph: 0 = informative only (decision of K1, measured on the synthetic identity series V1, see
/// `tests/kinetic_v1.rs`). Shape, stability and generation are recovered (shape right in 96 % of the products above 10 times the noise,
/// generation right or "undecidable" in 99 %), but the coupling with the parent does not separate the true products from random-kinetics decoys
/// when V1 is built as it is (formation rates independent of the parent: 84 % of the products "against", the decoys 48 %); it separates only
/// when the generator assumes the coupling it tests (92 % "for" against 16 %), which proves nothing. At most 1 once the real series (timolol)
/// show how much it separates. The record carries `points_if_weighed` so the evidence layer can switch it on.
pub const POINTS_MAX: f64 = 0.0;
/// Weight a coherent (or incoherent) kinetics would carry: the upper bound of the rule (1 at most).
pub const POINTS_IF_WEIGHED: f64 = 1.0;
/// Tolerance of the coupling: a rate is compatible with the parent's when it is within this factor.
pub const COUPLING_FACTOR: f64 = 2.0;
/// Factor over the reference of `reference_difference` for "reduced" (not absent) in the controls.
pub const CONTROL_FACTOR: f64 = 3.0;
pub const KIND: &str = "cinetica";
pub const SOURCE: &str = "mzlab-feat/kinetic";

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Sign {
    For,
    Against,
    Info,
}

impl Sign {
    pub fn name(self) -> &'static str {
        match self {
            Sign::For => "for",
            Sign::Against => "against",
            Sign::Info => "info",
        }
    }
}

/// Record for the evidence layer: `value` is the distance `|ln(k_form / k_parent)|` of the coupling against the `threshold` `ln(COUPLING_FACTOR)`
/// (0 when there is no coupling); `params` are numbers only (codes in `Report`).
#[derive(Clone, Debug)]
pub struct Evidence {
    pub kind: &'static str,
    pub value: f64,
    pub threshold: f64,
    pub points: f64,
    pub sign: Sign,
    pub source: &'static str,
    pub version: u32,
    pub params: Vec<(&'static str, f64)>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Controls {
    Absent,
    Reduced,
    Present,
    NoSignal,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Coupling {
    Coherent,
    TooFast,
    TooSlow,
    Unresolved,
    Unknown,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ParentSource {
    Direct,
    Isotope,
    AfterPlateau,
    Unavailable,
}

/// Decay of the parent: `k` in 1/min when the decay shape wins, `saturated` when the areas of the main ion saturate.
#[derive(Clone, Copy, Debug)]
pub struct ParentKinetics {
    pub k: Option<f64>,
    pub saturated: bool,
    pub source: ParentSource,
}

fn decay_rate(forms: &FormBank, y: &[f64], cols: &[usize]) -> Option<f64> {
    let fits = forms.fit_cols(y, cols);
    let best = fits.first()?;
    (best.form == Form::Decreasing).then_some(best.rates[0])
}

/// Apparent decay rate (1/min) of a window: minus the slope of `ln y` against time over its positive points; `None` under 3 of them.
fn window_rate(times: &[f64], y: &[f64], from: usize, to: usize) -> Option<f64> {
    let pts: Vec<(f64, f64)> = (from..to)
        .filter(|&i| y[i] > 0.0)
        .map(|i| (times[i], libm::log(y[i])))
        .collect();
    let n = pts.len() as f64;
    let (mt, my) = (
        pts.iter().map(|p| p.0).sum::<f64>() / n,
        pts.iter().map(|p| p.1).sum::<f64>() / n,
    );
    let var: f64 = pts.iter().map(|p| (p.0 - mt) * (p.0 - mt)).sum();
    (pts.len() >= 3 && var > 0.0)
        .then(|| -pts.iter().map(|p| (p.0 - mt) * (p.1 - my)).sum::<f64>() / var)
}

/// A plain first-order decay spanning more than a decade also looks like "a plateau and then a collapse" to the rule of `parent_saturation`
/// (areas only). A real saturation is flat while the ion is being consumed: the plateau must decay at less than a third of the rate after it.
fn is_saturated(times: &[f64], y: &[f64]) -> bool {
    let sat = parent_saturation(y);
    if !sat.saturated {
        return false;
    }
    let (inside, after) = (
        window_rate(times, y, sat.plateau.0, sat.plateau.1),
        window_rate(times, y, sat.plateau.1, y.len()),
    );
    match (inside, after) {
        (Some(i), Some(a)) => a > 0.0 && i < a / 3.0,
        _ => true,
    }
}

/// Decay constant of the parent. Saturated main ion (flat apex or constant area, then collapse): the isotopologue series `y_isotope` if it is not
/// saturated, else only the times after the plateau; never the plateau.
pub fn parent_kinetics(
    forms: &FormBank,
    times: &[f64],
    y: &[f64],
    y_isotope: Option<&[f64]>,
) -> ParentKinetics {
    let all: Vec<usize> = (0..y.len()).collect();
    if !is_saturated(times, y) {
        return ParentKinetics {
            k: decay_rate(forms, y, &all),
            saturated: false,
            source: ParentSource::Direct,
        };
    }
    if let Some(iso) = y_isotope.filter(|iso| !is_saturated(times, iso)) {
        return ParentKinetics {
            k: decay_rate(forms, iso, &all),
            saturated: true,
            source: ParentSource::Isotope,
        };
    }
    let after: Vec<usize> = (parent_saturation(y).plateau.1..y.len()).collect();
    let k = if after.len() >= 3 {
        decay_rate(forms, y, &after)
    } else {
        None
    };
    ParentKinetics {
        k,
        saturated: true,
        source: if k.is_some() {
            ParentSource::AfterPlateau
        } else {
            ParentSource::Unavailable
        },
    }
}

/// What the kinetics of one identity says, with the numbers behind it.
#[derive(Clone, Debug)]
pub struct Report {
    pub shape: Option<Form>,
    pub gap: f64,
    pub stability: Option<Stability>,
    pub generation: Option<Generation>,
    pub controls: Controls,
    pub log2fc: f64,
    pub coupling: Coupling,
    pub k_form: f64,
    pub evidence: Evidence,
}

pub struct Setup<'a> {
    pub times: &'a [f64],
    pub forms: &'a FormBank,
    pub chain: &'a ChainBank,
    pub parent: ParentKinetics,
    /// Noise of the areas (3 x the median of the positive blank areas, as `reference_difference`).
    pub pseudo: f64,
}

fn controls_of(s: &Setup, y: &[f64], ctrl: &[f64]) -> (Controls, f64) {
    let at0 = s.times.iter().position(|&t| t <= 0.0).map_or(0.0, |i| y[i]);
    let reference = ctrl.iter().copied().fold(at0, f64::max);
    let treated = s
        .times
        .iter()
        .zip(y)
        .filter(|(&t, _)| t > 0.0)
        .fold(0.0f64, |m, (_, &v)| m.max(v));
    let fc = libm::log2((treated + s.pseudo) / (reference + s.pseudo));
    let c = if treated <= s.pseudo {
        Controls::NoSignal
    } else if reference <= s.pseudo {
        Controls::Absent
    } else if treated >= CONTROL_FACTOR * reference {
        Controls::Reduced
    } else {
        Controls::Present
    };
    (c, fc)
}

/// Compares the rate(s) of a shape with the decay of the parent: the closest rate decides.
fn couple(s: &Setup, shape: Form, rates: [f64; 2]) -> (Coupling, f64, f64) {
    let (Some(kp), true) = (s.parent.k, matches!(shape, Form::Formation | Form::Bateman)) else {
        return (Coupling::Unknown, 0.0, 0.0);
    };
    let cands: &[f64] = if shape == Form::Bateman {
        &rates
    } else {
        &rates[..1]
    };
    let near = cands
        .iter()
        .copied()
        .min_by(|a, b| libm::fabs(libm::log(a / kp)).total_cmp(&libm::fabs(libm::log(b / kp))))
        .unwrap_or(kp);
    let d = libm::log(near / kp);
    let c = if libm::fabs(d) <= libm::log(COUPLING_FACTOR) {
        Coupling::Coherent
    } else if d > 0.0 {
        Coupling::TooFast
    } else {
        Coupling::TooSlow
    };
    (c, near, libm::fabs(d))
}

fn code<T: PartialEq>(v: T, all: &[T]) -> f64 {
    all.iter().position(|x| *x == v).map_or(-1.0, |i| i as f64)
}

/// Report and evidence record of the kinetics of `y` (area at `s.times`; t0 is the point with time 0) with the controls `ctrl` (blank, dark).
pub fn kinetic_report(s: &Setup, y: &[f64], ctrl: &[f64]) -> Report {
    let (controls, log2fc) = controls_of(s, y, ctrl);
    let all: Vec<usize> = (0..y.len()).collect();
    let fits = s.forms.fit_cols(y, &all);
    let (shape, gap) = choice(&fits).map_or((None, 0.0), |(f, g)| (Some(f), g));
    let stability = s.forms.stability(y, Resample::LeaveOneOut);
    let gener = generation(s.forms, s.chain, y, MARGIN);
    let stable = stability.is_some_and(|st| st.stable());
    let (coupling, k_form, dist) = shape.map_or((Coupling::Unknown, 0.0, 0.0), |f| {
        couple(s, f, fits[0].rates)
    });
    let first_t = s
        .times
        .iter()
        .copied()
        .filter(|&t| t > 0.0)
        .fold(f64::INFINITY, f64::min);
    let instant = shape == Some(Form::Formation)
        && 1.0 - libm::exp(-k_form.max(fits.first().map_or(0.0, |f| f.rates[0])) * first_t) >= 0.9;
    let coupling = if instant {
        Coupling::Unresolved
    } else {
        coupling
    };
    let second = gener.is_some_and(|g| g.verdict == Gen::Second);
    let formed = matches!(controls, Controls::Absent | Controls::Reduced) && stable;
    let sign = match (formed, coupling) {
        (true, Coupling::Coherent) if !second => Sign::For,
        (true, Coupling::TooFast) => Sign::Against,
        _ if controls == Controls::Present => Sign::Against,
        _ => Sign::Info,
    };
    let would = if sign == Sign::Info {
        0.0
    } else {
        POINTS_IF_WEIGHED
    };
    let points = would.min(POINTS_MAX);
    let params = vec![
        ("shape", shape.map_or(-1.0, |f| f as u8 as f64)),
        ("gap_aicc", if gap.is_finite() { gap } else { 1e9 }),
        ("stability", stability.map_or(0.0, |st| st.share())),
        (
            "generation",
            gener.map_or(-1.0, |g| {
                code(g.verdict, &[Gen::First, Gen::Second, Gen::Undecidable])
            }),
        ),
        ("generation_delta", gener.map_or(0.0, |g| g.delta)),
        ("k_form", k_form),
        ("k_parent", s.parent.k.unwrap_or(0.0)),
        (
            "controls",
            code(
                controls,
                &[
                    Controls::Absent,
                    Controls::Reduced,
                    Controls::Present,
                    Controls::NoSignal,
                ],
            ),
        ),
        ("log2fc_controls", log2fc),
        (
            "coupling",
            code(
                coupling,
                &[
                    Coupling::Coherent,
                    Coupling::TooFast,
                    Coupling::TooSlow,
                    Coupling::Unresolved,
                    Coupling::Unknown,
                ],
            ),
        ),
        ("parent_saturated", s.parent.saturated as u8 as f64),
        (
            "parent_source",
            code(
                s.parent.source,
                &[
                    ParentSource::Direct,
                    ParentSource::Isotope,
                    ParentSource::AfterPlateau,
                    ParentSource::Unavailable,
                ],
            ),
        ),
        ("points_if_weighed", would),
        ("n_points", y.len() as f64),
    ];
    let evidence = Evidence {
        kind: KIND,
        value: dist,
        threshold: libm::log(COUPLING_FACTOR),
        points,
        sign,
        source: SOURCE,
        version: super::KINETIC_VERSION,
        params,
    };
    Report {
        shape,
        gap,
        stability,
        generation: gener,
        controls,
        log2fc,
        coupling,
        k_form,
        evidence,
    }
}

/// The record `{kind, value, threshold, points, sign, source, version, params}` that the evidence layer consumes.
pub fn evidence_from_kinetics(s: &Setup, y: &[f64], ctrl: &[f64]) -> Evidence {
    kinetic_report(s, y, ctrl).evidence
}
