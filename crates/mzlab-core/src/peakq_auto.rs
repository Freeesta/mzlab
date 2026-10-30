//! One-click integration of every peak of a high-resolution XIC (`auto_integrate` of mzlab-wasm, `autoAll` of picchi.js): AsLS background,
//! Savitzky-Golay smoothing scaled to the peak width, apexes from the prominence picking, edges walked out to the noise of the smoothed trace.
//! The method won the benchmark of docs/studi/INTEGRAZIONE_PICCHI.md (tests/integrazione_bench.rs).

use crate::peakq::{detect_prominence, ProminenceParams};
use crate::peaks::{noise_level, savgol};
use crate::signal::asls_baseline;

/// One integrated peak: limits (indices into the trace) and the baseline value at each limit; the area is the trapezoid of the raw trace
/// above the straight line between the two baseline ends.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct AutoPeak {
    pub lo: usize,
    pub hi: usize,
    pub base_lo: f64,
    pub base_hi: f64,
    pub area: f64,
}

#[derive(Clone, Copy, Debug)]
pub struct AutoParams {
    /// Apex picking (the prominence detector).
    pub pick: ProminenceParams,
    /// Half window of the smoothing that sets the edges (points); 0 = 30 % of the FWHM of the tallest peak, between 2 and 30.
    pub half_window: usize,
    /// An edge is where the smoothed trace above the background falls to this many times its own noise.
    pub edge_sd: f64,
    /// ... or where, past half height, its slope stays under this fraction of the steepest flank (a background that is not flat).
    pub slope: f64,
}

impl Default for AutoParams {
    fn default() -> Self {
        AutoParams {
            pick: ProminenceParams {
                local_snr: 5.0,
                ..Default::default()
            },
            half_window: 0,
            edge_sd: 1.0,
            slope: 0.03,
        }
    }
}

/// The background (AsLS, lambda 1e7, raised to the median of the points within 3 noise of it) and the trace above it.
fn background(y: &[f64], noise: f64) -> (Vec<f64>, Vec<f64>) {
    let mut b = asls_baseline(y, 1e7, 0.01, 10, 160);
    let mut bg: Vec<f64> = y
        .iter()
        .zip(&b)
        .map(|(v, w)| v - w)
        .filter(|r| *r < 3.0 * noise)
        .collect();
    crate::sortc::by(&mut bg, |a, c| a.total_cmp(c));
    let off = if bg.is_empty() { 0.0 } else { bg[bg.len() / 2] };
    b.iter_mut().for_each(|v| *v += off);
    let yc = y.iter().zip(&b).map(|(v, w)| v - w).collect();
    (b, yc)
}

/// Indices where `s` first falls to `level` or below, walking left and right from `a`.
fn half_span(s: &[f64], a: usize, level: f64) -> (usize, usize) {
    let (mut l, mut r) = (a, a);
    while l > 0 && s[l] > level {
        l -= 1;
    }
    while r + 1 < s.len() && s[r] > level {
        r += 1;
    }
    (l, r)
}

fn auto_half_window(yc: &[f64]) -> usize {
    let coarse = savgol(yc, 5, 2);
    let top = (0..coarse.len())
        .max_by(|&i, &j| coarse[i].total_cmp(&coarse[j]))
        .unwrap_or(0);
    let (l, r) = half_span(&coarse, top, coarse[top] / 2.0);
    (((r - l) as f64 * 0.3).round() as usize).clamp(2, 30)
}

/// One edge of the peak at `a`, walking by `dir` up to `stop`: (index, true when it ended on the background or at the shared valley).
struct Walk<'a> {
    sc: &'a [f64],
    a: usize,
    thr: f64,
    half: f64,
    cap: usize,
    m: usize,
    flat_slope: f64,
}

impl Walk<'_> {
    fn slope(&self, i: usize) -> f64 {
        (self.sc[(i + 1).min(self.sc.len() - 1)] - self.sc[i.saturating_sub(1)]) / 2.0
    }

    fn edge(&self, dir: i64, stop: usize) -> (usize, bool) {
        let (mut j, mut flat) = (self.a as i64, 0usize);
        loop {
            let ju = j as usize;
            if ju == stop || self.sc[ju] <= self.thr {
                return (ju, true);
            }
            if (j - self.a as i64).unsigned_abs() as usize >= self.cap {
                return (ju, false);
            }
            flat = if self.sc[ju] < self.half && self.slope(ju).abs() < self.flat_slope {
                flat + 1
            } else {
                0
            };
            if flat > self.m {
                return ((j - dir * self.m as i64) as usize, false);
            }
            j += dir;
        }
    }
}

