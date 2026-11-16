//! Every peak of a jagged low-resolution SIM/MRM trace in one step (`lr_auto_integrate` of mzlab-wasm, `lrAll` of picchi.js, `mzlab-cli peak-auto-lr`).
//! Detection runs on a smoothed copy (median of 3 against single-point spikes, then the quadratic Savitzky-Golay of `peaks::savgol`); neighbouring
//! maxima merge unless the valley between them is deep enough; the area is the trapezoid of the RAW trace above a straight baseline whose ends are
//! raised to zero where they fall below it but never set above the signal; the part of the trace under that line is cut off and reported as
//! `below`, so no area is ever negative (docs/agenti/dati-integrazione.md). Measured with tools/lr_auto_misura.py.

use std::cmp::Reverse;
use std::collections::BinaryHeap;

use crate::baseline::integrate;
use crate::peakq_auto::background;
use crate::peaks::{noise_level, savgol};

/// How two touching peaks share their valley.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Split {
    /// Drop line: one straight baseline under the whole group, a vertical cut at each valley.
    Drop,
    /// Valley to valley: each part's baseline ends on the trace at the valley.
    Valley,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct LrAutoParams {
    /// Half window of the Savitzky-Golay smoothing of the detection copy (points); 0 = `AUTO_FRAC` of the FWHM of the tallest peak, 2 to 30.
    pub half: usize,
    /// Two maxima stay two peaks only when the smaller one rises above the valley between them by this fraction of its own height.
    pub valley: f64,
    /// Smallest height of a peak over the noise (the smoothed height above the background over the MAD noise of the raw trace).
    pub snr: f64,
    /// Fewest points of a peak.
    pub min_pts: usize,
    /// Median of 3 before the smoothing (single-point spikes never become peaks).
    pub despike: bool,
    pub split: Split,
}

impl Default for LrAutoParams {
    fn default() -> Self {
        LrAutoParams {
            half: 0,
            valley: 0.10,
            snr: 5.0,
            min_pts: 5,
            despike: true,
            split: Split::Drop,
        }
    }
}

impl LrAutoParams {
    /// The numbers of the page dialog (wasm interface), in the order half, valley (%), snr, min_pts, despike (0/1), split (0 drop, 1 valley); a missing or
    /// negative one keeps its default.
    pub fn from_slice(p: &[f64]) -> LrAutoParams {
        let d = LrAutoParams::default();
        let g = |i: usize| p.get(i).copied().filter(|v| v.is_finite() && *v >= 0.0);
        LrAutoParams {
            half: g(0).map_or(d.half, |v| v as usize),
            valley: g(1).map_or(d.valley, |v| (v / 100.0).min(1.0)),
            snr: g(2).unwrap_or(d.snr),
            min_pts: g(3).map_or(d.min_pts, |v| v as usize),
            despike: g(4).map_or(d.despike, |v| v != 0.0),
            split: if g(5) == Some(1.0) {
                Split::Valley
            } else {
                Split::Drop
            },
        }
    }
}

/// How one end of a peak was found.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum End {
    /// The smoothed trace came down to the background (or the trace ended).
    Background,
    /// The valley shared with the neighbour.
    Valley,
    /// The walk stopped at 5 FWHM from the apex, still above the background.
    Cap,
}

/// What the clamp of a baseline end changed.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Clamp {
    /// The end is the engine's level as found.
    None,
    /// The level was below zero and was raised (to zero, or to the trace when the trace itself is below zero).
    Zero,
    /// The level was above the raw trace and was lowered onto it.
    Trace,
}

/// One peak: indices into the trace, the baseline at each end (counts) and what its clamp did, areas (counts x min, raw trace: `area` the part
/// above the baseline, `below` the part under it that was cut off, `area_zero` the part above zero; never negative), height above the baseline
/// (raw) and its S/N (over the MAD noise of the trace).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct LrAutoPeak {
    pub lo: usize,
    pub hi: usize,
    pub apex: usize,
    pub base_lo: f64,
    pub base_hi: f64,
    pub area: f64,
    pub below: f64,
    pub area_zero: f64,
    pub height: f64,
    pub sn: f64,
    pub end_lo: End,
    pub end_hi: End,
    pub clamp_lo: Clamp,
    pub clamp_hi: Clamp,
}

