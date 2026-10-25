//! Broad-band detector for the 1D trace of one ion: peaks 30-300 s wide (overloaded basic compounds, interconverting species) that the narrow-peak
//! detectors lose. Three steps: a ridge test on the narrow regions already found, linear inpainting under the true narrow peaks only, and one
//! multiscale Ricker pass on the residue decimated by `decimate`. Used by `centwave::refine` and `untargeted::resolve`; off by default.
//! Sums run in a fixed order and `exp` comes from `libm`: native and WASM give the same bits.

use mzlab_core::error::Result;

use crate::centwave::ricker;
use crate::trace::{filled, reserve};

/// Standard deviation of a Gaussian over its full width at half maximum.
const FWHM: f64 = 2.354_820_045_030_949_3;
/// Number of scales of the multiscale pass.
const SCALES: usize = 6;
/// Narrow scales of the ridge test, FWHM in seconds.
const RIDGE_S: [f64; 3] = [15.0, 22.0, 30.0];
/// A candidate needs at least this fraction of `width_s.0` between its half-height points.
const MIN_HALF_WIDTH: f64 = 0.75;
/// The border walk stops where the signal is this fraction of the height over the base (never below two noise units).
const EDGE: f64 = 0.03;

/// Parameters of the broad-band pass (`enabled: false` leaves every table as it was).
#[derive(Clone, Debug, PartialEq)]
pub struct BroadParams {
    pub enabled: bool,
    /// Scans per block of the decimated residue.
    pub decimate: usize,
    /// Range of the full width at half maximum of the bands, seconds.
    pub width_s: (f64, f64),
    /// Minimum height over the local bridge, in units of the noise of the decimated residue.
    pub snr: f64,
    /// Traces shorter than this (scans) are not searched.
    pub min_scans: usize,
    /// Minimum height over the local bridge, in intensity units.
    pub noise: f64,
}

impl Default for BroadParams {
    fn default() -> Self {
        BroadParams {
            enabled: false,
            decimate: 8,
            width_s: (30.0, 300.0),
            snr: 8.0,
            min_scans: 40,
            noise: 500.0,
        }
    }
}

/// A broad band: scans and RT (same unit as the `rt` given in), height over the bridge, area in scan units (like `centwave::Seg`) and in
/// time x intensity (like `untargeted::Feature`), both over the linear bridge between the edge blocks.
#[derive(Clone, Debug, PartialEq)]
pub struct BroadPeak {
    pub scan_lo: usize,
    pub scan_hi: usize,
    pub scan_apex: usize,
    pub rt_apex: f64,
    pub rt_lo: f64,
    pub rt_hi: f64,
    pub mz: f64,
    pub height: f64,
    pub area: f64,
    pub area_s: f64,
    pub snr: f64,
    pub scale_s: f64,
}

/// Standard deviation (in samples) of the Ricker wavelet whose full width at half maximum is `fwhm_s`.
fn sigma(fwhm_s: f64, dt_s: f64) -> f64 {
    (fwhm_s / dt_s / FWHM).max(0.8)
}

/// Response of the Ricker wavelet of standard deviation `s` samples centred on sample `t` (taps outside the series are dropped).
fn response_at(y: &[f64], t: usize, s: f64) -> f64 {
    let half = libm::ceil(4.0 * s) as isize;
    let (i0, i1) = (
        (t as isize - half).max(0) as usize,
        (t + half as usize + 1).min(y.len()),
    );
    let mut acc = 0.0;
    for (i, v) in y.iter().enumerate().take(i1).skip(i0) {
        acc += v * ricker((i as f64 - t as f64) / s);
    }
    acc / libm::sqrt(s)
}

fn argmax(y: &[f64], lo: usize, hi: usize) -> usize {
    let mut at = lo;
    for (i, v) in y.iter().enumerate().take(hi + 1).skip(lo) {
        if *v > y[at] {
            at = i;
        }
    }
    at
}

/// True when the maximum of `y[lo..=hi]` belongs to a structure wider than the narrow scales: its Ricker response keeps growing from 15 to 22 to 30 s
/// (and is positive), so inpainting would cut the crest of a band. False for a true narrow peak (the response is already falling at 30 s).
/// A ragged crest can read false and a moderate narrow peak on the steep flank of an intense band can read true: both are accepted limits.
pub fn is_broad_ridge(y: &[f64], lo: usize, hi: usize, dt_s: f64) -> Result<bool> {
    if y.is_empty() || lo > hi || hi >= y.len() || dt_s.is_nan() || dt_s <= 0.0 {
        return Ok(false);
    }
    let t = argmax(y, lo, hi);
    let r = RIDGE_S.map(|w| response_at(y, t, sigma(w, dt_s)));
    Ok(r[2] > 0.0 && r[2] >= r[1] && r[1] >= r[0])
}

