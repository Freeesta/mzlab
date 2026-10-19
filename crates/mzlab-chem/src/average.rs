//! Boltzmann average of the charges and Fukui values over the conformers (`average:"boltzmann"` of `reactivity_json`); used by
//! `reatt::compute`. Weights come from the force-field single-point energy of each ETKDG geometry.

use crate::embed::Conf;
use crate::fukui::Fukui;
use crate::mol3d::Mol3d;
use crate::reatt::{gfn2, Gfn2, Result3};

/// kT at 298.15 K in kcal/mol, the unit of `Conf::energy`.
const KT_KCAL: f64 = 0.592_5;

/// Fukui values, HOMO and LUMO averaged over every conformer whose three SCCs converge; `first` (the shown geometry) must.
pub(crate) fn averaged(m: &Mol3d, confs: &[Conf], water: bool, first: Gfn2) -> Result3 {
    let (c, it) = (first.3, first.4);
    let mut runs = vec![Some(first)];
    runs.extend(confs[1..].iter().map(|c| {
        gfn2(m, &c.xyz, water)
            .ok()
            .filter(|r| r.3.iter().all(|&k| k))
    }));
    let mut w: Vec<f64> = confs
        .iter()
        .zip(&runs)
        .map(|(c, r)| {
            if r.is_some() {
                (-(c.energy - confs[0].energy) / KT_KCAL).exp()
            } else {
                0.0
            }
        })
        .collect();
    let sum: f64 = w.iter().sum();
    w.iter_mut().for_each(|v| *v /= sum);
    let n = m.len();
    let (mut q, mut fp, mut fm, mut homo, mut lumo) =
        (vec![0.0; n], vec![0.0; n], vec![0.0; n], 0.0, 0.0);
    for (r, w) in runs.iter().zip(&w) {
        let Some((f, h, l, ..)) = r else { continue };
        for i in 0..n {
            q[i] += w * f.q[i].unwrap_or(0.0);
            fp[i] += w * f.f_plus[i];
            fm[i] += w * f.f_minus[i];
        }
        homo += w * h;
        lumo += w * l;
    }
    Result3 {
        method: "GFN2-xTB",
        fukui: Fukui {
            q: q.into_iter().map(Some).collect(),
            f_zero: fp
                .iter()
                .zip(&fm)
                .map(|(p, m)| Some(0.5 * (p + m)))
                .collect(),
            f_plus: fp,
            f_minus: fm,
        },
        homo_ev: homo,
        lumo_ev: lumo,
        converged: c,
        iterations: it,
        spread: None,
        weights: Some(w),
    }
}
