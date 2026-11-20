//! The line between the edges of a window as the Data view draws it: the 3-point end means, the area and height against that line, and the line the parts of
//! a split window share.

use super::*;

/// The page's end point of the line between the edges: the mean of the point and its two neighbours.
pub(super) fn edge3(y: &[f64], i: usize) -> f64 {
    let (lo, hi) = (i.saturating_sub(1), (i + 1).min(y.len() - 1));
    y[lo..=hi].iter().sum::<f64>() / (hi - lo + 1) as f64
}

/// `[area (counts × s), height, apex RT]` against the line between the edges, as the Data view measures a window with `edges`: trapezoids minus the
/// line, area and height clipped at 0.
pub(super) fn edges_area(t: &[f64], y: &[f64], a: f64, b: f64) -> [f64; 3] {
    let (lo, hi) = (near_idx(t, a.min(b)), near_idx(t, a.max(b)));
    if hi <= lo {
        return [0.0, 0.0, t[lo]];
    }
    let mut ar = 0.0;
    for j in lo + 1..=hi {
        ar += (y[j] + y[j - 1]) / 2.0 * (t[j] - t[j - 1]) * 60.0;
    }
    let at = (lo..=hi).fold(lo, |m, j| if y[j] > y[m] { j } else { m });
    let (ya, yb) = (edge3(y, lo), edge3(y, hi));
    let line = (ya + yb) / 2.0 * (t[hi] - t[lo]) * 60.0;
    let hb = ya + (yb - ya) * (t[at] - t[lo]) / (t[hi] - t[lo]);
    [(ar - line).max(0.0), (y[at] - hb).max(0.0), t[at]]
}

/// The line the parts of a split window share: `edges` as the line between the 3-point means of the edges, the others as they are.
pub(super) fn drop_line(base: Baseline, tr: &Trace, lo: usize, hi: usize) -> Baseline {
    match base {
        Baseline::Edges => Baseline::Line {
            t0: tr.t[lo],
            y0: edge3(&tr.y, lo),
            t1: tr.t[hi],
            y1: edge3(&tr.y, hi),
        },
        b => b,
    }
}