/// The result with every choice the engine made: the half window used (and whether it was chosen automatically) and the noise.
#[derive(Clone, Debug, PartialEq)]
pub struct LrAuto {
    pub half: usize,
    pub half_auto: bool,
    pub noise: f64,
    pub params: LrAutoParams,
    pub peaks: Vec<LrAutoPeak>,
}

/// The MAD noise of the first differences; when more than half the steps are equal (a flat or regular trace) their RMS instead, so the noise is
/// never zero.
fn robust_noise(y: &[f64]) -> f64 {
    let mad = noise_level(y);
    if mad > 0.0 {
        return mad;
    }
    let d: f64 = y.windows(2).map(|w| (w[1] - w[0]).powi(2)).sum::<f64>() / (y.len() - 1) as f64;
    (d / 2.0).sqrt().max(1e-9)
}

/// The share of the FWHM (points) of the tallest peak taken as the automatic half window (tools/synthetic_data_jagged.py, PR body).
pub const AUTO_FRAC: f64 = 0.4;

/// The automatic half window: `AUTO_FRAC` of the FWHM of the tallest maximum of a coarse smoothing (window 5), between 2 and 30 points.
fn auto_half(d: &[f64]) -> usize {
    let c = savgol(d, 5, 2);
    let top = (0..c.len()).fold(0, |m, i| if c[i] > c[m] { i } else { m });
    let h = c[top] / 2.0;
    let (mut l, mut r) = (top, top);
    while l > 0 && c[l] > h {
        l -= 1;
    }
    while r + 1 < c.len() && c[r] > h {
        r += 1;
    }
    (((r - l) as f64 * AUTO_FRAC).round() as usize).clamp(2, 30)
}

/// A heap key: the f64 ordered by `total_cmp`, so the merge order is the same on every target.
#[derive(Clone, Copy, PartialEq)]
struct Key(f64);
impl Eq for Key {}
impl PartialOrd for Key {
    fn partial_cmp(&self, o: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(o))
    }
}
impl Ord for Key {
    fn cmp(&self, o: &Self) -> std::cmp::Ordering {
        self.0.total_cmp(&o.0)
    }
}

fn median3(y: &[f64]) -> Vec<f64> {
    let n = y.len();
    (0..n)
        .map(|i| {
            if i == 0 || i + 1 == n {
                return y[i];
            }
            let (a, b, c) = (y[i - 1], y[i], y[i + 1]);
            a.max(b).min(a.min(b).max(c))
        })
        .collect()
}

/// The first lowest point of `s` strictly between `a` and `b`.
fn lowest(s: &[f64], a: usize, b: usize) -> usize {
    (a + 1..b).fold(a + 1, |m, j| if s[j] < s[m] { j } else { m })
}