/// Replaces every `(s, e)` (both included) by the straight line between the points just outside it (flat at an edge, unchanged without anchors).
pub fn inpaint_linear(y: &mut [f64], regions: &[(usize, usize)]) {
    let n = y.len();
    for &(s, e) in regions {
        if s > e || e >= n {
            continue;
        }
        match (s.checked_sub(1), (e + 1 < n).then_some(e + 1)) {
            (Some(l), Some(r)) => {
                let (yl, yr) = (y[l], y[r]);
                for (j, v) in y[l + 1..r].iter_mut().enumerate() {
                    *v = yl + (yr - yl) * (j + 1) as f64 / (r - l) as f64;
                }
            }
            (None, Some(r)) => {
                let v = y[r];
                y[..r].fill(v);
            }
            (Some(l), None) => {
                let v = y[l];
                y[l + 1..].fill(v);
            }
            (None, None) => {}
        }
    }
}

/// Blocks of `k` consecutive points: mean RT, mean intensity and m/z weighted by intensity (plain mean of the block when all weights are 0).
/// The last block may be shorter. Returns `(rt, y, mz)`.
pub fn decimate(
    rt: &[f64],
    mz: &[f64],
    y: &[f64],
    k: usize,
) -> Result<(Vec<f64>, Vec<f64>, Vec<f64>)> {
    let k = k.max(1);
    let n = rt.len().min(mz.len()).min(y.len());
    let m = n.div_ceil(k);
    let (mut r, mut v, mut z) = (Vec::new(), Vec::new(), Vec::new());
    reserve(&mut r, m)?;
    reserve(&mut v, m)?;
    reserve(&mut z, m)?;
    for b in 0..m {
        let (a, e) = (b * k, ((b + 1) * k).min(n));
        let c = (e - a) as f64;
        let (mut sr, mut sy, mut sw, mut sm) = (0.0, 0.0, 0.0, 0.0);
        for ((&t, &v), &m) in rt[a..e].iter().zip(&y[a..e]).zip(&mz[a..e]) {
            sr += t;
            sy += v;
            sw += v * m;
            sm += m;
        }
        r.push(sr / c);
        v.push(sy / c);
        z.push(if sy > 0.0 { sw / sy } else { sm / c });
    }
    Ok((r, v, z))
}

fn median(v: &mut [f64]) -> f64 {
    if v.is_empty() {
        return 0.0;
    }
    v.sort_by(|a, b| a.total_cmp(b));
    let m = v.len() / 2;
    if v.len() % 2 == 1 {
        v[m]
    } else {
        (v[m - 1] + v[m]) / 2.0
    }
}

/// Noise of a short series: 1.4826 x median of |first difference| / sqrt 2 (robust to the bands themselves; no reach of a peak is involved).
fn noise_of(y: &[f64]) -> Result<f64> {
    let mut d = Vec::new();
    reserve(&mut d, y.len())?;
    d.extend(y.windows(2).map(|w| (w[1] - w[0]).abs()));
    Ok(1.4826 * median(&mut d) / core::f64::consts::SQRT_2)
}

/// Lower quartile of a series: the base of the bands.
fn base_of(y: &[f64]) -> Result<f64> {
    let mut v = Vec::new();
    reserve(&mut v, y.len())?;
    v.extend_from_slice(y);
    v.sort_by(|a, b| a.total_cmp(b));
    Ok(v.get(v.len() / 4).copied().unwrap_or(0.0))
}

/// One candidate: the sample, the scale (index) and the response that found it.
struct Cand {
    t: usize,
    s: usize,
    r: f64,
}

