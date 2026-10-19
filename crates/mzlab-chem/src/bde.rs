//! Energy of the C–H radical of each site: GFN2-xTB electronic energy of the molecule with one hydrogen removed (odd electron
//! count, same geometry, same solvent) minus that of the parent. A lower value means an easier H· loss; it is a second signal
//! independent of the Fukui values, used by `reatt.rs` for the combined score.

use crate::fukui::Site;
use crate::mol3d::Mol3d;
use crate::xtb::{alpb, System};
use mzlab_core::sortc::SortC;

/// Sites within this score of the first one (scores run from 0 to 1) are a tie.
const TIE_SCORE: f64 = 0.02;
/// Hartree to kcal/mol.
const KCAL: f64 = 627.509_47;

fn energy(el: &[u8], xyz: &[[f64; 3]], charge: i32, water: bool) -> Option<f64> {
    let s = System::new(el, xyz, water.then_some(&alpb::WATER)).ok()?;
    let r = s.solve(charge, None).ok()?;
    r.converged.then_some(r.energy)
}

/// Radical formation energy (kcal/mol, relative to the parent) of the first hydrogen of each site; `None` when an SCC fails.
pub fn radical_energies(m: &Mol3d, sites: &[Site], water: bool) -> Option<Vec<f64>> {
    let charge: i32 = m.charge.iter().map(|&c| c as i32).sum();
    let e0 = energy(&m.el, &m.xyz, charge, water)?;
    sites
        .iter()
        .map(|s| {
            let c = m
                .heavy_index
                .iter()
                .position(|h| *h == Some(s.heavy_index))?;
            let h = m.bonds.iter().find_map(|b| match (b.0 == c, b.1 == c) {
                (true, _) if m.el[b.1] == 1 => Some(b.1),
                (_, true) if m.el[b.0] == 1 => Some(b.0),
                _ => None,
            })?;
            let keep = |i: &usize| *i != h;
            let el: Vec<u8> = (0..m.el.len()).filter(keep).map(|i| m.el[i]).collect();
            let xyz: Vec<[f64; 3]> = (0..m.el.len()).filter(keep).map(|i| m.xyz[i]).collect();
            Some((energy(&el, &xyz, charge, water)? - e0) * KCAL)
        })
        .collect()
}

/// Min-max scale of `v` to 0..1 (0 when all equal).
fn scale(v: f64, lo: f64, hi: f64) -> f64 {
    if hi > lo {
        (v - lo) / (hi - lo)
    } else {
        0.0
    }
}

/// Combined score of every site, half from f(−) and half from the radical energy (both scaled to 0..1 over the sites, higher
/// is more reactive): sets `bde` and `parts` and ranks the sites again by the score.
pub fn combine(sites: &mut [Site], e: &[f64]) {
    let range = |it: &mut dyn Iterator<Item = f64>| {
        it.fold((f64::MAX, f64::MIN), |a, v| (a.0.min(v), a.1.max(v)))
    };
    let (fl, fh) = range(&mut sites.iter().map(|s| s.f_minus));
    let (el, eh) = range(&mut e.iter().copied());
    for (s, &v) in sites.iter_mut().zip(e) {
        let (pf, pb) = (scale(s.f_minus, fl, fh), scale(eh - v, 0.0, eh - el));
        s.bde = Some(v);
        s.parts = Some([0.5 * (pf + pb), pf, pb]);
    }
    sites.sortc_by(|a, b| b.parts.unwrap()[0].total_cmp(&a.parts.unwrap()[0]));
    let top = sites.first().map_or(0.0, |s| s.parts.unwrap()[0]);
    for (k, s) in sites.iter_mut().enumerate() {
        s.rank = k + 1;
        s.tie_first = top - s.parts.unwrap()[0] < TIE_SCORE;
    }
}
