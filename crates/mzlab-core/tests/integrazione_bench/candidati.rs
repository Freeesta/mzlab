//! The candidates of the benchmark: the methods mzLab already has (CWT, the click of the page, the prominence), two variants and the
//! prototypes of the chosen method; each returns the windows `(lo, hi)` and their areas for one trace.

use mzlab_core::peakq::{detect_prominence, ProminenceParams};
use mzlab_core::peakq_auto::{auto_integrate, AutoParams};
use mzlab_core::peaks::{detect_peaks, Params};
use mzlab_core::signal::asls_baseline;

/// `integ()` of picchi.js: trapezoid of the raw trace minus the straight line between the 3-point means at the two limits.
pub fn integ_page(rt: &[f64], y: &[f64], i0: usize, i1: usize) -> f64 {
    if i1 <= i0 {
        return 0.0;
    }
    let ar: f64 = (i0 + 1..=i1)
        .map(|j| (y[j] + y[j - 1]) / 2.0 * (rt[j] - rt[j - 1]))
        .sum();
    let e3 = |i: usize| {
        let (a, b) = (i.saturating_sub(1), (i + 1).min(y.len() - 1));
        y[a..=b].iter().sum::<f64>() / (b - a + 1) as f64
    };
    (ar - (e3(i0) + e3(i1)) / 2.0 * (rt[i1] - rt[i0])).max(0.0)
}

/// Area above a baseline trace between two limits (negative parts clipped).
fn integ_over(rt: &[f64], y: &[f64], b: &[f64], i0: usize, i1: usize) -> f64 {
    (i0 + 1..=i1)
        .map(|j| {
            ((y[j] - b[j]).max(0.0) + (y[j - 1] - b[j - 1]).max(0.0)) / 2.0 * (rt[j] - rt[j - 1])
        })
        .sum()
}

/// Port of `sgFilter` of picchi.js (half window m, edges replicated).
fn sg_filter(y: &[f64], m: usize, deriv: bool) -> Vec<f64> {
    let n = y.len();
    let mf = m as f64;
    let mm = mf * (mf + 1.0) * (2.0 * mf + 1.0) / 3.0;
    let w: Vec<f64> = (-(m as i64)..=m as i64)
        .map(|j| {
            let j = j as f64;
            if deriv {
                j / mm
            } else {
                3.0 * (3.0 * mf * mf + 3.0 * mf - 1.0 - 5.0 * j * j)
                    / ((2.0 * mf - 1.0) * (2.0 * mf + 1.0) * (2.0 * mf + 3.0))
            }
        })
        .collect();
    (0..n)
        .map(|i| {
            (0..2 * m + 1)
                .map(|k| w[k] * y[(i as i64 + k as i64 - m as i64).clamp(0, n as i64 - 1) as usize])
                .sum()
        })
        .collect()
}

fn near_idx(xs: &[f64], v: f64) -> usize {
    let (mut lo, mut hi) = (0usize, xs.len() - 1);
    if hi < 1 {
        return 0;
    }
    while hi - lo > 1 {
        let m = (lo + hi) >> 1;
        if xs[m] < v {
            lo = m
        } else {
            hi = m
        }
    }
    if (xs[lo] - v).abs() < (xs[hi] - v).abs() {
        lo
    } else {
        hi
    }
}

