//! CentWave peak model for the ion traces of `trace` (`features.detect`, algorithm `centwave`): each region of interest (a stitched trace) is
//! rebuilt as an intensity series per scan and cut into peaks with a continuous wavelet transform (Ricker wavelet) over the expected widths.
//! Sums run in a fixed order and `exp` comes from `libm`: native and WASM give the same bits.

use mzlab_core::error::Result;

use crate::broad::{detect_broad_at, BroadParams};
use crate::channels::Seg;
use crate::trace::{filled, reserve, Params};

/// How the segments of a slab become peaks: the cut of the runs at their gaps (`masstrace`) or the wavelet model.
#[derive(Debug, Clone, PartialEq, Default)]
pub enum Peak {
    #[default]
    Minima,
    CentWave(CentWave),
}

/// Parameters of the wavelet model (descriptor `features.detect`, algorithm `centwave`).
#[derive(Clone, PartialEq)]
pub struct CentWave {
    /// Half window of m/z (ppm; on the Da axis 0.5 Da) around a trace where its intensities are summed per scan.
    pub ppm: f64,
    /// Range of the full width at half maximum of the peaks, seconds.
    pub width_s: (f64, f64),
    /// Minimum height over the baseline, in units of the noise of the trace (never less than `Params::floor`).
    pub snr: f64,
    /// Broad bands (30-300 s) on the residue of the narrow peaks; off by default.
    pub broad: BroadParams,
}

impl Default for CentWave {
    fn default() -> Self {
        CentWave {
            ppm: 25.0,
            width_s: (5.0, 30.0),
            snr: 10.0,
            broad: BroadParams::default(),
        }
    }
}

// The text of the recipe is built from this: the broad pass adds nothing while it is off, so the fingerprints made before it exist keep their name.
impl std::fmt::Debug for CentWave {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut d = f.debug_struct("CentWave");
        d.field("ppm", &self.ppm)
            .field("width_s", &self.width_s)
            .field("snr", &self.snr);
        if self.broad.enabled {
            d.field("broad", &self.broad);
        }
        d.finish()
    }
}

impl CentWave {
    /// Sets the range of the full width at half maximum of the peaks, seconds.
    pub fn with_width_s(mut self, width_s: (f64, f64)) -> Self {
        self.width_s = width_s;
        self
    }
}

/// Number of scales between the narrowest and the widest peak.
const SCALES: usize = 6;
/// FWHM of a Gaussian over its standard deviation.
const FWHM: f64 = 2.354_820_045_030_949_3;
/// Half width of a peak, in standard deviations of the scale that found it.
const REACH: f64 = 2.5;
/// Two neighbouring apexes are two peaks only if the valley between them is lower than this fraction of the lower apex.
const RESOLVED: f64 = 0.9;

pub(crate) fn ricker(x: f64) -> f64 {
    let x2 = x * x;
    (1.0 - x2) * libm::exp(-0.5 * x2)
}

/// Standard deviations (scans) of the wavelets: geometric between the narrowest and the widest peak.
fn scales(width_s: (f64, f64), dt: f64) -> Vec<f64> {
    let lo = (width_s.0 / dt / FWHM).max(0.8);
    let hi = (width_s.1 / dt / FWHM).max(lo);
    if hi <= lo * 1.01 {
        return vec![lo];
    }
    let k = libm::pow(hi / lo, 1.0 / (SCALES - 1) as f64);
    (0..SCALES).map(|i| lo * libm::pow(k, i as f64)).collect()
}

/// The Ricker wavelet of one scale: standard deviation `s` (scans), samples over `-half..=half` and the factor that normalises the response.
struct Wavelet {
    s: f64,
    half: isize,
    kernel: Vec<f64>,
    norm: f64,
}

impl Wavelet {
    fn new(s: f64) -> Wavelet {
        let half = libm::ceil(4.0 * s) as isize;
        Wavelet {
            s,
            half,
            kernel: (-half..=half).map(|d| ricker(d as f64 / s)).collect(),
            norm: 1.0 / libm::sqrt(s),
        }
    }
}

/// Responses of the Ricker wavelet `w` over `y`.
fn response(y: &[f64], w: &Wavelet) -> Result<Vec<f64>> {
    let n = y.len();
    let (half, kernel, norm) = (w.half, &w.kernel, w.norm);
    let mut r = filled(n, 0.0f64)?;
    for (t, out) in r.iter_mut().enumerate() {
        // the taps that fall inside the series, in the order of the kernel
        let (i0, i1) = (
            (t as isize - half).max(0) as usize,
            (t + half as usize + 1).min(n),
        );
        let j0 = (i0 as isize - t as isize + half) as usize;
        let mut acc = 0.0;
        for (v, w) in y[i0..i1].iter().zip(&kernel[j0..]) {
            acc += v * w;
        }
        *out = acc * norm;
    }
    Ok(r)
}

