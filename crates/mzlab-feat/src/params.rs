//! Estimate of the parameters of detection from the data of one file, each with its proof: the noise floor from the histogram of the intensities
//! (the valley between the noise and the signal), the mass tolerance from the m/z scatter of the strongest traces (3 robust sigma), the width of
//! a peak from their FWHM. The numbers are suggestions for the panel "Pipeline"; the proof (histogram, chosen point, counts) is stored with the recipe.

use mzlab_core::error::Result;
use mzlab_core::idxread::{Index, ReadAt};
use mzlab_core::json::Out;

use crate::sortc;
use crate::table::FileFeatures;
use crate::trace::{self, Params};

/// Width of a bin of the histograms, in decades.
pub const BIN: f64 = 0.1;
/// Lowest edge of the histograms: log10 of the intensity (1 count).
const LO: f64 = 0.0;
/// Number of bins (10 decades).
const BINS: usize = 100;
/// Traces used for the tolerance and the width: the strongest ones, at most.
const STRONG: usize = 80;
/// Tolerance bounds, ppm.
const PPM_MIN: f64 = 1.0;
const PPM_MAX: f64 = 20.0;

/// Histogram of log10 values in bins of `BIN` decades from `LO`.
#[derive(Debug, Clone, PartialEq)]
pub struct Hist {
    pub counts: Vec<u64>,
}

impl Hist {
    pub fn new() -> Hist {
        Hist {
            counts: vec![0; BINS],
        }
    }
    /// Counts `v` (> 0); values outside the range go in the first or last bin.
    pub fn add(&mut self, v: f64) {
        if v > 0.0 {
            let i = ((libm::log10(v) - LO) / BIN).floor();
            self.counts[(i.max(0.0) as usize).min(BINS - 1)] += 1;
        }
    }
    /// Intensity at the lower edge of bin `i`.
    pub fn edge(i: usize) -> f64 {
        libm::pow(10.0, LO + i as f64 * BIN)
    }
}

impl Default for Hist {
    fn default() -> Self {
        Hist::new()
    }
}

/// Noise floor and what it comes from.
#[derive(Debug, Clone, PartialEq)]
pub struct Noise {
    pub floor: f64,
    /// Intensity at the mode of the noise lump (the highest bin of the histogram).
    pub mode: f64,
    /// Width of the lump, in decades (a Gaussian in log10 of the intensity).
    pub sigma: f64,
}

/// Three-bin moving average (counts as floats).
fn smooth(c: &[u64]) -> Vec<f64> {
    (0..c.len())
        .map(|i| {
            let (a, b) = (i.saturating_sub(1), (i + 1).min(c.len() - 1));
            c[a..=b].iter().sum::<u64>() as f64 / (b - a + 1) as f64
        })
        .collect()
}

/// Sigmas above the mode of the noise lump where the floor is put (the level that 0.13 % of the noise peaks exceed).
pub const FLOOR_SIGMAS: f64 = 3.0;
/// A lump wider than this (decades) is not noise but a plateau: no floor is proposed.
const MAX_SIGMA: f64 = 1.0;
/// Variance (decades squared) that the bins and the three-bin average add to the lump.
const BLUR: f64 = BIN * BIN * 9.0 / 12.0;

