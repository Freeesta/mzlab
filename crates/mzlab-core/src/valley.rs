//! The valleys of an integration window: where a drop line splits two peaks that overlap. A valley splits only when the smaller of its two maxima
//! rises above it by at least `depth` × the highest point of the window (the trace minus the baseline, a 3-point mean to find the extrema), so the
//! noise of a tail never splits it and a shoulder (no minimum between two maxima) is never split. The parts share the baseline and the split
//! points, so their areas add up to the area of the whole window (`manint`, `mzlab-cli peak-split`).

use crate::baseline::integrate;

/// The default depth: the smaller peak rises above the valley by a tenth of the highest point.
pub const DEPTH: f64 = 0.1;

/// The depth asked, inside 0..=1 (`DEPTH` when it is not a number).
pub fn depth_of(v: f64) -> f64 {
    if v.is_finite() {
        v.clamp(0.0, 1.0)
    } else {
        DEPTH
    }
}

/// The first lowest point of `s` in `p..=q`.
fn lowest(s: &[f64], p: usize, q: usize) -> usize {
    (p..=q).fold(p, |m, j| if s[j] < s[m] { j } else { m })
}

/// The indices strictly inside `lo..hi` where the window `lo..=hi` of `d` (the trace minus its baseline) splits, in order.
pub fn valleys(d: &[f64], lo: usize, hi: usize, depth: f64) -> Vec<usize> {
    let hi = hi.min(d.len().saturating_sub(1));
    if hi < lo + 2 {
        return Vec::new();
    }
    let s: Vec<f64> = (lo..=hi)
        .map(|j| {
            let (a, b) = (j.saturating_sub(1).max(lo), (j + 1).min(hi));
            d[a..=b].iter().sum::<f64>() / (b - a + 1) as f64
        })
        .collect();
    let n = s.len();
    // a maximum is above the point before and not below the point after: a plateau counts once
    let mut peaks: Vec<usize> = (1..n - 1)
        .filter(|&j| s[j] > s[j - 1] && s[j] >= s[j + 1])
        .collect();
    let top = peaks
        .iter()
        .map(|&p| s[p])
        .fold(f64::NEG_INFINITY, f64::max);
    if top.is_nan() || top <= 0.0 {
        return Vec::new();
    }
    let floor = depth_of(depth) * top;
    loop {
        let rise = |i: usize, p: &[usize]| s[p[i]].min(s[p[i + 1]]) - s[lowest(&s, p[i], p[i + 1])];
        let worst = (0..peaks.len().saturating_sub(1))
            .min_by(|&i, &j| rise(i, &peaks).total_cmp(&rise(j, &peaks)));
        match worst {
            Some(i) if rise(i, &peaks) <= floor => {
                let lower = if s[peaks[i]] < s[peaks[i + 1]] {
                    i
                } else {
                    i + 1
                };
                peaks.remove(lower);
            }
            _ => break,
        }
    }
    peaks
        .windows(2)
        .map(|w| lo + lowest(&s, w[0], w[1]))
        .collect()
}

/// One part of a split window: its points, area (units of y × those of the times), height and apex above the line, and Hp/Hv (`None` without a
/// valley above the line next to it).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Part {
    pub lo: usize,
    pub hi: usize,
    pub area: f64,
    pub height: f64,
    pub apex: usize,
    pub pv: Option<f64>,
}

/// The parts of the window `lo..=hi` cut at `cuts` (points inside it; the others ignored), all against the same `line`. Hp/Hv of a part: its height
/// over the higher of the valleys next to it (Ph. Eur. 2.2.46 gives the value of the smaller peak).
pub fn parts(
    t: &[f64],
    y: &[f64],
    lo: usize,
    hi: usize,
    cuts: &[usize],
    line: impl Fn(f64) -> f64,
) -> Vec<Part> {
    let mut b = vec![lo];
    b.extend(cuts.iter().copied().filter(|&k| k > lo && k < hi));
    b.push(hi);
    b.sort_unstable();
    b.dedup();
    let hv = |k: usize| y[k] - line(t[k]);
    b.windows(2)
        .enumerate()
        .map(|(i, w)| {
            let r = integrate(t, y, w[0], w[1], &line);
            let v = [
                (i > 0).then(|| hv(w[0])),
                (i + 2 < b.len()).then(|| hv(w[1])),
            ]
            .into_iter()
            .flatten()
            .fold(f64::NEG_INFINITY, f64::max);
            Part {
                lo: w[0],
                hi: w[1],
                area: r.area,
                height: r.height,
                apex: r.apex,
                pv: (v > 0.0 && r.height > 0.0).then(|| r.height / v),
            }
        })
        .collect()
}

#[cfg(test)]
#[path = "valley_tests.rs"]
mod tests;
