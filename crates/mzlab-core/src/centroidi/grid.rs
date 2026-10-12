//! The points of a scan on a regular grid. Regular files (the 3200: 0.06 Da): every bin between the first and the last point, absent ones are zero.
//! Irregular files (Orbitrap, TOF): the points themselves, with a zero on each side of a gap wider than 1.5 steps (the gap is not filled).

use super::{Buf, Params};

/// Fills `buf.x` (m/z of each bin) and `buf.v` (raw intensity); a bin that is not in the scan is zero.
pub(super) fn fill(mz: &[f64], y: &[f64], p: &Params, buf: &mut Buf) {
    buf.x.clear();
    buf.v.clear();
    let n = mz.len().min(y.len());
    if n == 0 {
        return;
    }
    if regular(&mz[..n], p.step) {
        dense(&mz[..n], &y[..n], p.step, buf);
    } else {
        sparse(&mz[..n], &y[..n], p.step, buf);
    }
}

/// True when every point sits on the grid of the first one (within 5 % of a step).
fn regular(mz: &[f64], step: f64) -> bool {
    mz.iter().all(|&m| {
        let q = (m - mz[0]) / step;
        (q - q.round()).abs() <= 0.05
    })
}

fn dense(mz: &[f64], y: &[f64], step: f64, buf: &mut Buf) {
    let last = ((mz[mz.len() - 1] - mz[0]) / step).round() as usize;
    for b in 0..=last {
        buf.x.push(mz[0] + b as f64 * step);
        buf.v.push(0.0);
    }
    for (i, &m) in mz.iter().enumerate() {
        let b = ((m - mz[0]) / step).round() as usize;
        buf.x[b] = m;
        buf.v[b] += y[i];
    }
}

fn sparse(mz: &[f64], y: &[f64], step: f64, buf: &mut Buf) {
    for i in 0..mz.len() {
        if i > 0 && mz[i] - mz[i - 1] > 1.5 * step {
            let gap = mz[i] - mz[i - 1];
            buf.x.push(mz[i - 1] + step);
            buf.v.push(0.0);
            if gap > 2.0 * step {
                buf.x.push(mz[i] - step);
                buf.v.push(0.0);
            }
        }
        buf.x.push(mz[i]);
        buf.v.push(y[i]);
    }
}