/// The floor from the histogram of the intensities: the noise is the highest lump of the histogram (log10 of the intensity, about Gaussian), the signal the long
/// tail above it; the width of the lump is measured on its right side at half maximum (the left one may be cut by the threshold of the instrument) and the
/// floor is `FLOOR_SIGMAS` sigma above the mode. `None` if the histogram is empty or the lump never falls to half.
pub fn noise_from_hist(h: &Hist) -> Option<Noise> {
    let s = smooth(&h.counts);
    let mode = (0..BINS).max_by(|&a, &b| s[a].total_cmp(&s[b]).then(b.cmp(&a)))?;
    if s[mode] <= 0.0 {
        return None;
    }
    let half = s[mode] / 2.0;
    let j = (mode + 1..BINS).find(|&j| s[j] < half)?;
    let frac = (s[j - 1] - half) / (s[j - 1] - s[j]);
    // parabola through the logarithms of the three bins around the mode
    let off = if mode > 0 && mode + 1 < BINS && s[mode - 1] > 0.0 && s[mode + 1] > 0.0 {
        let (a, b, c) = (
            libm::log(s[mode - 1]),
            libm::log(s[mode]),
            libm::log(s[mode + 1]),
        );
        let d = a - 2.0 * b + c;
        if d < 0.0 {
            (0.5 * (a - c) / d).clamp(-0.5, 0.5)
        } else {
            0.0
        }
    } else {
        0.0
    };
    let hwhm = ((j - 1) as f64 + frac - (mode as f64 + off)) * BIN;
    // the lump is the Gaussian blurred by the bins and by the moving average: its variance is taken off
    let sigma = libm::sqrt((libm::pow(hwhm / 1.1774, 2.0) - BLUR).max(0.0));
    if sigma > MAX_SIGMA {
        return None;
    }
    let at = LO + (mode as f64 + 0.5 + off) * BIN;
    Some(Noise {
        floor: libm::pow(10.0, at + FLOOR_SIGMAS * sigma),
        mode: libm::pow(10.0, at),
        sigma,
    })
}

/// Median of a slice (sorted in place; `NaN` for an empty one).
fn median(v: &mut [f64]) -> f64 {
    if v.is_empty() {
        return f64::NAN;
    }
    sortc::by(v, f64::total_cmp);
    let n = v.len();
    if n % 2 == 1 {
        v[n / 2]
    } else {
        (v[n / 2 - 1] + v[n / 2]) / 2.0
    }
}

/// Mass tolerance: three robust sigma (1.4826 MAD) of the errors in ppm, within the bounds. Returns `(ppm, sigma)`.
pub fn tolerance_from_errors(errors: &mut [f64]) -> Option<(f64, f64)> {
    if errors.len() < 20 {
        return None;
    }
    let m = median(errors);
    let mut dev: Vec<f64> = errors.iter().map(|e| (e - m).abs()).collect();
    let sigma = 1.4826 * median(&mut dev);
    Some(((3.0 * sigma).clamp(PPM_MIN, PPM_MAX), sigma))
}

/// FWHM (same unit as `x`) of a trace by linear interpolation of the half-height crossings around `apex`; `None` if it does not come down to half.
pub fn fwhm_of_trace(x: &[f64], y: &[f64], apex: usize) -> Option<f64> {
    let half = y[apex] / 2.0;
    let mut l = apex;
    while l > 0 && y[l] > half {
        l -= 1;
    }
    let mut r = apex;
    while r + 1 < y.len() && y[r] > half {
        r += 1;
    }
    if y[l] > half || y[r] > half {
        return None;
    }
    let cross = |a: usize, b: usize| x[a] + (half - y[a]) / (y[b] - y[a]) * (x[b] - x[a]);
    Some(cross(r, r - 1) - cross(l, l + 1))
}

/// The proofs of the parameters measured on one file.
#[derive(Debug, Clone)]
pub struct Estimate {
    pub intensities: Hist,
    pub noise: Option<Noise>,
    pub ppm: Option<f64>,
    pub ppm_sigma: f64,
    pub ppm_n: usize,
    /// Median FWHM of the strong traces, minutes.
    pub fwhm: Option<f64>,
    pub fwhm_n: usize,
}

/// Peaks read for the histogram of the intensities at most (in pieces of `PIECE` peaks spread over the whole m/z range).
const SAMPLE_PEAKS: u64 = 4 << 20;
const PIECE: u64 = 1 << 16;

/// Histogram of the intensities of the peaks of the MS1 section; a file with more than `SAMPLE_PEAKS` peaks is read in pieces at even distances.
pub fn intensity_hist<R: ReadAt>(index: &mut Index<R>, si: usize) -> Result<Hist> {
    let mut h = Hist::new();
    let n = index.sections[si].n_peaks;
    if n <= SAMPLE_PEAKS {
        index.each_peak(si, 0, n, |_p, _mz, it| h.add(it))?;
        return Ok(h);
    }
    let pieces = SAMPLE_PEAKS / PIECE;
    for k in 0..pieces {
        let a = (n - PIECE) / (pieces - 1) * k;
        index.each_peak(si, a, a + PIECE, |_p, _mz, it| h.add(it))?;
    }
    Ok(h)
}