/// The candidates: local maxima of the response at every scale, strongest first kept when two lie within the larger standard deviation.
fn candidates(y: &[f64], sig: &[f64]) -> Result<Vec<Cand>> {
    let n = y.len();
    let mut all: Vec<Cand> = Vec::new();
    for (si, &s) in sig.iter().enumerate() {
        let mut r = filled(n, 0.0f64)?;
        for (t, out) in r.iter_mut().enumerate() {
            *out = response_at(y, t, s);
        }
        for (t, &v) in r.iter().enumerate() {
            let left = t == 0 || v >= r[t - 1];
            let right = t + 1 == n || v > r[t + 1];
            if v > 0.0 && left && right {
                reserve(&mut all, 1)?;
                all.push(Cand { t, s: si, r: v });
            }
        }
    }
    all.sort_by(|a, b| b.r.total_cmp(&a.r).then(a.t.cmp(&b.t)));
    let mut kept: Vec<Cand> = Vec::new();
    for c in all {
        let near = kept
            .iter()
            .any(|k| (k.t as f64 - c.t as f64).abs() <= sig[k.s].max(sig[c.s]).ceil());
        if !near {
            reserve(&mut kept, 1)?;
            kept.push(c);
        }
    }
    kept.sort_by_key(|c| c.t);
    Ok(kept)
}

/// Walks from `t` towards `limit` (included) and stops where the smoothed signal falls to `thr`, or rises again by more than `rise` over its running minimum.
fn walk(sm: &[f64], t: usize, thr: f64, rise: f64, limit: usize, up: bool) -> usize {
    let (mut at, mut low) = (t, sm[t]);
    while at != limit {
        let next = if up { at + 1 } else { at - 1 };
        if sm[next] <= thr {
            return next;
        }
        if sm[next] > low + rise {
            return at;
        }
        low = low.min(sm[next]);
        at = next;
    }
    at
}

/// Half-height width of the band around `t`, in samples.
fn half_width(sm: &[f64], t: usize, base: f64) -> usize {
    let half = base + (sm[t] - base) / 2.0;
    let (mut a, mut z) = (t, t);
    while a > 0 && sm[a - 1] >= half {
        a -= 1;
    }
    while z + 1 < sm.len() && sm[z + 1] >= half {
        z += 1;
    }
    z - a + 1
}

/// Broad bands of one trace. `rt_s` seconds, `mz`/`y` per scan, `narrow` the inclusive scan ranges of the narrow peaks already found. The narrow
/// peaks that are not the crest of a band are inpainted away, the residue is decimated and searched with Ricker wavelets of `p.width_s`. A band
/// needs a half-height width of at least 0.75 `width_s.0`, a height over the bridge of `p.snr` noise units and of `p.noise`. The height is that of the
/// decimated residue; the area is taken on the inpainted residue over the line between the two edge blocks. Without the common mode of the other
/// traces a drift reads as a band: keep it off unless the traces are flat.
pub fn detect_broad(
    rt_s: &[f64],
    mz: &[f64],
    y: &[f64],
    narrow: &[(usize, usize)],
    p: &BroadParams,
) -> Result<Vec<BroadPeak>> {
    let len = y.len();
    if !p.enabled || len < p.min_scans || rt_s.len() != len || mz.len() != len || len < 2 {
        return Ok(Vec::new());
    }
    let mut d = Vec::new();
    reserve(&mut d, len)?;
    d.extend(rt_s.windows(2).map(|w| w[1] - w[0]));
    let dt_s = median(&mut d);
    if dt_s.is_nan() || dt_s <= 0.0 {
        return Ok(Vec::new());
    }
    let mut cut: Vec<(usize, usize)> = Vec::new();
    for &(lo, hi) in narrow {
        if !is_broad_ridge(y, lo, hi, dt_s)? {
            reserve(&mut cut, 1)?;
            cut.push((lo, hi));
        }
    }
    let mut y_res = filled(len, 0.0f64)?;
    y_res.copy_from_slice(y);
    inpaint_linear(&mut y_res, &cut);
    let r = Residue::new(rt_s, mz, y, y_res, dt_s, p)?;
    let widths = scale_widths(p);
    let sig: Vec<f64> = widths.iter().map(|&w| sigma(w, r.dt_d)).collect();
    // only candidates that clear the height bars limit the borders of their neighbours
    let bar = (p.snr * r.noise).max(p.noise);
    let mut cands = candidates(&r.y_d, &sig)?;
    cands.retain(|c| r.sm[c.t] - r.base >= bar);
    let mut out = Vec::new();
    for (ci, c) in cands.iter().enumerate() {
        let thr = r.base + (EDGE * (r.sm[c.t] - r.base)).max(2.0 * r.noise);
        let prev = ci.checked_sub(1).map_or(0, |j| cands[j].t);
        let next = cands.get(ci + 1).map_or(r.y_d.len() - 1, |x| x.t);
        let lo_d = walk(&r.sm, c.t, thr, 4.0 * r.noise, prev, false);
        let hi_d = walk(&r.sm, c.t, thr, 4.0 * r.noise, next, true);
        if let Some(b) = r.band(c.t, lo_d, hi_d, widths[c.s]) {
            reserve(&mut out, 1)?;
            out.push(b);
        }
    }
    Ok(out)
}