/// What every peak of one trace shares: the trace, its background, the smoothed trace above it and the edge rules.
struct Ctx<'a> {
    rt: &'a [f64],
    y: &'a [f64],
    b: &'a [f64],
    sc: &'a [f64],
    thr: f64,
    m: usize,
    slope: f64,
}

impl Ctx<'_> {
    /// The peak at apex `a`, its edges kept between `left` and `right` (the valleys shared with the neighbours).
    fn peak(&self, a: usize, left: usize, right: usize) -> Option<AutoPeak> {
        let (l2, r2) = half_span(self.sc, a, self.sc[a] / 2.0);
        let fw = (r2 - l2).max(2);
        let mut w = Walk {
            sc: self.sc,
            a,
            thr: self.thr,
            half: self.sc[a] / 2.0,
            cap: 5 * fw,
            m: self.m,
            flat_slope: 0.0,
        };
        let steep = (a.saturating_sub(fw)..=(a + fw).min(self.sc.len() - 1))
            .map(|i| w.slope(i).abs())
            .fold(0.0, f64::max);
        w.flat_slope = self.slope * steep;
        let ((lo, lo_bg), (hi, hi_bg)) = (w.edge(-1, left), w.edge(1, right));
        if hi < lo + 2 {
            return None;
        }
        let end = |i: usize, on_bg: bool| {
            if on_bg {
                self.b[i]
            } else {
                self.b[i] + self.sc[i]
            }
        };
        let (base_lo, base_hi) = (end(lo, lo_bg), end(hi, hi_bg));
        let raw: f64 = (lo + 1..=hi)
            .map(|j| (self.y[j] + self.y[j - 1]) / 2.0 * (self.rt[j] - self.rt[j - 1]))
            .sum();
        Some(AutoPeak {
            lo,
            hi,
            base_lo,
            base_hi,
            area: raw - (base_lo + base_hi) / 2.0 * (self.rt[hi] - self.rt[lo]),
        })
    }
}

/// Every peak whose apex is between `x0` and `x1` (minutes), by RT. Neighbours share at most their valley point, so the windows never
/// overlap. At a background or valley edge the baseline end is the background; at a slope stop it is the smoothed level of the trace.
pub fn auto_integrate(rt: &[f64], y: &[f64], x0: f64, x1: f64, p: AutoParams) -> Vec<AutoPeak> {
    let n = rt.len().min(y.len());
    let (rt, y) = (&rt[..n], &y[..n]);
    if n < 5 {
        return Vec::new();
    }
    let pick = ProminenceParams {
        smooth: p.pick.smooth.max(3) | 1,
        min_scans: p.pick.min_scans.max(2),
        ..p.pick
    };
    let mut apex: Vec<usize> = detect_prominence(rt, y, pick)
        .iter()
        .map(|k| k.apex_i)
        .collect();
    if apex.is_empty() {
        return Vec::new();
    }
    crate::sortc::by(&mut apex, |u, v| u.cmp(v));
    let noise = noise_level(y).max(1e-9);
    let (b, yc) = background(y, noise);
    let m = if p.half_window > 0 {
        p.half_window.min(n / 2 - 1).max(1)
    } else {
        auto_half_window(&yc)
    };
    let sc = savgol(&yc, 2 * m + 1, 2);
    let mf = m as f64;
    // noise of the smoothed trace: the raw noise times the root of the central weight of the quadratic SG filter
    let w0 = 3.0 * (3.0 * mf * mf + 3.0 * mf - 1.0)
        / ((2.0 * mf - 1.0) * (2.0 * mf + 1.0) * (2.0 * mf + 3.0));
    let thr = p.edge_sd * noise * w0.sqrt();
    let valley = |a: usize, c: usize| {
        (a + 1..c)
            .min_by(|&i, &j| sc[i].total_cmp(&sc[j]))
            .unwrap_or(a)
    };
    let ctx = Ctx {
        rt,
        y,
        b: &b,
        sc: &sc,
        thr,
        m,
        slope: p.slope,
    };
    let (lo_t, hi_t) = (x0.min(x1), x0.max(x1));
    let mut out = Vec::new();
    for (q, &a) in apex.iter().enumerate() {
        if rt[a] < lo_t || rt[a] > hi_t {
            continue;
        }
        let left = if q > 0 { valley(apex[q - 1], a) } else { 0 };
        let right = if q + 1 < apex.len() {
            valley(a, apex[q + 1])
        } else {
            n - 1
        };
        out.extend(ctx.peak(a, left, right));
    }
    out
}
