//! Benchmark of the peak-integration candidates on synthetic XIC traces with known areas (docs/studi/INTEGRAZIONE_PICCHI.md).
//! Each case is drawn with 30 noise seeds; the table (area error, boundary error, missed and false peaks, time) is printed with
//! `cargo test --release -p mzlab-core --test integrazione_bench -- --nocapture`; the asserts keep the chosen method's numbers.

use mzlab_core::peakq::{detect_prominence, ProminenceParams};
use mzlab_core::peakq_auto::{auto_integrate, AutoParams};
use mzlab_core::peaks::{detect_peaks, Params};
use mzlab_core::signal::asls_baseline;
use std::time::Instant;

const DT: f64 = 0.01;
const SLOPE: f64 = 0.03;
const N: usize = 600;
const SEEDS: u64 = 30;

/// One true peak: centre (min), sigma (min), height of the Gaussian, exponential tail tau (min, 0 = none).
#[derive(Clone, Copy)]
struct True {
    c: f64,
    s: f64,
    h: f64,
    tau: f64,
}

struct Case {
    name: &'static str,
    peaks: Vec<True>,
    noise: f64,
    /// baseline: offset + slope * t + curve * t^2, or (-1, centre, height) for a broad Gaussian hump of sigma 0.6 min on 1e3
    base: (f64, f64, f64),
}

fn g(c: f64, s: f64, h: f64) -> True {
    True { c, s, h, tau: 0.0 }
}

fn cases() -> Vec<Case> {
    let s = 0.05;
    let flat = (1.0e3, 0.0, 0.0);
    vec![
        Case {
            name: "single, S/N 100",
            peaks: vec![g(3.0, s, 1.0e5)],
            noise: 1.0e3,
            base: flat,
        },
        Case {
            name: "noisy, S/N 10",
            peaks: vec![g(3.0, s, 1.0e4)],
            noise: 1.0e3,
            base: flat,
        },
        Case {
            name: "low S/N 5",
            peaks: vec![g(3.0, s, 5.0e3)],
            noise: 1.0e3,
            base: flat,
        },
        Case {
            name: "tailing tau/sigma 1.6",
            peaks: vec![True {
                c: 3.0,
                s,
                h: 1.0e5,
                tau: 0.08,
            }],
            noise: 1.0e3,
            base: flat,
        },
        Case {
            name: "pair Rs 1.5, 1:0.6",
            peaks: vec![g(2.85, s, 1.0e5), g(3.15, s, 6.0e4)],
            noise: 1.0e3,
            base: flat,
        },
        Case {
            name: "pair Rs 0.8, 1:0.6",
            peaks: vec![g(2.92, s, 1.0e5), g(3.08, s, 6.0e4)],
            noise: 1.0e3,
            base: flat,
        },
        Case {
            name: "shoulder Rs 0.5, 1:0.4",
            peaks: vec![g(2.95, s, 1.0e5), g(3.05, s, 4.0e4)],
            noise: 1.0e3,
            base: flat,
        },
        Case {
            name: "drifting baseline",
            peaks: vec![g(3.0, s, 1.0e5)],
            noise: 1.0e3,
            base: (2.0e3, 1.0e4, 1.5e3),
        },
        Case {
            name: "broad hump under the peak",
            peaks: vec![g(3.0, s, 1.0e5)],
            noise: 1.0e3,
            base: (-1.0, 3.2, 2.0e4),
        },
        Case {
            name: "five peaks 1e5..5e3",
            peaks: vec![
                g(0.8, s, 1.0e5),
                g(1.8, s, 3.0e4),
                g(2.8, s, 1.0e4),
                g(3.8, 0.06, 2.0e4),
                g(4.8, s, 5.0e3),
            ],
            noise: 1.0e3,
            base: flat,
        },
        Case {
            name: "no peak",
            peaks: vec![],
            noise: 1.0e3,
            base: flat,
        },
    ]
}