/// The maxima of `s` above `thr` that survive the valley rule, by index. Pairs merge worst first (the shallowest valley against what it needs); of a
/// merged pair the lower maximum goes and the group keeps the lower of the two valleys. A heap with lazy entries keeps it O(k log k).
fn maxima(s: &[f64], thr: f64, valley: f64, floor: f64) -> Vec<usize> {
    let ap: Vec<usize> = (1..s.len().saturating_sub(1))
        .filter(|&j| s[j] > thr && s[j] > s[j - 1] && s[j] >= s[j + 1])
        .collect();
    let k = ap.len();
    if k < 2 {
        return ap;
    }
    let none = usize::MAX;
    let (mut prev, mut next): (Vec<usize>, Vec<usize>) = (
        (0..k).map(|i| i.wrapping_sub(1)).collect(),
        (1..=k).map(|i| if i < k { i } else { none }).collect(),
    );
    let mut vly: Vec<usize> = (0..k)
        .map(|i| {
            if i + 1 < k {
                lowest(s, ap[i], ap[i + 1])
            } else {
                0
            }
        })
        .collect();
    let (mut ver, mut alive) = (vec![0u32; k], vec![true; k]);
    let need = |i: usize, j: usize, v: usize| {
        let small = s[ap[i]].min(s[ap[j]]);
        (small - s[v]) - (valley * small).max(floor)
    };
    let mut heap = BinaryHeap::new();
    let key = |x: f64| Reverse(Key(x));
    for (i, &v) in vly.iter().enumerate().take(k - 1) {
        heap.push((key(need(i, i + 1, v)), Reverse(i), 0u32));
    }
    while let Some((Reverse(Key(d)), Reverse(i), v)) = heap.pop() {
        if !alive[i] || v != ver[i] || next[i] == none {
            continue;
        }
        if d >= 0.0 {
            break;
        }
        let j = next[i];
        if s[ap[i]] < s[ap[j]] {
            alive[i] = false;
            let p = prev[i];
            prev[j] = p;
            if p != none {
                vly[p] = if s[vly[i]] < s[vly[p]] {
                    vly[i]
                } else {
                    vly[p]
                };
                next[p] = j;
                ver[p] += 1;
                heap.push((key(need(p, j, vly[p])), Reverse(p), ver[p]));
            }
        } else {
            alive[j] = false;
            let q = next[j];
            next[i] = q;
            ver[i] += 1;
            if q != none {
                prev[q] = i;
                vly[i] = if s[vly[j]] < s[vly[i]] {
                    vly[j]
                } else {
                    vly[i]
                };
                heap.push((key(need(i, q, vly[i])), Reverse(i), ver[i]));
            }
        }
    }
    (0..k).filter(|&i| alive[i]).map(|i| ap[i]).collect()
}

/// One end of the peak at `a`, walking by `dir` to `stop` (the valley with the neighbour, or the trace end when `stop_valley` is false).
fn walk(
    s: &[f64],
    a: usize,
    dir: i64,
    stop: usize,
    stop_valley: bool,
    thr: f64,
    cap: usize,
) -> (usize, End) {
    let mut j = a;
    loop {
        if j == stop {
            return (
                j,
                if stop_valley {
                    End::Valley
                } else {
                    End::Background
                },
            );
        }
        if s[j] <= thr {
            return (j, End::Background);
        }
        if j.abs_diff(a) >= cap {
            return (j, End::Cap);
        }
        j = (j as i64 + dir) as usize;
    }
}

/// Every peak whose apex is between `x0` and `x1` (min), by RT; neighbours share at most their valley point.
pub fn lr_auto_integrate(rt: &[f64], y: &[f64], x0: f64, x1: f64, p: LrAutoParams) -> LrAuto {
    let n = rt.len().min(y.len());
    let (rt, y) = (&rt[..n], &y[..n]);
    let mut out = LrAuto {
        half: p.half,
        half_auto: p.half == 0,
        noise: 0.0,
        params: p,
        peaks: Vec::new(),
    };
    if n < 7 {
        return out;
    }
    let noise = robust_noise(y);
    out.noise = noise;
    let (b, yc) = background(y, noise);
    let det = if p.despike { median3(&yc) } else { yc };
    let m = if p.half > 0 {
        p.half.min((n - 1) / 2).max(1)
    } else {
        auto_half(&det)
    };
    out.half = m;
    let s = savgol(&det, 2 * m + 1, 2);
    let mf = m as f64;
    // noise of the smoothed copy: the raw noise times the root of the central weight of the quadratic filter
    let w0 = 3.0 * (3.0 * mf * mf + 3.0 * mf - 1.0)
        / ((2.0 * mf - 1.0) * (2.0 * mf + 1.0) * (2.0 * mf + 3.0));
    let thr = noise * w0.sqrt();
    let apex: Vec<usize> = maxima(&s, thr, p.valley, 3.0 * thr)
        .into_iter()
        .filter(|&a| s[a] >= p.snr * noise)
        .collect();
    out.peaks = peaks_of(
        &Trace {
            rt,
            y,
            b: &b,
            s: &s,
            thr,
        },
        &apex,
        p,
    )
    .into_iter()
    .filter(|k| rt[k.apex] >= x0.min(x1) && rt[k.apex] <= x0.max(x1))
    .collect();
    for k in out.peaks.iter_mut() {
        k.sn = k.height / noise;
    }
    out
}

