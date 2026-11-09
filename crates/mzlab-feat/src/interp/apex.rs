//! The MS1 around the apex of the rows: intensities of M, M+1 and M+2 and the m/z of M, for all the rows of a file in one sweep of its MS1 section.
//! `build::read_file` uses it.

use mzlab_core::idxread::{Index, ReadAt};

use super::build::Res;
use crate::axis::half;
use crate::sweep::windows;

/// Intensities of M, M+1 and M+2 and the m/z of M.
pub(super) type Apex = ([f64; 3], f64);

/// Intensities of M, M+1 and M+2 and the m/z of M in the MS1 scan nearest to the apex, for every `(m/z, apex)` of `reqs` (singly charged, as everything
/// in HR here): one sweep of the MS1 section for all of them, each gets what a sweep of its own would give.
pub(super) fn apex_ms1_many<R: ReadAt>(
    ix: &mut Index<R>,
    pol: i8,
    reqs: &[(f64, f64)],
) -> Res<Vec<Option<Apex>>> {
    let none = || vec![None; reqs.len()];
    let Some(si) = ix.section_of(1, Some(pol)) else {
        return Ok(none());
    };
    let rt = ix.arrays(si).map_err(|e| e.to_string())?.rt.clone();
    if rt.is_empty() {
        return Ok(none());
    }
    let near = |apex: f64| {
        let p = rt.partition_point(|&t| t < apex);
        if p > 0 && (p == rt.len() || apex - rt[p - 1] < rt[p] - apex) {
            p - 1
        } else {
            p.min(rt.len() - 1)
        }
    };
    let pos: Vec<u32> = reqs.iter().map(|r| near(r.1) as u32).collect();
    let tol: Vec<f64> = reqs.iter().map(|r| half(r.0, 10.0)).collect();
    let mut order: Vec<usize> = (0..reqs.len()).collect();
    mzlab_core::sortc::by(&mut order, |&a, &b| {
        (reqs[a].0 - tol[a]).total_cmp(&(reqs[b].0 - tol[b]))
    });
    let ranges: Vec<(f64, f64)> = order
        .iter()
        .map(|&r| (reqs[r].0 - tol[r], reqs[r].0 + 2.2))
        .collect();
    let mut out = vec![([0.0; 3], f64::NAN); reqs.len()];
    let c13 = mzlab_core::elements::C13_STEP;
    windows(ix, si, &ranges, 1 << 20, |j, a, b, slab| {
        let r = order[j];
        let o = &mut out[r];
        for i in (a..b).filter(|&i| slab.pos[i] == pos[r]) {
            let d = slab.mz[i] - reqs[r].0;
            if d.abs() <= tol[r] && slab.it[i] > o.0[0] {
                o.0[0] = slab.it[i];
                o.1 = slab.mz[i];
            } else if (d - c13).abs() <= 0.01 {
                o.0[1] = o.0[1].max(slab.it[i]);
            } else if (d - 2.0).abs() <= 0.015 {
                o.0[2] += slab.it[i];
            }
        }
        Ok(())
    })
    .map_err(|e| e.to_string())?;
    Ok(out.into_iter().map(Some).collect())
}