/// Deterministic standard normal noise (LCG + Box-Muller).
struct Rng(u64);
impl Rng {
    fn u(&mut self) -> f64 {
        self.0 = self
            .0
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        ((self.0 >> 11) as f64 + 0.5) / (1u64 << 53) as f64
    }
    fn n(&mut self) -> f64 {
        (-2.0 * self.u().ln()).sqrt() * (2.0 * std::f64::consts::PI * self.u()).cos()
    }
}

/// The clean shape of one peak on the grid: a Gaussian, convolved with a unit-area exponential when tau > 0 (the area is kept).
fn shape(rt: &[f64], p: &True) -> Vec<f64> {
    let gauss: Vec<f64> = rt
        .iter()
        .map(|t| p.h * (-((t - p.c) / p.s).powi(2) / 2.0).exp())
        .collect();
    if p.tau <= 0.0 {
        return gauss;
    }
    let k: Vec<f64> = (0..rt.len())
        .map(|i| (-(i as f64) * DT / p.tau).exp() * DT / p.tau)
        .collect();
    let ks: f64 = k.iter().sum();
    (0..rt.len())
        .map(|i| (0..=i).map(|j| gauss[j] * k[i - j]).sum::<f64>() / ks)
        .collect()
}

/// The trace, the clean shape of each peak and the true area of each (counts x min).
fn draw(c: &Case, seed: u64) -> (Vec<f64>, Vec<f64>, Vec<Vec<f64>>) {
    let rt: Vec<f64> = (0..N).map(|i| i as f64 * DT).collect();
    let shapes: Vec<Vec<f64>> = c.peaks.iter().map(|p| shape(&rt, p)).collect();
    let mut r = Rng(seed * 7919 + 13);
    let y = (0..N)
        .map(|i| {
            let t = rt[i];
            let sig: f64 = shapes.iter().map(|s| s[i]).sum();
            // additive detector noise plus 3 % ion-statistics noise on the signal
            let bl = if c.base.0 < 0.0 {
                1.0e3 + c.base.2 * (-((t - c.base.1) / 0.6).powi(2) / 2.0).exp()
            } else {
                c.base.0 + c.base.1 * t / 6.0 + c.base.2 * (t / 6.0).powi(2)
            };
            bl + sig * (1.0 + 0.03 * r.n()) + c.noise * r.n()
        })
        .collect();
    (rt, y, shapes)
}

fn true_area(p: &True) -> f64 {
    p.h * p.s * (2.0 * std::f64::consts::PI).sqrt()
}

/// Limits of the clean shape: where it stays above 1 % of its maximum.
fn true_limits(s: &[f64]) -> (usize, usize, usize) {
    let (mut apex, mut mx) = (0, f64::MIN);
    for (i, &v) in s.iter().enumerate() {
        if v > mx {
            mx = v;
            apex = i;
        }
    }
    let lo = s.iter().position(|&v| v >= 0.01 * mx).unwrap_or(0);
    let hi = s
        .iter()
        .rposition(|&v| v >= 0.01 * mx)
        .unwrap_or(s.len() - 1);
    (lo, apex, hi)
}