struct Trace<'a> {
    rt: &'a [f64],
    y: &'a [f64],
    b: &'a [f64],
    s: &'a [f64],
    thr: f64,
}

impl Trace<'_> {
    /// A baseline end on the trace at `i`: the background, or the smoothed level on a cut or a cap, clamped (`clamp`).
    fn end(&self, i: usize, e: End, split: Split) -> (f64, Clamp) {
        let on = matches!(e, End::Cap) || (matches!(e, End::Valley) && split == Split::Valley);
        let v = if on { self.b[i] + self.s[i] } else { self.b[i] };
        clamp(v, self.y[i])
    }
}

/// A baseline level `v` at a point of raw value `y`: raised to zero when below it, then never above `y` (the trace wins over zero: on a trace
/// below zero the end sits on the trace, never above it).
fn clamp(v: f64, y: f64) -> (f64, Clamp) {
    let c = v.max(0.0).min(y);
    let how = if c < v {
        Clamp::Trace
    } else if c > v {
        Clamp::Zero
    } else {
        Clamp::None
    };
    (c, how)
}

/// The two ends of a peak and why each stopped.
type Ends = (usize, End, usize, End);
/// The baseline level at the two ends of a peak and what each clamp did.
type Base = ((f64, Clamp), (f64, Clamp));

/// From each apex out to the background, the valley shared with a neighbour, or 5 FWHM of the smoothed copy.
fn edges(t: &Trace, apex: &[usize]) -> Vec<Ends> {
    let n = t.s.len();
    let vl: Vec<usize> = apex.windows(2).map(|w| lowest(t.s, w[0], w[1])).collect();
    let mut ends: Vec<Ends> = Vec::new();
    for (q, &a) in apex.iter().enumerate() {
        let (l2, r2) = {
            let h = t.s[a] / 2.0;
            let (mut l, mut r) = (a, a);
            while l > 0 && t.s[l] > h {
                l -= 1;
            }
            while r + 1 < n && t.s[r] > h {
                r += 1;
            }
            (l, r)
        };
        let cap = 5 * (r2 - l2).max(2);
        let (ls, lv) = if q > 0 {
            (vl[q - 1], t.s[vl[q - 1]] > t.thr)
        } else {
            (0, false)
        };
        let (rs, rv) = if q < vl.len() {
            (vl[q], t.s[vl[q]] > t.thr)
        } else {
            (n - 1, false)
        };
        let (lo, el) = walk(t.s, a, -1, ls, lv, t.thr, cap);
        let (hi, eh) = walk(t.s, a, 1, rs, rv, t.thr, cap);
        ends.push((lo, el, hi, eh));
    }
    ends
}

/// A drop line takes, at each cut, the straight line under the whole group of touching peaks (clamped as any end).
fn drop_lines(t: &Trace, ends: &[Ends], base: &mut [Base]) {
    let mut g0 = 0;
    for q in 0..ends.len() {
        let last = q + 1 == ends.len() || ends[q].3 != End::Valley;
        if !last {
            continue;
        }
        let (i0, i1) = (ends[g0].0, ends[q].2);
        let (y0, y1) = (base[g0].0 .0, base[q].1 .0);
        let at = |i: usize| {
            let w = t.rt[i1] - t.rt[i0];
            let v = if w > 0.0 {
                y0 + (y1 - y0) * (t.rt[i] - t.rt[i0]) / w
            } else {
                y0
            };
            clamp(v, t.y[i])
        };
        for r in g0..q {
            let c = at(ends[r].2);
            base[r].1 = c;
            base[r + 1].0 = c;
        }
        g0 = q + 1;
    }
}