/// Apexes (index, scale) of the series: the local maxima of the response at each scale, narrowest scale first; a wider scale adds only what the narrower
/// ones did not find nearby. The apex is the highest point of `y` within one standard deviation of the maximum.
fn apexes(y: &[f64], wavelets: &[Wavelet]) -> Result<Vec<(usize, f64)>> {
    let n = y.len();
    let mut found: Vec<(usize, f64)> = Vec::new();
    for w in wavelets {
        let s = w.s;
        let r = response(y, w)?;
        let reach = libm::ceil(s) as usize;
        for t in 0..n {
            let left = t == 0 || r[t] >= r[t - 1];
            let right = t + 1 == n || r[t] > r[t + 1];
            if r[t] > 0.0 && left && right {
                let (a, z) = (t.saturating_sub(reach), (t + reach).min(n - 1));
                let mut apex = a;
                for i in a..=z {
                    if y[i] > y[apex] {
                        apex = i;
                    }
                }
                if !found.iter().any(|&(f, _)| f.abs_diff(apex) <= reach) {
                    found.push((apex, s));
                }
            }
        }
    }
    found.sort_by_key(|f| f.0);
    Ok(found)
}

/// Joins the neighbours that no valley of at least `1 - RESOLVED` of the lower apex separates: the higher apex stays, with the wider scale.
fn resolve(y: &[f64], found: Vec<(usize, f64)>) -> Vec<(usize, f64)> {
    let mut out: Vec<(usize, f64)> = Vec::new();
    for f in found {
        match out.last_mut() {
            Some(last) if y[valley(y, last.0, f.0)] > RESOLVED * y[last.0].min(y[f.0]) => {
                if y[f.0] > y[last.0] {
                    last.0 = f.0;
                }
                last.1 = last.1.max(f.1);
            }
            _ => out.push(f),
        }
    }
    out
}

/// Peaks of the series `y` as (first, apex, last) indexes, ordered by apex; adjacent peaks meet at the lowest point between their apexes.
fn peaks(
    y: &[f64],
    wavelets: &[Wavelet],
    snr: f64,
    floor: f64,
) -> Result<Vec<(usize, usize, usize)>> {
    let n = y.len();
    let found = resolve(y, apexes(y, wavelets)?);
    let mut out: Vec<(usize, usize, usize)> = Vec::new();
    for (k, &(apex, s)) in found.iter().enumerate() {
        let reach = libm::ceil(REACH * s) as usize;
        let mut lo = apex.saturating_sub(reach);
        let mut hi = (apex + reach).min(n - 1);
        if k > 0 {
            lo = lo.max(valley(y, found[k - 1].0, apex));
        }
        if let Some(next) = found.get(k + 1) {
            hi = hi.min(valley(y, apex, next.0));
        }
        out.push((lo, apex, hi));
    }
    let skirt = wavelets.iter().fold(0.0f64, |m, w| m.max(w.s));
    Ok(keep_above_noise(
        y,
        out,
        snr,
        floor,
        libm::ceil(REACH * skirt) as usize,
    ))
}

/// Index of the lowest point of `y` between two apexes (the first one if several).
fn valley(y: &[f64], a: usize, b: usize) -> usize {
    let mut at = a;
    for i in a..=b {
        if y[i] < y[at] {
            at = i;
        }
    }
    at
}

/// The peaks whose height over the baseline is `snr` times the noise (at least `floor`). The noise is the spread of the points farther than `skirt` scans
/// from every apex: the flanks of a peak found by a narrow wavelet are not noise, and a short series with no such point has the noise of `floor`.
fn keep_above_noise(
    y: &[f64],
    found: Vec<(usize, usize, usize)>,
    snr: f64,
    floor: f64,
    skirt: usize,
) -> Vec<(usize, usize, usize)> {
    let (mut sum, mut sum2, mut m) = (0.0, 0.0, 0.0);
    for (i, &v) in y.iter().enumerate() {
        if !found
            .iter()
            .any(|&(lo, apex, hi)| (i >= lo && i <= hi) || i.abs_diff(apex) <= skirt)
        {
            sum += v;
            sum2 += v * v;
            m += 1.0;
        }
    }
    let sd = if m >= 3.0 {
        libm::sqrt(((sum2 - sum * sum / m) / (m - 1.0)).max(0.0))
    } else {
        0.0
    };
    let noise = sd.max(floor);
    found
        .into_iter()
        .filter(|&(lo, apex, hi)| (y[apex] - y[lo].min(y[hi])) >= snr * noise)
        .collect()
}