/// Port of `autoEdges` of picchi.js with the defaults of `PK_DEF` (noise "mad", no width constraint, not MRM): the edges of the peak clicked at x.
pub fn auto_edges(xs: &[f64], ys: &[f64], x: f64) -> (usize, usize) {
    let min_half = 2usize;
    let n = xs.len();
    let ic = near_idx(xs, x);
    let (q0, q1) = (ic.saturating_sub(50), (ic + 50).min(n - 1));
    let dt = (xs[q1] - xs[q0]) / (q1 - q0).max(1) as f64;
    let a0 = near_idx(xs, x - 2.0);
    let a1 = near_idx(xs, x + 2.0).min(n - 1);
    let y = &ys[a0..=a1];
    let nn = y.len();
    let m_coarse = ((0.03 / dt / 2.0).round() as usize).max(2);
    let co = sg_filter(y, m_coarse, false);
    let lo = near_idx(xs, x - 0.25) - a0;
    let hi = (near_idx(xs, x + 0.25) - a0).min(nn - 1);
    let mut m0 = lo;
    for j in lo..=hi {
        if co[j] > co[m0] {
            m0 = j;
        }
    }
    let cmin = co.iter().cloned().fold(f64::INFINITY, f64::min);
    let half = cmin + (co[m0] - cmin) / 2.0;
    let (mut hl, mut hr) = (m0, m0);
    while hl > 0 && co[hl] > half {
        hl -= 1;
    }
    while hr < nn - 1 && co[hr] > half {
        hr += 1;
    }
    let m = (((hr - hl) as f64 * 0.3).round() as usize).clamp(min_half, 60);
    let sm = sg_filter(y, m, false);
    let d = sg_filter(y, m, true);
    let mut k = m0.saturating_sub(m);
    let k1 = (m0 + m).min(nn - 1);
    for j in k..=k1 {
        if sm[j] > sm[k] {
            k = j;
        }
    }
    let (l, r) = edges_from(y, &sm, &d, k, m, dt);
    (a0 + l, a0 + r)
}

/// Second half of `autoEdges`: noise and local baseline around the apex `k` of the smoothed trace, then the walk out to each edge.
fn edges_from(y: &[f64], sm: &[f64], d: &[f64], k: usize, m: usize, dt: f64) -> (usize, usize) {
    let (win, area_noise, edge_frac, multi, tail) = (1.2, 3.0, 3.0, 50.0, 1.5);
    let nn = y.len();
    let wn = (win / dt).round() as usize;
    let (w0, w1) = (k.saturating_sub(wn), (k + wn).min(nn - 1));
    let mut res: Vec<f64> = (w0..=w1).map(|j| (y[j] - sm[j]).abs()).collect();
    res.sort_by(|a, b| a.partial_cmp(b).unwrap());
    let noise = (1.4826 * res[res.len() >> 1]).max(1e-12);
    let mut low: Vec<f64> = (w0..=w1).map(|j| sm[j]).collect();
    low.sort_by(|a, b| a.partial_cmp(b).unwrap());
    let base = low[(low.len() as f64 * 0.1).floor() as usize];
    let hh = (sm[k] - base).max(0.0);
    let frac = if noise > 0.0 && hh / noise > 800.0 {
        (edge_frac / 100.0_f64).min((5.0 * noise / hh.max(1.0)).max(0.005))
    } else {
        edge_frac / 100.0
    };
    let lim = base + (frac * hh).max(area_noise * noise);
    let need = ((m as f64 * multi / 100.0).round() as usize).max(2);
    let dth = noise / (m.max(1) as f64) * 0.3;
    let maxw = (tail / dt).round() as usize;
    let walk = |dir: i64| -> usize {
        let (mut j, mut bad) = (k as i64, 0usize);
        while j + dir >= 0 && j + dir < nn as i64 && ((j - k as i64).unsigned_abs() as usize) < maxw
        {
            let nx = j + dir;
            j = nx;
            if sm[nx as usize] <= lim {
                break;
            }
            let rising = if dir < 0 {
                d[nx as usize] < -dth
            } else {
                d[nx as usize] > dth
            };
            bad = if rising { bad + 1 } else { 0 };
            if bad >= need {
                j = nx - dir * need as i64;
                break;
            }
        }
        j as usize
    };
    (walk(-1).min(k), walk(1).max(k))
}

/// One candidate: windows `(lo, hi)` and their areas for a trace (the click-based page method clicks at the true apexes).
pub type Method = fn(&[f64], &[f64], &[f64]) -> Vec<(usize, usize, f64)>;

fn prom() -> ProminenceParams {
    ProminenceParams {
        local_snr: 5.0,
        ..Default::default()
    }
}

