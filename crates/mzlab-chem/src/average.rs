//! Boltzmann average of the charges and Fukui values over the conformers (`average:"boltzmann"` of `reactivity_json`); used by
//! `reatt::compute`. Weights come from the force-field single-point energy of each ETKDG geometry.

use crate::embed::Conf;
use crate::fukui::Fukui;
use crate::mol3d::Mol3d;
use crate::reatt::{gfn2, Gfn2, Result3, SPREAD_CONFORMERS};

/// kT at 298.15 K in kcal/mol, the unit of `Conf::energy`.
const KT_KCAL: f64 = 0.592_5;

/// Weights ∝ exp(−ΔE/kT) from the conformer energies (kcal/mol, lowest first); `None` entries get weight 0.
fn boltzmann(confs: &[Conf], ok: &[bool]) -> Vec<f64> {
    let w: Vec<f64> = confs
        .iter()
        .zip(ok)
        .map(|(c, &k)| {
            if k {
                (-(c.energy - confs[0].energy) / KT_KCAL).exp()
            } else {
                0.0
            }
        })
        .collect();
    let sum: f64 = w.iter().sum();
    w.iter().map(|v| v / sum).collect()
}

/// Weighted mean of per-conformer vectors.
fn mean(rows: &[Option<&Vec<f64>>], w: &[f64]) -> Vec<f64> {
    let n = rows.iter().flatten().next().map_or(0, |r| r.len());
    (0..n)
        .map(|i| {
            rows.iter()
                .zip(w)
                .map(|(r, w)| r.map_or(0.0, |r| r[i]) * w)
                .sum()
        })
        .collect()
}

/// Fukui values, HOMO and LUMO averaged over every conformer whose three SCCs converge; the first one (shown geometry) must.
pub(crate) fn averaged(m: &Mol3d, confs: &[Conf], water: bool, first: Gfn2) -> Result3 {
    let mut runs: Vec<Option<Gfn2>> = vec![Some(first)];
    for c in &confs[1..] {
        runs.push(
            gfn2(m, &c.xyz, water)
                .ok()
                .filter(|r| r.3.iter().all(|&k| k)),
        );
    }
    let ok: Vec<bool> = runs.iter().map(Option::is_some).collect();
    let w = boltzmann(confs, &ok);
    let pick = |f: fn(&Fukui) -> Vec<f64>| -> Vec<f64> {
        let v: Vec<Option<Vec<f64>>> = runs.iter().map(|r| r.as_ref().map(|r| f(&r.0))).collect();
        mean(&v.iter().map(Option::as_ref).collect::<Vec<_>>(), &w)
    };
    let (q, fp, fm) = (
        pick(|f| f.q.iter().map(|v| v.unwrap_or(0.0)).collect()),
        pick(|f| f.f_plus.clone()),
        pick(|f| f.f_minus.clone()),
    );
    let sum = |g: fn(&Gfn2) -> f64| -> f64 {
        runs.iter()
            .zip(&w)
            .map(|(r, w)| r.as_ref().map_or(0.0, g) * w)
            .sum()
    };
    let f_zero = fp
        .iter()
        .zip(&fm)
        .map(|(p, m)| Some(0.5 * (p + m)))
        .collect();
    let rows: Vec<&Gfn2> = runs.iter().flatten().collect();
    let spread = (rows.len() > 1).then(|| {
        (0..fm.len())
            .map(|i| {
                let v = rows.iter().take(SPREAD_CONFORMERS).map(|r| r.0.f_minus[i]);
                v.clone().fold(f64::MIN, f64::max) - v.fold(f64::MAX, f64::min)
            })
            .fold(0.0, f64::max)
    });
    let first = runs[0].as_ref().expect("first conformer");
    Result3 {
        method: "GFN2-xTB",
        fukui: Fukui {
            q: q.into_iter().map(Some).collect(),
            f_plus: fp,
            f_zero,
            f_minus: fm,
        },
        homo_ev: sum(|r| r.1),
        lumo_ev: sum(|r| r.2),
        converged: first.3,
        iterations: first.4,
        spread,
        weights: Some(w),
    }
}