/// The series of `s`: the intensities of the slab inside the window of m/z of the segment summed per scan, and the same weighted by m/z.
pub(crate) fn series_of(
    s: &Seg,
    mz: &[f64],
    it: &[f64],
    pos: &[u32],
    p: &Params,
    cw: &CentWave,
) -> Result<(Vec<f64>, Vec<f64>)> {
    let half = match p.axis {
        crate::axis::MassAxis::Ppm => s.mzc * cw.ppm * 1e-6,
        crate::axis::MassAxis::Da { .. } => 0.5,
    };
    let from = mz.partition_point(|&m| m < s.mzc - half);
    let to = mz.partition_point(|&m| m <= s.mzc + half);
    let len = (s.z - s.a + 1) as usize;
    let mut y = filled(len, 0.0f64)?;
    let mut weighted = filled(len, 0.0f64)?;
    for i in from..to {
        if pos[i] >= s.a && pos[i] <= s.z {
            let k = (pos[i] - s.a) as usize;
            y[k] += it[i];
            weighted[k] += it[i] * mz[i];
        }
    }
    Ok((y, weighted))
}

/// Cuts every segment into the peaks of its series: the intensities of the slab inside the window of m/z of the segment, summed per scan.
/// `mz` ascending; `dt` is the median interval between scans, seconds.
pub(crate) fn refine(
    segs: Vec<Seg>,
    mz: &[f64],
    it: &[f64],
    pos: &[u32],
    dt: f64,
    p: &Params,
    cw: &CentWave,
) -> Result<Vec<Seg>> {
    let wavelets: Vec<Wavelet> = scales(cw.width_s, dt)
        .into_iter()
        .map(Wavelet::new)
        .collect();
    let mut out: Vec<Seg> = Vec::new();
    for s in segs {
        let (y, weighted) = series_of(&s, mz, it, pos, p, cw)?;
        let len = y.len();
        // no peak can pass the height of the row or the signal-to-noise bar: the series is not worth the transform
        let top = y.iter().fold(0.0f64, |m, &v| m.max(v));
        if top < p.min_height.max(cw.snr * p.floor) {
            continue;
        }
        let found = peaks(&y, &wavelets, cw.snr, p.floor)?;
        if cw.broad.enabled && len >= cw.broad.min_scans {
            broad_segs(&s, &y, &weighted, &found, dt, &cw.broad, &mut out)?;
        }
        for (lo, apex, hi) in found {
            let tot: f64 = y[lo..=hi].iter().sum();
            let w: f64 = weighted[lo..=hi].iter().sum();
            reserve(&mut out, 1)?;
            out.push(Seg {
                chan: s.chan,
                mzc: w / tot,
                apex: s.a + apex as u32,
                n: y[lo..=hi].iter().filter(|&&v| v > 0.0).count() as u32,
                mx: y[apex],
                tot,
                a: s.a + lo as u32,
                z: s.a + hi as u32,
            });
        }
    }
    Ok(out)
}

/// Appends the broad bands of the series of `s` (the narrow peaks `found` are inpainted away first) as segments.
fn broad_segs(
    s: &Seg,
    y: &[f64],
    weighted: &[f64],
    found: &[(usize, usize, usize)],
    dt: f64,
    bp: &BroadParams,
    out: &mut Vec<Seg>,
) -> Result<()> {
    let len = y.len();
    let mut rt = filled(len, 0.0f64)?;
    let mut mz = filled(len, s.mzc)?;
    for i in 0..len {
        rt[i] = i as f64 * dt;
        if y[i] > 0.0 {
            mz[i] = weighted[i] / y[i];
        }
    }
    let narrow: Vec<(usize, usize)> = found.iter().map(|&(lo, _, hi)| (lo, hi)).collect();
    for b in detect_broad_at(&rt, &mz, y, &narrow, bp, s.a as usize)? {
        reserve(out, 1)?;
        out.push(Seg {
            chan: s.chan,
            mzc: b.mz,
            apex: s.a + b.scan_apex as u32,
            n: y[b.scan_lo..=b.scan_hi]
                .iter()
                .filter(|&&v| v > 0.0)
                .count() as u32,
            mx: b.height,
            tot: b.area,
            a: s.a + b.scan_lo as u32,
            z: s.a + b.scan_hi as u32,
        });
    }
    Ok(())
}
