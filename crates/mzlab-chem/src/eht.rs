//! Fallback when GFN2-xTB does not converge: extended Hückel theory (Hoffmann 1963, doi:10.1063/1.1734456) of sci-form 0.15.2
//! (`sci_form::eht`, `sci_form::reactivity::compute_fukui_descriptors`; MIT, see LICENSE-sci-form): f(−) = Mulliken population of
//! the HOMO per atom, f(+) = population of the LUMO; no charges and no f(0).

use crate::fukui::Fukui;
use mzlab_core::error::{Error, Result};

/// Fixed note of the fallback.
pub const NOTE: &str = "EHT fallback (GFN2-xTB did not converge): f(−) is the HOMO population per atom and f(+) the LUMO population; no charges, no f(0); validated on 8 of 10 sites (nitrobenzene and paracetamol wrong).";

/// Fukui values and (HOMO, LUMO) in eV.
pub fn fukui(el: &[u8], xyz: &[[f64; 3]]) -> Result<(Fukui, f64, f64)> {
    let r = sci_form::eht::solve_eht(el, xyz, None)
        .map_err(|e| Error::new("error.chem.scc").with("reason", &e))?;
    let d = sci_form::reactivity::compute_fukui_descriptors(el, xyz, &r);
    let n = el.len();
    Ok((
        Fukui {
            q: vec![None; n],
            f_plus: d.f_plus,
            f_zero: vec![None; n],
            f_minus: d.f_minus,
        },
        r.homo_energy,
        r.lumo_energy,
    ))
}
