//! Specificity of the m/z of a row (`detail.specificity`): the signal of the same file and the same time window at ±1 and ±2 Da (the
//! neighbours, where a hump rather than an ion shows up) and at ±0.5 Da (decoy windows with no ion in them), next to the signal of the ion.

use mzlab_core::idxread::{Index, ReadAt};
use mzlab_core::json::{round, Out};

use super::detail::xic;
use super::{Entry, LrDig};
use crate::legacy::Res;

/// Neighbour windows, Da.
const NEIGHBOURS: [f64; 4] = [-2.0, -1.0, 1.0, 2.0];
/// Decoy windows, Da.
const DECOYS: [f64; 2] = [-0.5, 0.5];

/// Summed intensity of the points of the XIC at `mz` inside the time window of the peak.
fn signal<R: ReadAt>(ix: &mut Index<R>, mz: f64, tol: f64, w: (f64, f64), rt_min: f64) -> Res<f64> {
    let (rt, y) = xic(ix, mz - tol, mz + tol, rt_min)?;
    Ok(rt
        .iter()
        .zip(&y)
        .filter(|(t, _)| **t >= w.0 && **t <= w.1)
        .map(|p| p.1)
        .sum())
}

fn windows<R: ReadAt>(
    o: &mut Out,
    ix: &mut Index<R>,
    d: &LrDig,
    e: &Entry,
    w: (f64, f64),
    here: f64,
) -> Res<()> {
    let mut stronger = [0usize; 2];
    for (g, set) in [&NEIGHBOURS[..], &DECOYS[..]].into_iter().enumerate() {
        o.raw(if g == 0 {
            ",\"neighbours\":["
        } else {
            "],\"decoys\":["
        });
        for (i, dm) in set.iter().enumerate() {
            let s = signal(ix, e.mz_x + dm, d.settings.tol_da, w, d.settings.rt_min)?;
            stronger[g] += usize::from(s >= here);
            o.raw(if i > 0 { "," } else { "" })
                .raw("{\"dm\":")
                .num(*dm)
                .raw(",\"signal\":")
                .num(s.round())
                .raw("}");
        }
    }
    o.raw("],\"stronger\":{\"neighbours\":")
        .int(stronger[0] as i64)
        .raw(",\"decoys\":")
        .int(stronger[1] as i64)
        .raw("}");
    Ok(())
}

/// `,"specificity":...` for `e` in its strongest column; `null` when it has no peak. `open(k)` gives the index of column `k`.
pub fn specificity_json<R: ReadAt>(
    o: &mut Out,
    d: &LrDig,
    e: &Entry,
    open: &mut dyn FnMut(usize) -> Res<Index<R>>,
) -> Res<()> {
    let best = (0..e.cells.len())
        .filter(|&k| e.cells[k].detected)
        .max_by(|&a, &b| e.cells[a].area.total_cmp(&e.cells[b].area));
    let Some(k) = best else {
        o.raw("null");
        return Ok(());
    };
    let (c, mut ix) = (&e.cells[k], open(k)?);
    let w = (c.lo, c.hi);
    let here = signal(&mut ix, e.mz_x, d.settings.tol_da, w, d.settings.rt_min)?;
    o.raw("{\"file\":")
        .str(&d.lc[k].label)
        .raw(",\"from\":")
        .num(round(w.0, 3))
        .raw(",\"to\":")
        .num(round(w.1, 3))
        .raw(",\"signal\":")
        .num(here.round());
    windows(o, &mut ix, d, e, w, here)?;
    o.raw("}");
    Ok(())
}