/// `integ()` of picchi.js: trapezoid of the raw trace minus the straight line between the 3-point means at the two limits.
fn integ_page(rt: &[f64], y: &[f64], i0: usize, i1: usize) -> f64 {
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
fn auto_edges(xs: &[f64], ys: &[f64], x: f64) -> (usize, usize) {
    let (win, area_noise, edge_frac, min_half, multi, tail) = (1.2, 3.0, 3.0, 2usize, 50.0, 1.5);
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
    (a0 + walk(-1).min(k), a0 + walk(1).max(k))
}

/// One candidate: windows `(lo, hi)` and their areas for a trace (the click-based page method clicks at the true apexes).
type Method = fn(&[f64], &[f64], &[f64]) -> Vec<(usize, usize, f64)>;

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

fn m_cwt(rt: &[f64], y: &[f64], _clicks: &[f64]) -> Vec<(usize, usize, f64)> {
    detect_peaks(rt, y, Params::default())
        .iter()
        .map(|p| (p.lo, p.hi, integ_page(rt, y, p.lo, p.hi)))
        .collect()
}

fn m_page(rt: &[f64], y: &[f64], clicks: &[f64]) -> Vec<(usize, usize, f64)> {
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

fn m_prom(rt: &[f64], y: &[f64], _clicks: &[f64]) -> Vec<(usize, usize, f64)> {
    auto_limits(rt, y, rt[0], rt[rt.len() - 1], prom())
        .into_iter()
        .map(|(l, h)| (l, h, integ_page(rt, y, l, h)))
        .collect()
}

/// Prominence limits; touching windows form a cluster with ONE straight baseline from its first to its last limit, split by vertical drops
/// at the valleys (the drop-line rule of HPLC data systems).
fn m_prom_drop(rt: &[f64], y: &[f64], _clicks: &[f64]) -> Vec<(usize, usize, f64)> {
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
fn m_prom_asls(rt: &[f64], y: &[f64], _clicks: &[f64]) -> Vec<(usize, usize, f64)> {
    let b = asls_baseline(y, 1e5, 0.01, 10, 160);
    auto_limits(rt, y, rt[0], rt[rt.len() - 1], prom())
        .into_iter()
        .map(|(l, h)| (l, h, integ_over(rt, y, &b, l, h)))
        .collect()
}

/// Candidate F: AsLS background moved up to the median of the background points; Savitzky-Golay smoothing with a window of about 60 % of the
/// FWHM of the tallest peak; apexes from the prominence picking; each edge walks out from the apex until the smoothed corrected signal falls to
/// `k` times its own noise (sigma of the raw noise times the root of the central SG weight) or reaches the valley shared with a neighbour.
/// Area = trapezoid of the raw trace above the straight line between the baseline values at the two edges (what the page would draw).
fn integrate_f(rt: &[f64], y: &[f64], k: f64, local: bool) -> Vec<(usize, usize, f64)> {
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
    let half = coarse[top] / 2.0;
    let (mut hl, mut hr) = (top, top);
    while hl > 0 && coarse[hl] > half {
        hl -= 1;
    }
    while hr < n - 1 && coarse[hr] > half {
        hr += 1;
    }
    let m = (((hr - hl) as f64 * 0.3).round() as usize).clamp(2, 30);
    let w = 2 * m + 1;
    let sc = mzlab_core::peaks::savgol(&yc, w, 2);
    let mf = m as f64;
    let w0 = 3.0 * (3.0 * mf * mf + 3.0 * mf - 1.0)
        / ((2.0 * mf - 1.0) * (2.0 * mf + 1.0) * (2.0 * mf + 3.0));
    let thr = k * noise * w0.sqrt();
    let mut apex: Vec<usize> = mzlab_core::peakq::detect_prominence(rt, y, prom())
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
        if !local {
            let mut lo = a;
            while lo > left && sc[lo] > thr {
                lo -= 1;
            }
            let mut hi = a;
            while hi < right && sc[hi] > thr {
                hi += 1;
            }
            if hi - lo < 2 {
                continue;
            }
            let ar: f64 = (lo + 1..=hi)
                .map(|j| (y[j] + y[j - 1]) / 2.0 * (rt[j] - rt[j - 1]))
                .sum();
            out.push((lo, hi, ar - (b[lo] + b[hi]) / 2.0 * (rt[hi] - rt[lo])));
            continue;
        }
        // the edge also stops where the smoothed trace has risen again for `need` points walking away (it sits on a hump or another
        // peak), and at 5 FWHM of this peak; there the baseline end is the smoothed level, on background or at a shared valley it is b
        let hh = sc[a] / 2.0;
        let (mut l2, mut r2) = (a, a);
        while l2 > 0 && sc[l2] > hh {
            l2 -= 1;
        }
        while r2 < n - 1 && sc[r2] > hh {
            r2 += 1;
        }
        let cap = 5 * (r2 - l2).max(2);
        // steepest slope of this peak's flanks (central differences of the smoothed trace, within one FWHM of the apex)
        let fw = (r2 - l2).max(2);
        let dd = |i: usize| (sc[(i + 1).min(n - 1)] - sc[i.saturating_sub(1)]) / 2.0;
        let dmax = (a.saturating_sub(fw)..=(a + fw).min(n - 1))
            .map(|i| dd(i).abs())
            .fold(0.0, f64::max);
        let walk = |dir: i64, stop: usize| -> (usize, bool) {
            let (mut j, mut flat) = (a as i64, 0usize);
            loop {
                let ju = j as usize;
                if ju == stop || sc[ju] <= thr {
                    return (ju, true);
                }
                if (j - a as i64).unsigned_abs() as usize >= cap {
                    return (ju, false);
                }
                // past half height, a slope below SLOPE of the steepest one for m points is the end of the peak on a background that is not flat
                flat = if sc[ju] < hh && dd(ju).abs() < SLOPE * dmax {
                    flat + 1
                } else {
                    0
                };
                if flat > m {
                    return ((j - dir * m as i64) as usize, false);
                }
                j += dir;
            }
        };
        let (lo, lo_bg) = walk(-1, left);
        let (hi, hi_bg) = walk(1, right);
        if hi - lo < 2 {
            continue;
        }
        let bl = if lo_bg {
            b[lo]
        } else {
            y[lo] - yc[lo] + sc[lo]
        };
        let bh = if hi_bg {
            b[hi]
        } else {
            y[hi] - yc[hi] + sc[hi]
        };
        let ar: f64 = (lo + 1..=hi)
            .map(|j| (y[j] + y[j - 1]) / 2.0 * (rt[j] - rt[j - 1]))
            .sum();
        out.push((lo, hi, ar - (bl + bh) / 2.0 * (rt[hi] - rt[lo])));
    }
    out
}

fn m_f0(rt: &[f64], y: &[f64], _c: &[f64]) -> Vec<(usize, usize, f64)> {
    integrate_f(rt, y, 0.0, false)
}

fn m_f1(rt: &[f64], y: &[f64], _c: &[f64]) -> Vec<(usize, usize, f64)> {
    integrate_f(rt, y, 1.0, false)
}

/// F2 is the product: `peakq_auto::auto_integrate` (the bench copy above is its prototype, kept for F0 and F1).
fn m_f2(rt: &[f64], y: &[f64], _c: &[f64]) -> Vec<(usize, usize, f64)> {
    auto_integrate(rt, y, rt[0], rt[rt.len() - 1], AutoParams::default())
        .iter()
        .map(|k| (k.lo, k.hi, k.area))
        .collect()
}

#[derive(Default, Clone)]
struct Score {
    area_err: Vec<f64>,
    edge_err: Vec<f64>,
    missed: usize,
    falses: usize,
    truths: usize,
    nanos: u128,
    runs: usize,
}

fn score(m: Method, c: &Case) -> Score {
    let mut sc = Score::default();
    for seed in 0..SEEDS {
        let (rt, y, shapes) = draw(c, seed);
        let clicks: Vec<f64> = if c.peaks.is_empty() {
            vec![3.0]
        } else {
            shapes.iter().map(|s| rt[true_limits(s).1]).collect()
        };
        let t0 = Instant::now();
        let found = m(&rt, &y, &clicks);
        sc.nanos += t0.elapsed().as_nanos();
        sc.runs += 1;
        let mut used = vec![false; found.len()];
        for (p, s) in c.peaks.iter().zip(&shapes) {
            sc.truths += 1;
            let (lo, apex, hi) = true_limits(s);
            // the window that holds the true apex, nearest centre first
            let best = found
                .iter()
                .enumerate()
                .filter(|(k, w)| !used[*k] && w.0 <= apex && apex <= w.1)
                .min_by_key(|(_, w)| ((w.0 + w.1) as i64 - 2 * apex as i64).abs());
            match best {
                Some((k, w)) => {
                    used[k] = true;
                    sc.area_err
                        .push(100.0 * (w.2 - true_area(p)) / true_area(p));
                    sc.edge_err.push(
                        ((w.0 as f64 - lo as f64).abs() + (w.1 as f64 - hi as f64).abs()) / 2.0
                            * DT
                            / p.s,
                    );
                }
                None => sc.missed += 1,
            }
        }
        sc.falses += used.iter().filter(|u| !**u).count();
    }
    sc
}

fn mean(v: &[f64]) -> f64 {
    if v.is_empty() {
        f64::NAN
    } else {
        v.iter().sum::<f64>() / v.len() as f64
    }
}

fn mean_abs(v: &[f64]) -> f64 {
    if v.is_empty() {
        f64::NAN
    } else {
        v.iter().map(|x| x.abs()).sum::<f64>() / v.len() as f64
    }
}

const METHODS: [(&str, Method); 8] = [
    ("A cwt", m_cwt),
    ("B page click", m_page),
    ("C prominence", m_prom),
    ("D prom+drop", m_prom_drop),
    ("E prom+asls", m_prom_asls),
    ("F0 bg, edge at 0", m_f0),
    ("F1 bg, edge at 1 sd", m_f1),
    ("F2 = F1 + slope stop", m_f2),
];

#[test]
fn benchmark_table() {
    let cs = cases();
    // MZLAB_BENCH_DUMP=<file>: one trace per case with the edges of the Rust port of autoEdges, for the check against picchi.js
    if let Ok(path) = std::env::var("MZLAB_BENCH_DUMP") {
        let rows: Vec<String> = cs
            .iter()
            .filter(|c| !c.peaks.is_empty())
            .map(|c| {
                let (rt, y, shapes) = draw(c, 1);
                let x = rt[true_limits(&shapes[0]).1];
                let (l, h) = auto_edges(&rt, &y, x);
                format!(
                    "{{\"x\":{x},\"y\":{:?},\"edges\":[{},{}]}}",
                    y, rt[l], rt[h]
                )
            })
            .collect();
        std::fs::write(path, format!("[{}]", rows.join(","))).unwrap();
    }
    let mut all: Vec<Vec<Score>> = Vec::new();
    println!("\n| case | method | mean abs area error % | bias % | mean edge error (sigma) | missed | false | us/trace |");
    println!("|---|---|---|---|---|---|---|---|");
    for c in &cs {
        let mut row = Vec::new();
        for (name, m) in METHODS {
            let s = score(m, c);
            println!(
                "| {} | {} | {:.1} | {:+.1} | {:.2} | {}/{} | {} | {:.0} |",
                c.name,
                name,
                mean_abs(&s.area_err),
                mean(&s.area_err),
                mean(&s.edge_err),
                s.missed,
                s.truths,
                s.falses,
                s.nanos as f64 / s.runs as f64 / 1000.0
            );
            row.push(s);
        }
        all.push(row);
    }
    println!("\n| method | mean abs area error % (all matched peaks) | missed | false |");
    println!("|---|---|---|---|");
    for (k, (name, _)) in METHODS.iter().enumerate() {
        let e: Vec<f64> = all.iter().flat_map(|r| r[k].area_err.clone()).collect();
        let (mi, fa, tr): (usize, usize, usize) = all.iter().fold((0, 0, 0), |a, r| {
            (a.0 + r[k].missed, a.1 + r[k].falses, a.2 + r[k].truths)
        });
        println!(
            "| {} | {:.1} | {}/{} | {} |",
            name,
            mean_abs(&e),
            mi,
            tr,
            fa
        );
    }
    // the chosen method (F2, the last one) keeps its accuracy: mean abs area error per case, no false peaks, few missed ones
    let f2 = METHODS.len() - 1;
    for (c, r) in cs.iter().zip(&all) {
        let s = &r[f2];
        let lim = match c.name {
            n if n.starts_with("shoulder") => 45.0,
            n if n.starts_with("low") => 10.0,
            _ => 5.0,
        };
        assert!(
            s.area_err.is_empty() || mean_abs(&s.area_err) < lim,
            "{}: {:.1} % over {lim}",
            c.name,
            mean_abs(&s.area_err)
        );
        assert_eq!(s.falses, 0, "{}: false peaks", c.name);
    }
    let e: Vec<f64> = all.iter().flat_map(|r| r[f2].area_err.clone()).collect();
    let missed: usize = all.iter().map(|r| r[f2].missed).sum();
    assert!(
        mean_abs(&e) < 6.0 && missed <= 35,
        "overall {:.1} %, {missed} missed",
        mean_abs(&e)
    );
}
