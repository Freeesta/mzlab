//! The grid of `compose`: the ranges of the elements other than C and H, narrowed to the parent formula and to the ion mass, then counted
//! against the limit, which gives the number of rows. The mass cut drops only rows that could never be hits, so it changes no answer (`tests/composition_cut.rs`).

use super::{sym_mass, Ranges, MAX_GRID};
use crate::elements::Formula;
use crate::error::{Error, Result};

/// Narrows `els` to the parent and, with `limit` (ion mass + tolerance), to the mass, and gives the number of rows; a grid still over `MAX_GRID` is an error with its size and the limit.
pub(super) fn narrow(
    els: &mut Ranges,
    parent: Option<&Formula>,
    limit: Option<f64>,
) -> Result<u64> {
    if let Some(p) = parent {
        for (sym, r) in els.iter_mut() {
            let hi = p
                .iter()
                .find(|(e, _)| e == sym)
                .map_or(0, |(_, n)| *n as i64);
            *r = (r.0.min(hi), r.1.min(hi));
        }
        els.retain(|(_, r)| r.1 >= r.0);
    }
    let others = els.iter_mut().filter(|(s, _)| s != "C" && s != "H");
    let mut size: u64 = 1;
    for (sym, r) in others {
        // a row is a hit only if the ion mass minus its mass is at least -tolerance, and C and H only add mass: one atom more is kept for rounding
        let (m, top) = (
            sym_mass(sym),
            limit.map_or(f64::INFINITY, |l| (l / sym_mass(sym)).floor() + 1.0),
        );
        if m > 0.0 && top < r.1 as f64 {
            r.1 = (top as i64).max(r.0);
        }
        size = size.saturating_mul((r.1 - r.0 + 1) as u64);
    }
    if size > MAX_GRID {
        return Err(Error::new("err.comp.many")
            .with("n", size)
            .with("max", MAX_GRID));
    }
    Ok(size)
}