/// The `SCALES` full widths (seconds), geometric from `width_s.0` to `width_s.1`.
fn scale_widths(p: &BroadParams) -> Vec<f64> {
    let ratio = p.width_s.1 / p.width_s.0.max(1e-9);
    let step = if ratio > 1.0 {
        libm::pow(ratio, 1.0 / (SCALES - 1) as f64)
    } else {
        1.0
    };
    (0..SCALES)
        .map(|i| p.width_s.0 * libm::pow(step, i as f64))
        .collect()
}

/// The trace after inpainting and decimation, with what is needed to measure a band on it.
struct Residue<'a> {
    rt_s: &'a [f64],
    mz: &'a [f64],
    y: &'a [f64],
    y_res: Vec<f64>,
    rt_d: Vec<f64>,
    y_d: Vec<f64>,
    /// `y_d` smoothed over three blocks.
    sm: Vec<f64>,
    k: usize,
    dt_d: f64,
    noise: f64,
    base: f64,
    p: &'a BroadParams,
}

impl<'a> Residue<'a> {
    fn new(
        rt_s: &'a [f64],
        mz: &'a [f64],
        y: &'a [f64],
        y_res: Vec<f64>,
        dt_s: f64,
        p: &'a BroadParams,
    ) -> Result<Self> {
        let k = p.decimate.max(1);
        let (rt_d, y_d, _) = decimate(rt_s, mz, &y_res, k)?;
        let n = y_d.len();
        let mut sm = filled(n, 0.0f64)?;
        for (i, out) in sm.iter_mut().enumerate() {
            let (a, z) = (i.saturating_sub(1), (i + 1).min(n - 1));
            *out = y_d[a..=z].iter().sum::<f64>() / (z - a + 1) as f64;
        }
        let (noise, base) = (noise_of(&y_d)?, base_of(&y_d)?);
        Ok(Residue {
            rt_s,
            mz,
            y,
            y_res,
            rt_d,
            y_d,
            sm,
            k,
            dt_d: dt_s * k as f64,
            noise,
            base,
            p,
        })
    }

    /// The band around block `t` between the blocks `lo_d` and `hi_d`: height and area over the line between the two edge blocks; `None` when it is
    /// not wide enough or does not clear the height bars.
    fn band(&self, t: usize, lo_d: usize, hi_d: usize, scale_s: f64) -> Option<BroadPeak> {
        let (k, len) = (self.k, self.y.len());
        let (lo, hi) = (lo_d * k, ((hi_d + 1) * k - 1).min(len - 1));
        let (b_lo, b_hi) = (self.y_d[lo_d], self.y_d[hi_d]);
        let bridge = |i: usize| {
            if hi == lo {
                b_lo
            } else {
                b_lo + (b_hi - b_lo) * (i - lo) as f64 / (hi - lo) as f64
            }
        };
        let at = (t * k + k / 2).min(len - 1);
        let height = self.y_d[t] - bridge(at.clamp(lo, hi));
        let snr = if self.noise > 0.0 {
            height / self.noise
        } else {
            f64::INFINITY
        };
        let wide = half_width(&self.y_d, t, self.base) as f64 * self.dt_d
            >= MIN_HALF_WIDTH * self.p.width_s.0;
        if !(wide && snr >= self.p.snr && height >= self.p.noise) {
            return None;
        }
        let (mut area, mut area_s, mut sw, mut swm) = (0.0, 0.0, 0.0, 0.0);
        for i in lo..=hi {
            let v = self.y_res[i] - bridge(i);
            sw += self.y[i];
            swm += self.y[i] * self.mz[i];
            if i > lo {
                let u = self.y_res[i - 1] - bridge(i - 1);
                area += (u + v) / 2.0;
                area_s += (self.rt_s[i] - self.rt_s[i - 1]) * (u + v) / 2.0;
            }
        }
        Some(BroadPeak {
            scan_lo: lo,
            scan_hi: hi,
            scan_apex: at,
            rt_apex: self.rt_d[t],
            rt_lo: self.rt_s[lo],
            rt_hi: self.rt_s[hi],
            mz: if sw > 0.0 { swm / sw } else { self.mz[at] },
            height,
            area,
            area_s,
            snr,
            scale_s,
        })
    }
}

#[cfg(test)]
mod tests;