/// Indices of the `n` highest features.
pub fn strongest(f: &FileFeatures, n: usize) -> Vec<usize> {
    let mut ix: Vec<usize> = (0..f.len()).collect();
    sortc::by(&mut ix, |&a, &b| f.height[b].total_cmp(&f.height[a]));
    ix.truncate(n);
    ix
}

/// Errors in ppm of the peaks of a trace around its mean m/z, and the FWHM in minutes, for feature `i` (one pass over the peaks within 20 ppm).
fn measure<R: ReadAt>(
    index: &mut Index<R>,
    si: usize,
    f: &FileFeatures,
    rt: &[f64],
    i: usize,
    errors: &mut Vec<f64>,
) -> Result<Option<f64>> {
    let mz = f.mz[i];
    let w = 20e-6 * mz;
    let a = index.search(si, mz - w, false)?;
    let b = index.search(si, mz + w, true)?;
    let (lo, hi) = (f.scan_lo[i] as usize, f.scan_hi[i] as usize);
    let (from, to) = (lo.saturating_sub(5), (hi + 6).min(rt.len()));
    let apex = f.apex[i] as usize;
    if apex < from || apex >= to {
        return Ok(None);
    }
    let mut y = vec![0.0; to - from];
    let half = f.height[i] as f64 * 0.2;
    index.each_peak(si, a, b, |p, m, it| {
        if p >= from && p < to && it > y[p - from] {
            y[p - from] = it;
        }
        if p >= lo && p <= hi && it >= half {
            errors.push((m - mz) / mz * 1e6);
        }
    })?;
    Ok(fwhm_of_trace(&rt[from..to], &y, apex - from))
}

/// Everything the panel proposes, measured on `f` (the features of the file) and its index.
pub fn estimate<R: ReadAt>(index: &mut Index<R>, p: &Params, f: &FileFeatures) -> Result<Estimate> {
    let empty = Estimate {
        intensities: Hist::new(),
        noise: None,
        ppm: None,
        ppm_sigma: 0.0,
        ppm_n: 0,
        fwhm: None,
        fwhm_n: 0,
    };
    let Some(si) = index.section_of(1, p.polarity) else {
        return Ok(empty);
    };
    let intensities = intensity_hist(index, si)?;
    let noise = noise_from_hist(&intensities);
    let rt = index.arrays(si)?.rt.clone();
    let (mut errors, mut widths) = (Vec::new(), Vec::new());
    for i in strongest(f, STRONG) {
        if let Some(w) = measure(index, si, f, &rt, i, &mut errors)? {
            widths.push(w);
        }
    }
    let tol = tolerance_from_errors(&mut errors);
    Ok(Estimate {
        intensities,
        noise,
        ppm: tol.map(|t| t.0),
        ppm_sigma: tol.map_or(0.0, |t| t.1),
        ppm_n: errors.len(),
        fwhm: (widths.len() >= 5).then(|| median(&mut widths)),
        fwhm_n: widths.len(),
    })
}

/// One example trace for the preview: the XIC of a feature with its edges, the points in minutes and counts.
pub struct Example {
    pub mz: f64,
    pub rt: f64,
    pub left: f64,
    pub right: f64,
    pub height: f64,
    pub x: Vec<f64>,
    pub y: Vec<f64>,
}