fn peaks_of(t: &Trace, apex: &[usize], p: LrAutoParams) -> Vec<LrAutoPeak> {
    let ends = edges(t, apex);
    // the baseline of every end: a drop line takes, at each cut, the straight line under the whole group of touching peaks (clamped as any end)
    let mut base: Vec<Base> = ends
        .iter()
        .map(|&(lo, el, hi, eh)| (t.end(lo, el, p.split), t.end(hi, eh, p.split)))
        .collect();
    if p.split == Split::Drop {
        drop_lines(t, &ends, &mut base);
    }
    ends.iter()
        .zip(&base)
        .zip(apex)
        .filter(|(((lo, _, hi, _), _), _)| hi + 1 >= lo + p.min_pts.max(2))
        .map(|(((lo, el, hi, eh), &((b0, c0), (b1, c1))), &a)| {
            let (lo, hi) = (*lo, *hi);
            let w = t.rt[hi] - t.rt[lo];
            let line = |x: f64| {
                if w > 0.0 {
                    b0 + (b1 - b0) * (x - t.rt[lo]) / w
                } else {
                    b0
                }
            };
            let r = integrate(t.rt, t.y, lo, hi, line);
            let z = integrate(t.rt, t.y, lo, hi, |_| 0.0);
            // the signed integral minus the part under the line: only where the trace is above it, so never negative
            let above = |i: crate::baseline::Integral| (i.area + i.below).max(0.0);
            LrAutoPeak {
                lo,
                hi,
                apex: a,
                base_lo: b0,
                base_hi: b1,
                area: above(r),
                below: r.below,
                area_zero: above(z),
                height: r.height,
                sn: 0.0,
                end_lo: *el,
                end_hi: *eh,
                clamp_lo: c0,
                clamp_hi: c1,
            }
        })
        .collect()
}

fn clamp_name(c: Clamp) -> &'static str {
    match c {
        Clamp::None => "none",
        Clamp::Zero => "zero",
        Clamp::Trace => "trace",
    }
}

fn end_name(e: End) -> &'static str {
    match e {
        End::Background => "background",
        End::Valley => "valley",
        End::Cap => "cap",
    }
}

/// The result as the JSON the page (`lr_auto_integrate` of mzlab-wasm) and `mzlab-cli peak-auto-lr` both print, the same bytes: the parameters used
/// (`half_auto` true when the half window was chosen here, `baseline` always "engine": this route never reads the page's baseline setting) and every
/// peak (times in min, baseline in counts with what its clamp did, areas in counts x min, never negative, `below` the part cut off under the line).
pub fn lr_auto_json(rt: &[f64], r: &LrAuto) -> String {
    let p = &r.params;
    let head = format!(
        "{{\"params\":{{\"half\":{},\"half_auto\":{},\"valley\":{},\"sn\":{},\"min_pts\":{},\"despike\":{},\"split\":\"{}\",\"baseline\":\"engine\",\"noise\":{}}}",
        r.half, r.half_auto, p.valley, p.snr, p.min_pts, p.despike, if p.split == Split::Drop { "drop" } else { "valley" }, r.noise
    );
    let rows: Vec<String> = r
        .peaks
        .iter()
        .map(|k| {
            format!(
                "{{\"lo\":{},\"hi\":{},\"t0\":{},\"t1\":{},\"rt\":{},\"b0\":{},\"b1\":{},\"area\":{},\"below\":{},\"area_zero\":{},\"height\":{},\"sn\":{},\"end0\":\"{}\",\"end1\":\"{}\",\"clamp0\":\"{}\",\"clamp1\":\"{}\"}}",
                k.lo, k.hi, rt[k.lo], rt[k.hi], rt[k.apex], k.base_lo, k.base_hi, k.area, k.below, k.area_zero, k.height, k.sn, end_name(k.end_lo), end_name(k.end_hi),
                clamp_name(k.clamp_lo), clamp_name(k.clamp_hi)
            )
        })
        .collect();
    format!("{head},\"peaks\":[{}]}}", rows.join(","))
}

#[cfg(test)]
#[path = "peakq_lrauto_test.rs"]
mod tests;
