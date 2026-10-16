//! The data the charts of the Banco 2b draw, taken from what `review_feature` already computed: the peaks of the cleaned MS2 with the sub-formula that
//! explains each, the best exact reference as a mirror with its matched pairs, and the isotope pattern of the ion.

use super::explain::Explanation;
use super::feature::Scored;
use super::isotopes::{pattern, round_to};
use super::matching::{compare, Kind, Ms2};
use super::proposal::{FragView, Mirror, PeakView, View, J};
use super::settings::ReviewSettings;

fn rel(it: &[f64]) -> Vec<f64> {
    let top = it.iter().cloned().fold(0.0, f64::max);
    it.iter()
        .map(|v| {
            if top > 0.0 {
                round_to(v / top * 100.0, 2)
            } else {
                0.0
            }
        })
        .collect()
}

/// The mirror of the best exact reference whose spectrum is in the libraries (the list is already ordered by cosine, same structure first).
fn mirror(exp: &Ms2, scored: &[Scored], low: Option<f64>, s: &ReviewSettings) -> Option<Mirror> {
    let (k, sc) = scored
        .iter()
        .enumerate()
        .find(|(_, c)| c.r.kind == "exact" && c.spec.is_some())?;
    let spec = sc.spec.as_ref()?;
    let cmp = compare(exp, spec, s.tol_da, low, 0.0);
    Some(Mirror {
        reference: k,
        peaks: spec.mz.iter().cloned().zip(rel(&spec.it)).collect(),
        pairs: cmp
            .pairs
            .iter()
            .map(|&(a, b, kind)| (a, b, kind == Kind::Shifted))
            .collect(),
    })
}

/// The view of the review of one feature.
#[allow(clippy::too_many_arguments)]
pub(super) fn build(
    mz: &[f64],
    it: &[f64],
    ex: &Explanation,
    exp_q: &Ms2,
    scored: &[Scored],
    low: Option<f64>,
    s: &ReviewSettings,
    pol: &str,
) -> View {
    let peaks = mz
        .iter()
        .zip(rel(it))
        .enumerate()
        .map(|(i, (&m, r))| PeakView {
            mz: m,
            rel: r,
            frag: ex.frags.get(i).and_then(|f| f.as_ref()).map(|f| FragView {
                formula: f.formula.clone(),
                mz_th: f.mz_th,
                ppm: round_to(f.ppm, 2),
                loss: f.loss.clone(),
            }),
        })
        .collect();
    View {
        peaks,
        spectra: scored.iter().map(|c| c.spec.clone()).collect(),
        mirror: mirror(exp_q, scored, low, s),
        isotopes: ex
            .ion
            .as_ref()
            .map(|f| {
                pattern(f, pol, 0.5)
                    .into_iter()
                    .map(|(m, a)| (round_to(m, 5), round_to(a, 2)))
                    .collect()
            })
            .unwrap_or_default(),
    }
}

fn frag_json(f: &FragView) -> J {
    J::Obj(vec![
        ("formula", J::s(&f.formula)),
        ("mz_th", J::Num(f.mz_th)),
        ("ppm", J::on(Some(f.ppm))),
        ("loss", J::s(&f.loss)),
    ])
}

fn pair(a: f64, b: f64) -> J {
    J::Arr(vec![J::Num(a), J::Num(b)])
}

fn mirror_json(m: &Mirror) -> J {
    let pairs = m
        .pairs
        .iter()
        .map(|p| J::Arr(vec![J::Num(p.0 as f64), J::Num(p.1 as f64), J::Bool(p.2)]))
        .collect();
    J::Obj(vec![
        ("reference", J::Num(m.reference as f64)),
        (
            "peaks",
            J::Arr(m.peaks.iter().map(|p| pair(p.0, p.1)).collect()),
        ),
        ("pairs", J::Arr(pairs)),
    ])
}

impl View {
    pub(super) fn json(&self) -> J {
        let peaks = self
            .peaks
            .iter()
            .map(|p| {
                J::Obj(vec![
                    ("mz", J::Num(p.mz)),
                    ("rel", J::Num(p.rel)),
                    ("frag", p.frag.as_ref().map_or(J::Null, frag_json)),
                ])
            })
            .collect();
        J::Obj(vec![
            ("peaks", J::Arr(peaks)),
            ("mirror", self.mirror.as_ref().map_or(J::Null, mirror_json)),
            (
                "isotopes",
                J::Arr(self.isotopes.iter().map(|p| pair(p.0, p.1)).collect()),
            ),
        ])
    }
}