/// The limits of the prominence detector, by RT (the first HR build of the one-click tool).
fn auto_limits(
    rt: &[f64],
    y: &[f64],
    _x0: f64,
    _x1: f64,
    p: ProminenceParams,
) -> Vec<(usize, usize)> {
    let mut w: Vec<(usize, usize)> = detect_prominence(rt, y, p)
        .iter()
        .map(|k| (k.lo, k.hi))
        .collect();
    w.sort();
    w
}

pub fn m_cwt(rt: &[f64], y: &[f64], _clicks: &[f64]) -> Vec<(usize, usize, f64)> {
    detect_peaks(rt, y, Params::default())
        .iter()
        .map(|p| (p.lo, p.hi, integ_page(rt, y, p.lo, p.hi)))
        .collect()
}

pub fn m_page(rt: &[f64], y: &[f64], clicks: &[f64]) -> Vec<(usize, usize, f64)> {
    // integrations never overlap in the page: a click inside an integrated peak is refused, next to one the edge stops at its border
    let mut out: Vec<(usize, usize, f64)> = Vec::new();
    for &x in clicks {
        let ic = near_idx(rt, x);
        if out.iter().any(|w| w.0 < ic && ic < w.1) {
            continue;
        }
        let (mut l, mut r) = auto_edges(rt, y, x);
        for w in &out {
            if w.1 <= ic {
                l = l.max(w.1)
            }
            if w.0 >= ic {
                r = r.min(w.0)
            }
        }
        if r > l {
            out.push((l, r, integ_page(rt, y, l, r)));
        }
    }
    out
}

pub fn m_prom(rt: &[f64], y: &[f64], _clicks: &[f64]) -> Vec<(usize, usize, f64)> {
    auto_limits(rt, y, rt[0], rt[rt.len() - 1], prom())
        .into_iter()
        .map(|(l, h)| (l, h, integ_page(rt, y, l, h)))
        .collect()
}

/// Prominence limits; touching windows form a cluster with ONE straight baseline from its first to its last limit, split by vertical drops
/// at the valleys (the drop-line rule of HPLC data systems).
pub fn m_prom_drop(rt: &[f64], y: &[f64], _clicks: &[f64]) -> Vec<(usize, usize, f64)> {
    let w = auto_limits(rt, y, rt[0], rt[rt.len() - 1], prom());
    let e3 = |i: usize| {
        let (a, b) = (i.saturating_sub(1), (i + 1).min(y.len() - 1));
        y[a..=b].iter().sum::<f64>() / (b - a + 1) as f64
    };
    let mut out = Vec::new();
    let mut i = 0;
    while i < w.len() {
        let mut j = i;
        while j + 1 < w.len() && w[j].1 == w[j + 1].0 {
            j += 1;
        }
        let (c0, c1) = (w[i].0, w[j].1);
        let (y0, y1) = (e3(c0), e3(c1));
        let line: Vec<f64> = (0..y.len())
            .map(|k| y0 + (y1 - y0) * (rt[k] - rt[c0]) / (rt[c1] - rt[c0]).max(1e-12))
            .collect();
        for k in i..=j {
            out.push((w[k].0, w[k].1, integ_over(rt, y, &line, w[k].0, w[k].1)));
        }
        i = j + 1;
    }
    out
}

/// Prominence limits, area above an asymmetric-least-squares baseline of the whole trace (the baseline of the CWT detector).
pub fn m_prom_asls(rt: &[f64], y: &[f64], _clicks: &[f64]) -> Vec<(usize, usize, f64)> {
    let b = asls_baseline(y, 1e5, 0.01, 10, 160);
    auto_limits(rt, y, rt[0], rt[rt.len() - 1], prom())
        .into_iter()
        .map(|(l, h)| (l, h, integ_over(rt, y, &b, l, h)))
        .collect()
}

