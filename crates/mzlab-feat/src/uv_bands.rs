//! Peaks of one UV/PDA trace: the narrow peaks of `peaks::detect_peaks` (unchanged) plus, when asked, the broad bands (FWHM up to ~240 s) that its
//! CWT scales and its AsLS baseline lose. The bands come from `broad::detect_broad` run on the single trace. Used by the step `chrom.uv_peaks`.
//! Pure functions, fixed order of sums.

use mzlab_core::error::Result;
use mzlab_core::peaks::{detect_peaks, Params, Peak};

use crate::broad::{detect_broad, inpaint_linear, is_broad_ridge, BroadParams, BroadPeak};

/// Seconds per block of the decimated residue that `detect_broad` searches.
const BLOCK_S: f64 = 3.0;

/// What to look for. `broad` off = exactly the narrow peaks of `detect_peaks`.
#[derive(Clone, Debug, PartialEq)]
pub struct UvParams {
    pub broad: bool,
    /// Range of the full width at half maximum of the bands, seconds.
    pub min_s: f64,
    pub max_s: f64,
    /// Minimum height of a band over its bridge, in noise units.
    pub snr: f64,
}

impl Default for UvParams {
    fn default() -> Self {
        UvParams {
            broad: false,
            min_s: 30.0,
            max_s: 300.0,
            snr: 8.0,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum UvKind {
    Narrow,
    Broad,
}

/// One peak: RT in minutes, width in seconds, `area` in signal x seconds over its linear bridge.
#[derive(Clone, Debug, PartialEq)]
pub struct UvPeak {
    pub kind: UvKind,
    pub rt_apex: f64,
    pub rt_lo: f64,
    pub rt_hi: f64,
    pub fwhm_s: f64,
    pub height: f64,
    pub area: f64,
    pub snr: f64,
}

/// Peaks of the trace `y` at `rt_min` (minutes), in order of RT.
pub fn uv_peaks(rt_min: &[f64], y: &[f64], p: &UvParams) -> Result<Vec<UvPeak>> {
    if rt_min.len() != y.len() || y.len() < 2 {
        return Ok(Vec::new());
    }
    let narrow = detect_peaks(rt_min, y, Params::default());
    let rt_s: Vec<f64> = rt_min.iter().map(|t| t * 60.0).collect();
    let mut out: Vec<UvPeak> = narrow
        .iter()
        .map(|k| narrow_peak(rt_min, &rt_s, y, k))
        .collect();
    if p.broad {
        let mut d: Vec<f64> = rt_s.windows(2).map(|w| w[1] - w[0]).collect();
        d.sort_by(f64::total_cmp);
        let dt = d[d.len() / 2];
        if dt.is_nan() || dt <= 0.0 {
            return Ok(out);
        }
        let k = ((BLOCK_S / dt).round() as usize).clamp(1, 64);
        let bp = BroadParams {
            enabled: true,
            noise: 0.0,
            width_s: (p.min_s, p.max_s),
            snr: p.snr,
            decimate: k,
            ..BroadParams::default()
        };
        let ranges = merged(narrow.iter().map(|k| (k.lo, k.hi)).collect(), y.len());
        let zeros = vec![0.0; y.len()];
        // the detector reads a drift as a band, and its edge walk needs the baseline near zero
        let flat = remove_line(&rt_s, y, k);
        let found = detect_broad(&rt_s, &zeros, &flat, &ranges, &bp)?;
        let sm = smooth_without_narrow(&flat, &ranges, k, dt)?;
        out.extend(measure_bands(&rt_s, &sm, &found));
    }
    out.sort_by(|a, b| a.rt_apex.total_cmp(&b.rt_apex));
    Ok(out)
}

/// The scan ranges with those that overlap or touch joined: `detect_peaks` splits one peak into several nested ranges on a fast run, and a part of a
/// peak left in the trace would pass for a band.
fn merged(r: Vec<(usize, usize)>, len: usize) -> Vec<(usize, usize)> {
    let mut r: Vec<(usize, usize)> = r
        .into_iter()
        .map(|(lo, hi)| {
            let m = (hi - lo) / 2 + 1;
            (lo.saturating_sub(m), (hi + m).min(len - 1))
        })
        .collect();
    r.sort_unstable();
    let mut out: Vec<(usize, usize)> = Vec::new();
    for (lo, hi) in r {
        match out.last_mut() {
            Some(last) if lo <= last.1 + 1 => last.1 = last.1.max(hi),
            _ => out.push((lo, hi)),
        }
    }
    out
}

/// `y` minus the straight line that follows its lower envelope: an asymmetric least-squares fit (weight 0.05 above the line, 0.95 below) on blocks
/// of `k` samples, 8 passes. A line that cannot be fitted (fewer than 3 blocks) leaves `y` as it is.
fn remove_line(rt_s: &[f64], y: &[f64], k: usize) -> Vec<f64> {
    let n = y.len().div_ceil(k);
    let blocks: Vec<(f64, f64)> = (0..n)
        .map(|b| {
            let (a, e) = (b * k, ((b + 1) * k).min(y.len()));
            let c = (e - a) as f64;
            (
                rt_s[a..e].iter().sum::<f64>() / c,
                y[a..e].iter().sum::<f64>() / c,
            )
        })
        .collect();
    if n < 3 {
        return y.to_vec();
    }
    let t0 = blocks[0].0;
    let mut w = vec![1.0; n];
    let (mut a, mut b) = (0.0, 0.0);
    for _ in 0..8 {
        let (mut sw, mut st, mut stt, mut sy, mut sty) = (0.0, 0.0, 0.0, 0.0, 0.0);
        for (&(t, v), &wi) in blocks.iter().zip(&w) {
            let t = t - t0;
            sw += wi;
            st += wi * t;
            stt += wi * t * t;
            sy += wi * v;
            sty += wi * t * v;
        }
        let det = sw * stt - st * st;
        if det.abs() < 1e-12 {
            return y.to_vec();
        }
        a = (sy * stt - st * sty) / det;
        b = (sw * sty - st * sy) / det;
        for (wi, &(t, v)) in w.iter_mut().zip(&blocks) {
            *wi = if v > a + b * (t - t0) { 0.05 } else { 0.95 };
        }
    }
    rt_s.iter()
        .zip(y)
        .map(|(&t, &v)| v - (a + b * (t - t0)))
        .collect()
}

fn narrow_peak(rt_min: &[f64], rt_s: &[f64], y: &[f64], k: &Peak) -> UvPeak {
    let (lo, hi) = (k.lo, k.hi);
    UvPeak {
        kind: UvKind::Narrow,
        rt_apex: rt_min[k.apex_i],
        rt_lo: rt_min[lo],
        rt_hi: rt_min[hi],
        fwhm_s: fwhm(rt_s, y, k.apex_i, lo, hi),
        height: k.height,
        area: k.area * 60.0,
        snr: k.snr,
    }
}

/// `y` without the narrow peaks that are not the crest of a band (linear inpainting), smoothed over one block.
fn smooth_without_narrow(
    y: &[f64],
    narrow: &[(usize, usize)],
    k: usize,
    dt: f64,
) -> Result<Vec<f64>> {
    let mut v = y.to_vec();
    let mut cut = Vec::new();
    for &(s, e) in narrow {
        if !is_broad_ridge(y, s, e, dt)? {
            cut.push((s, e));
        }
    }
    inpaint_linear(&mut v, &cut);
    Ok(boxcar(&v, k))
}

/// Apex, width and height of every band on the smooth trace `sm`, over the line between the edges of the detector; the area is the detector's.
fn measure_bands(rt_s: &[f64], sm: &[f64], found: &[BroadPeak]) -> Vec<UvPeak> {
    found
        .iter()
        .map(|b| {
            let (lo, hi) = (b.scan_lo, b.scan_hi);
            let bridge = |i: usize| bridge_at(rt_s, sm, lo, hi, i);
            let apex = (lo..=hi)
                .max_by(|&a, &c| (sm[a] - bridge(a)).total_cmp(&(sm[c] - bridge(c))))
                .unwrap_or(b.scan_apex);
            UvPeak {
                kind: UvKind::Broad,
                rt_apex: rt_s[apex] / 60.0,
                rt_lo: rt_s[lo] / 60.0,
                rt_hi: rt_s[hi] / 60.0,
                fwhm_s: fwhm(rt_s, sm, apex, lo, hi),
                height: sm[apex] - bridge(apex),
                area: b.area_s,
                snr: b.snr,
            }
        })
        .collect()
}

/// Moving mean over `k` samples centred on each one (shorter at the ends).
fn boxcar(y: &[f64], k: usize) -> Vec<f64> {
    let h = k / 2;
    (0..y.len())
        .map(|i| {
            let (a, e) = (i.saturating_sub(h), (i + h + 1).min(y.len()));
            y[a..e].iter().sum::<f64>() / (e - a) as f64
        })
        .collect()
}

/// The straight line between `y[lo]` and `y[hi]`, at sample `i`.
fn bridge_at(rt_s: &[f64], y: &[f64], lo: usize, hi: usize, i: usize) -> f64 {
    if hi <= lo {
        return y[lo];
    }
    y[lo] + (y[hi] - y[lo]) * (rt_s[i] - rt_s[lo]) / (rt_s[hi] - rt_s[lo])
}

/// Full width (seconds) at half the height of `y[apex]` over the bridge `lo..hi`, linear interpolation; an edge of the region stands in for a
/// crossing that does not happen inside it.
fn fwhm(rt_s: &[f64], y: &[f64], apex: usize, lo: usize, hi: usize) -> f64 {
    let net = |i: usize| y[i] - bridge_at(rt_s, y, lo, hi, i);
    let half = net(apex) / 2.0;
    let cross = |a: usize, b: usize| {
        let (na, nb) = (net(a) - half, net(b) - half);
        if na == nb {
            return rt_s[a];
        }
        rt_s[a] + (rt_s[b] - rt_s[a]) * na / (na - nb)
    };
    let mut l = apex;
    while l > lo && net(l) > half {
        l -= 1;
    }
    let mut r = apex;
    while r < hi && net(r) > half {
        r += 1;
    }
    let t_l = if net(l) > half {
        rt_s[l]
    } else {
        cross(l, l + 1)
    };
    let t_r = if net(r) > half {
        rt_s[r]
    } else {
        cross(r, r - 1)
    };
    (t_r - t_l).max(0.0)
}