/// The XIC of feature `i` (maximum intensity per scan within 5 ppm), a few minutes around its edges.
pub fn example<R: ReadAt>(
    index: &mut Index<R>,
    p: &Params,
    f: &FileFeatures,
    i: usize,
) -> Result<Option<Example>> {
    let Some(si) = index.section_of(1, p.polarity) else {
        return Ok(None);
    };
    let mz = f.mz[i];
    let a = index.search(si, mz * (1.0 - 5e-6), false)?;
    let b = index.search(si, mz * (1.0 + 5e-6), true)?;
    let y = index.max_window(si, a, b)?;
    let rt = index.arrays(si)?.rt.clone();
    let span = (f.scan_hi[i] - f.scan_lo[i]) as usize + 1;
    let (from, to) = (
        (f.scan_lo[i] as usize).saturating_sub(span),
        (f.scan_hi[i] as usize + span + 1).min(y.len()),
    );
    Ok(Some(Example {
        mz,
        rt: f.rt[i] as f64,
        left: f.left[i] as f64,
        right: f.right[i] as f64,
        height: f.height[i] as f64,
        x: rt[from..to].to_vec(),
        y: y[from..to].to_vec(),
    }))
}

/// Features at the quantiles 0.95, 0.5 and 0.1 of the height: a strong, a medium and a weak example (indices).
pub fn example_picks(f: &FileFeatures) -> Vec<usize> {
    let mut ix: Vec<usize> = (0..f.len()).collect();
    sortc::by(&mut ix, |&a, &b| f.height[b].total_cmp(&f.height[a]));
    if ix.is_empty() {
        return ix;
    }
    [0.05, 0.5, 0.9]
        .iter()
        .map(|q| ix[((ix.len() - 1) as f64 * q) as usize])
        .collect()
}

/// Histogram of the heights of the features (same bins as the intensities).
pub fn height_hist(f: &FileFeatures) -> Hist {
    let mut h = Hist::new();
    for &v in &f.height {
        h.add(v as f64);
    }
    h
}

fn hist_json(o: &mut Out, h: &Hist) {
    o.raw("{\"lo\":")
        .num(LO)
        .raw(",\"step\":")
        .num(BIN)
        .raw(",\"counts\":[");
    for (i, c) in h.counts.iter().enumerate() {
        o.raw(if i > 0 { "," } else { "" }).int(*c as i64);
    }
    o.raw("]}");
}

fn example_json(o: &mut Out, e: &Example) {
    o.raw("{\"mz\":").num(e.mz).raw(",\"rt\":").num(e.rt);
    o.raw(",\"left\":")
        .num(e.left)
        .raw(",\"right\":")
        .num(e.right);
    o.raw(",\"height\":")
        .num(e.height)
        .raw(",\"x\":")
        .rounded(&e.x, 4);
    o.raw(",\"y\":").rounded(&e.y, 0).raw("}");
}

/// The preview as JSON: `{n, heights, intensities, noise, ppm, fwhm, examples}`; every estimate carries the numbers it comes from.
pub fn preview_json(est: &Estimate, f: &FileFeatures, examples: &[Example]) -> String {
    let mut o = Out::default();
    o.raw("{\"n\":").int(f.len() as i64).raw(",\"heights\":");
    hist_json(&mut o, &height_hist(f));
    o.raw(",\"intensities\":");
    hist_json(&mut o, &est.intensities);
    o.raw(",\"noise\":");
    match &est.noise {
        Some(n) => {
            o.raw("{\"floor\":")
                .num(n.floor)
                .raw(",\"mode\":")
                .num(n.mode);
            o.raw(",\"sigma\":").num(n.sigma).raw("}");
        }
        None => {
            o.raw("null");
        }
    }
    o.raw(",\"ppm\":{\"value\":")
        .opt(est.ppm)
        .raw(",\"sigma\":")
        .num(est.ppm_sigma);
    o.raw(",\"n\":")
        .int(est.ppm_n as i64)
        .raw("},\"fwhm\":{\"value\":")
        .opt(est.fwhm);
    o.raw(",\"n\":")
        .int(est.fwhm_n as i64)
        .raw("},\"examples\":[");
    for (i, e) in examples.iter().enumerate() {
        o.raw(if i > 0 { "," } else { "" });
        example_json(&mut o, e);
    }
    o.raw("]}");
    o.0
}