/// Candidates F0 and F1, prototypes of `peakq_auto::auto_integrate` (F2) without its flat-flank stop: AsLS background moved up to the median of
/// the background points; Savitzky-Golay smoothing of half window 30 % of the FWHM of the tallest peak; apexes from the prominence picking;
/// each edge walks out from the apex until the smoothed corrected signal falls to `k` times its own noise or reaches the valley shared with a
/// neighbour. Area = trapezoid of the raw trace above the straight line between the background at the two edges.
fn integrate_f(rt: &[f64], y: &[f64], k: f64) -> Vec<(usize, usize, f64)> {
    let n = y.len();
    let noise = mzlab_core::peaks::noise_level(y).max(1e-9);
    let mut b = asls_baseline(y, 1e7, 0.01, 10, 160);
    let mut bg: Vec<f64> = (0..n)
        .map(|i| y[i] - b[i])
        .filter(|r| *r < 3.0 * noise)
        .collect();
    bg.sort_by(|a, c| a.partial_cmp(c).unwrap());
    let off = if bg.is_empty() { 0.0 } else { bg[bg.len() / 2] };
    b.iter_mut().for_each(|v| *v += off);
    let yc: Vec<f64> = (0..n).map(|i| y[i] - b[i]).collect();
    let coarse = mzlab_core::peaks::savgol(&yc, 5, 2);
    let top = (0..n)
        .max_by(|&i, &j| coarse[i].partial_cmp(&coarse[j]).unwrap())
        .unwrap_or(0);
    let (mut hl, mut hr) = (top, top);
    while hl > 0 && coarse[hl] > coarse[top] / 2.0 {
        hl -= 1;
    }
    while hr < n - 1 && coarse[hr] > coarse[top] / 2.0 {
        hr += 1;
    }
    let m = (((hr - hl) as f64 * 0.3).round() as usize).clamp(2, 30);
    let sc = mzlab_core::peaks::savgol(&yc, 2 * m + 1, 2);
    let mf = m as f64;
    let w0 = 3.0 * (3.0 * mf * mf + 3.0 * mf - 1.0)
        / ((2.0 * mf - 1.0) * (2.0 * mf + 1.0) * (2.0 * mf + 3.0));
    let thr = k * noise * w0.sqrt();
    let mut apex: Vec<usize> = detect_prominence(rt, y, prom())
        .iter()
        .map(|p| p.apex_i)
        .collect();
    apex.sort();
    let valley = |a: usize, c: usize| {
        (a + 1..c)
            .min_by(|&i, &j| sc[i].partial_cmp(&sc[j]).unwrap())
            .unwrap_or(a)
    };
    let mut out = Vec::new();
    for (q, &a) in apex.iter().enumerate() {
        let left = if q > 0 { valley(apex[q - 1], a) } else { 0 };
        let right = if q + 1 < apex.len() {
            valley(a, apex[q + 1])
        } else {
            n - 1
        };
        let (mut lo, mut hi) = (a, a);
        while lo > left && sc[lo] > thr {
            lo -= 1;
        }
        while hi < right && sc[hi] > thr {
            hi += 1;
        }
        if hi - lo >= 2 {
            let ar: f64 = (lo + 1..=hi)
                .map(|j| (y[j] + y[j - 1]) / 2.0 * (rt[j] - rt[j - 1]))
                .sum();
            out.push((lo, hi, ar - (b[lo] + b[hi]) / 2.0 * (rt[hi] - rt[lo])));
        }
    }
    out
}

pub fn m_f0(rt: &[f64], y: &[f64], _c: &[f64]) -> Vec<(usize, usize, f64)> {
    integrate_f(rt, y, 0.0)
}

pub fn m_f1(rt: &[f64], y: &[f64], _c: &[f64]) -> Vec<(usize, usize, f64)> {
    integrate_f(rt, y, 1.0)
}

/// F2 is the product: `peakq_auto::auto_integrate`.
pub fn m_f2(rt: &[f64], y: &[f64], _c: &[f64]) -> Vec<(usize, usize, f64)> {
    auto_integrate(rt, y, rt[0], rt[rt.len() - 1], AutoParams::default())
        .iter()
        .map(|k| (k.lo, k.hi, k.area))
        .collect()
}
