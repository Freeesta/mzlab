//! Where one end of an integration window goes when the user drags it along the trace (the points where the area meets its baseline):
//! always onto a point of the trace, never past the other end (the window keeps at least two points) nor past a neighbouring integration.
//! The page (`window_end` of `mzlab-wasm`, `QUALITA.windowEnd`) and `mzlab-cli peak-window` print the same JSON.

use crate::peakq_lr::near_idx;

/// A neighbour's border counts as touching within this many minutes (as `INT_EPS` of the page).
const EPS: f64 = 1e-6;

/// The point of `rt` (ascending) one end of the window goes to. `cur` is that end, `other` the other end, `at` where the pointer is
/// (`None`: move `step` points from `cur`, the keyboard), `lo`/`hi` the borders of the neighbouring integrations it may touch but not
/// cross. The end before `other` stays at least one point before it, the end after `other` one point after. `None` for a trace with
/// fewer than two points or when no point is free.
pub fn move_end(
    rt: &[f64],
    cur: f64,
    other: f64,
    at: Option<f64>,
    step: i64,
    lo: Option<f64>,
    hi: Option<f64>,
) -> Option<usize> {
    let n = rt.len();
    if n < 2 || !cur.is_finite() || !other.is_finite() {
        return None;
    }
    let io = near_idx(rt, other);
    let want = match at {
        Some(t) if t.is_finite() => near_idx(rt, t) as i64,
        Some(_) => return None,
        None => near_idx(rt, cur) as i64 + step,
    };
    let (min, max) = if cur <= other {
        let first = lo.map_or(0, |l| rt.partition_point(|&x| x < l - EPS));
        (first as i64, io as i64 - 1)
    } else {
        let last = hi.map_or(n, |h| rt.partition_point(|&x| x <= h + EPS));
        (io as i64 + 1, last as i64 - 1)
    };
    (min <= max).then(|| want.clamp(min, max) as usize)
}

/// `move_end` as the JSON of the page and `mzlab-cli peak-window`: `{"t":MIN,"i":INDEX}`.
pub fn move_end_json(
    rt: &[f64],
    cur: f64,
    other: f64,
    at: Option<f64>,
    step: i64,
    lo: Option<f64>,
    hi: Option<f64>,
) -> Option<String> {
    let i = move_end(rt, cur, other, at, step, lo, hi)?;
    Some(format!("{{\"t\":{},\"i\":{i}}}", rt[i]))
}

#[cfg(test)]
#[path = "intwin_test.rs"]
mod tests;