/// Preview of one file: detection with `p`, the estimates and three example traces, as JSON.
pub fn preview<R: ReadAt>(index: &mut Index<R>, p: &Params) -> Result<String> {
    let f = trace::detect(index, p)?;
    let est = estimate(index, p, &f)?;
    let mut ex = Vec::new();
    for i in example_picks(&f) {
        ex.extend(example(index, p, &f, i)?);
    }
    Ok(preview_json(&est, &f, &ex))
}

#[cfg(test)]
mod tests {
    use super::*;
    use mzlab_core::rng::Pcg64;

    /// Uniform in (0, 1) from the generator.
    fn u(r: &mut Pcg64) -> f64 {
        (r.next64() >> 11) as f64 / (1u64 << 53) as f64 + 1e-12
    }

    /// Standard normal by Box-Muller.
    fn gauss(r: &mut Pcg64) -> f64 {
        libm::sqrt(-2.0 * libm::log(u(r))) * libm::cos(6.283185307179586 * u(r))
    }

    /// Noise: log-normal around `noise` (0.25 decade); signal: log-normal around 1e6 (0.6 decade), 2 % of the peaks.
    fn synthetic(noise: f64, seed: u32) -> Hist {
        let mut r = Pcg64::new(seed);
        let mut h = Hist::new();
        for _ in 0..200_000 {
            h.add(libm::pow(10.0, libm::log10(noise) + 0.25 * gauss(&mut r)));
        }
        for _ in 0..4_000 {
            h.add(libm::pow(10.0, 6.0 + 0.6 * gauss(&mut r)));
        }
        h
    }

    #[test]
    fn the_floor_is_within_30_percent_of_the_true_one() {
        // true floor: 3 sigma (0.25 decade) above the center of the noise
        for (noise, seed) in [(3e3, 1), (5e3, 2), (1e4, 3), (2e4, 4)] {
            let n = noise_from_hist(&synthetic(noise, seed)).unwrap();
            let want = noise * libm::pow(10.0, 3.0 * 0.25);
            assert!(
                (n.floor / want - 1.0).abs() <= 0.3,
                "noise {noise}: floor {} against {want} ({n:?})",
                n.floor
            );
            assert!((n.sigma - 0.25).abs() < 0.08, "sigma {}", n.sigma);
        }
    }

    #[test]
    fn a_histogram_without_a_lump_gives_no_floor() {
        assert!(noise_from_hist(&Hist::new()).is_none());
        let mut flat = Hist::new();
        for i in 10..90 {
            flat.counts[i] = 100;
        }
        assert!(noise_from_hist(&flat).is_none());
    }

    #[test]
    fn tolerance_is_three_robust_sigma_inside_the_bounds() {
        let mut r = Pcg64::new(7);
        let mut e: Vec<f64> = (0..500).map(|_| 0.8 * gauss(&mut r)).collect();
        e.extend([50.0, -40.0, 90.0]); // outliers must not move it
        let (ppm, sigma) = tolerance_from_errors(&mut e).unwrap();
        assert!(
            (sigma - 0.8).abs() < 0.15 && (ppm - 2.4).abs() < 0.5,
            "{ppm} {sigma}"
        );
        assert!(tolerance_from_errors(&mut [1.0, 2.0]).is_none());
        let mut wide: Vec<f64> = (0..100).map(|i| (i as f64 - 50.0) * 2.0).collect();
        assert_eq!(tolerance_from_errors(&mut wide).unwrap().0, PPM_MAX);
    }

    #[test]
    fn fwhm_of_a_gaussian_trace() {
        let x: Vec<f64> = (0..200).map(|i| i as f64 * 0.01).collect();
        let s = 0.1;
        let y: Vec<f64> = x
            .iter()
            .map(|t| 1e6 * libm::exp(-0.5 * ((t - 1.0) / s) * ((t - 1.0) / s)))
            .collect();
        let w = fwhm_of_trace(&x, &y, 100).unwrap();
        assert!((w - 2.3548 * s).abs() < 0.01, "{w}");
        assert!(fwhm_of_trace(&x, &vec![1e6; 200], 100).is_none());
    }
}
