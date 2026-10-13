//! Quality of every peak, from its ion trace in the raw peak table: shape, signal over noise, width, symmetry, gaps and edge.
//!
//! Explainable measures, never a learned classifier. The trace of a feature is the sum of the peaks within `window_ppm` of its m/z in
//! every scan of the section, from `pad` scans before the first to `pad` scans after the last; the flanks (outside the edges of the
//! peak, without the points above half of the apex) give the baseline (median) and the noise (1.4826 MAD, not below a third of the intensity floor, so that a peak whose flanks
//! are empty is judged against the detection limit and not against zero). The correlation with the Gaussian is `peakq::quality`.
//! A peak is good when its shape, S/N and number of points reach the limits of `QParams`; a row of the aligned table stays when at
//! least one file has a good peak (`keep_row`) and the reasons of the files that did not are written as a code (`reason_text`).

use crate::sortc;
use mzlab_core::error::Result;
use mzlab_core::idxread::{Index, ReadAt};
use mzlab_core::peakq;

use crate::sweep::{ppm_ranges, windows};
use crate::table::{FileFeatures, FLAG_EDGE, FLAG_GOOD};
use crate::trace::{filled, Params};

/// Reason bits of `FileFeatures::qreason`.
pub const Q_SHAPE: u8 = 1;
pub const Q_SNR: u8 = 2;
pub const Q_POINTS: u8 = 4;
/// Half width, minutes, of the window around the apex that gives `FileFeatures::area_win`.
pub const AREA_HALF_WINDOW: f64 = 0.05;

#[derive(Debug, Clone)]
pub struct QParams {
    /// Window of m/z of the trace around the m/z of the feature, ppm.
    pub window_ppm: f64,
    /// Scans of flank on each side of the peak.
    pub pad: usize,
    pub min_shape: f64,
    pub min_snr: f64,
    pub min_points: u16,
}

impl Default for QParams {
    fn default() -> Self {
        QParams {
            window_ppm: 6.0,
            pad: 15,
            min_shape: 0.5,
            min_snr: 3.0,
            min_points: 5,
        }
    }
}

/// Median of a slice (sorted copy; even length = mean of the middle pair).
fn median(v: &mut [f64]) -> f64 {
    if v.is_empty() {
        return 0.0;
    }
    sortc::by(v, |a, b| a.total_cmp(b));
    let n = v.len();
    if n % 2 == 1 {
        v[n / 2]
    } else {
        (v[n / 2 - 1] + v[n / 2]) / 2.0
    }
}

/// (baseline, noise) from the flanks `y[..lo]` and `y[hi+1..]` without the points above half of the apex (the shoulder of a neighbouring
/// ion or the tail of the peak itself is signal, not noise); without three flank points the edges give the baseline.
fn flank_noise(y: &[f64], lo: usize, hi: usize, floor: f64) -> (f64, f64) {
    let top = y[lo..=hi].iter().fold(0.0f64, |m, &v| m.max(v));
    let mut fl: Vec<f64> = y[..lo]
        .iter()
        .chain(y[hi + 1..].iter())
        .copied()
        .filter(|&v| v < top / 2.0)
        .collect();
    let min_noise = floor / 3.0;
    if fl.len() < 3 {
        return (y[lo].min(y[hi]), min_noise);
    }
    let base = median(&mut fl);
    let mut dev: Vec<f64> = fl.iter().map(|v| (v - base).abs()).collect();
    (base, (1.4826 * median(&mut dev)).max(min_noise))
}

/// Full width at half maximum in scans of the baseline-corrected trace around the maximum `k` (a side that does not come down counts as the other).
fn fwhm_scans(c: &[f64], k: usize) -> f64 {
    let half = c[k] / 2.0;
    let cross = |dir: isize| -> Option<f64> {
        let mut i = k as isize;
        while i + dir >= 0 && ((i + dir) as usize) < c.len() {
            let (a, b) = (c[i as usize], c[(i + dir) as usize]);
            if b <= half {
                return Some((a - half) / (a - b).max(1e-300));
            }
            i += dir;
        }
        None
    };
    let walk = |dir: isize| -> f64 {
        let mut n = 0.0;
        let mut i = k as isize;
        while i + dir >= 0 && ((i + dir) as usize) < c.len() && c[(i + dir) as usize] > half {
            n += 1.0;
            i += dir;
        }
        n
    };
    match (cross(-1), cross(1)) {
        (Some(l), Some(r)) => walk(-1) + l + walk(1) + r,
        (Some(l), None) => 2.0 * (walk(-1) + l),
        (None, Some(r)) => 2.0 * (walk(1) + r),
        (None, None) => c.len() as f64,
    }
}

/// Measures of one trace `y` (scans `w0..`) whose peak spans `lo..=hi` (positions in `y`).
pub(crate) fn measure(
    rt: &[f64],
    y: &[f64],
    lo: usize,
    hi: usize,
    floor: f64,
) -> (f64, f64, f64, f64, u16) {
    let (base, noise) = flank_noise(y, lo, hi, floor);
    let q = peakq::quality(rt, y, lo, hi, noise);
    let mut k = lo;
    for i in lo..=hi {
        if y[i] > y[k] {
            k = i;
        }
    }
    let c: Vec<f64> = y[lo..=hi].iter().map(|v| v - base).collect();
    let fwhm = if c[k - lo] > 0.0 {
        fwhm_scans(&c, k - lo)
    } else {
        0.0
    };
    let gaps = y[lo..=hi]
        .iter()
        .filter(|&&v| v <= 0.0)
        .count()
        .min(u16::MAX as usize) as u16;
    (
        (y[k] - base) / noise,
        q.gaussian,
        fwhm,
        q.asymmetry.unwrap_or(0.0),
        gaps,
    )
}

/// First and last scan within `half` minutes of the scan `apex`.
pub(crate) fn around_apex(rt: &[f64], apex: usize, half: f64) -> (usize, usize) {
    let (mut a, mut b) = (apex, apex);
    while a > 0 && (rt[apex] - rt[a - 1]).abs() <= half {
        a -= 1;
    }
    while b + 1 < rt.len() && (rt[b + 1] - rt[apex]).abs() <= half {
        b += 1;
    }
    (a, b)
}

/// Quality columns of row `i` from its trace `y`, which starts at scan `w0`; `(w0, wa, wb)` with `wa..=wb` the scans of the area window.
/// `scan_lo`, `scan_hi`, `apex` and `npts` of the row are already set.
pub(crate) fn fill_row(
    f: &mut FileFeatures,
    i: usize,
    rt: &[f64],
    y: &[f64],
    win: (usize, usize, usize),
    floor: f64,
    q: &QParams,
) {
    let (w0, wa, wb) = win;
    let nscan = rt.len();
    let (slo, shi) = (
        f.scan_lo[i] as usize,
        (f.scan_hi[i] as usize).min(nscan - 1),
    );
    f.flags[i] &= !(FLAG_GOOD | FLAG_EDGE);
    f.area_win[i] = y[wa - w0..=wb - w0].iter().sum::<f64>() * f.dt;
    let (snr, shape, fwhm, asym, gaps) =
        measure(&rt[w0..w0 + y.len()], y, slo - w0, shi - w0, floor);
    f.snr[i] = snr as f32;
    f.shape[i] = shape as f32;
    f.fwhm[i] = fwhm as f32;
    f.asym[i] = asym as f32;
    f.gaps[i] = gaps;
    let mut why = 0u8;
    if shape < q.min_shape {
        why |= Q_SHAPE;
    }
    if snr < q.min_snr {
        why |= Q_SNR;
    }
    if f.npts[i] < q.min_points {
        why |= Q_POINTS;
    }
    f.qreason[i] = why;
    if why == 0 {
        f.flags[i] |= FLAG_GOOD;
    }
    if slo == 0 || shi == nscan - 1 {
        f.flags[i] |= FLAG_EDGE;
    }
}

/// Fill the quality columns of `f` (computed by `trace::detect` on the same section) from the peak table of `index`.
pub fn assess<R: ReadAt>(
    index: &mut Index<R>,
    f: &mut FileFeatures,
    p: &Params,
    q: &QParams,
) -> Result<()> {
    assess_with(index, f, p, q, |_, _, _, _, _| Ok(()))
}

/// `assess`, and after every row `hook(f, i, rt, y, w0)` sees the finished row and its trace `y` (scans `w0..`) while the peaks are in memory.
pub(crate) fn assess_with<R: ReadAt>(
    index: &mut Index<R>,
    f: &mut FileFeatures,
    p: &Params,
    q: &QParams,
    mut hook: impl FnMut(&FileFeatures, usize, &[f64], &[f64], usize) -> Result<()>,
) -> Result<()> {
    let Some(si) = index.section_of(1, p.polarity) else {
        return Ok(());
    };
    let rt: Vec<f64> = index.arrays(si)?.rt.clone();
    let nscan = rt.len();
    let max_peaks = (p.budget / 24).clamp(1024, crate::trace::SLAB_PEAKS);
    let mz = f.mz.clone();
    let ranges = ppm_ranges(&mz, q.window_ppm);
    windows(index, si, &ranges, max_peaks, |i, a, b, slab| {
        let (slo, shi) = (
            f.scan_lo[i] as usize,
            (f.scan_hi[i] as usize).min(nscan - 1),
        );
        let (wa, wb) = around_apex(&rt, (f.apex[i] as usize).min(nscan - 1), AREA_HALF_WINDOW);
        let w0 = slo.min(wa).saturating_sub(q.pad);
        let w1 = (shi.max(wb) + q.pad).min(nscan - 1);
        let mut y = filled(w1 - w0 + 1, 0.0f64)?;
        for j in a..b {
            let s = slab.pos[j] as usize;
            if s >= w0 && s <= w1 {
                y[s - w0] += slab.it[j];
            }
        }
        fill_row(f, i, &rt, &y, (w0, wa, wb), p.floor, q);
        hook(f, i, &rt, &y, w0)?;
        Ok(())
    })
}

/// The row of an aligned table stays when at least one file has a good peak (`None` = the file has no peak in the row).
pub fn keep_row(per_file: &[Option<u8>]) -> bool {
    per_file.contains(&Some(0))
}

/// Code of the reason why a row was dropped, e.g. `Q_SHAPE<0.5 in 13/13 files`: the most common reason among the files with a peak.
pub fn reason_text(per_file: &[Option<u8>], q: &QParams) -> String {
    let with: Vec<u8> = per_file.iter().flatten().copied().collect();
    if with.is_empty() {
        return "Q_NONE no peak in any file".to_string();
    }
    let names = [
        (Q_SHAPE, format!("Q_SHAPE<{}", q.min_shape)),
        (Q_SNR, format!("Q_SNR<{}", q.min_snr)),
        (Q_POINTS, format!("Q_POINTS<{}", q.min_points)),
    ];
    let mut best = (0usize, 0usize);
    for (k, (bit, _)) in names.iter().enumerate() {
        let n = with.iter().filter(|r| *r & bit != 0).count();
        if n > best.0 {
            best = (n, k);
        }
    }
    if best.0 == 0 {
        return format!("Q_OK in {0}/{0} files", with.len());
    }
    format!("{} in {}/{} files", names[best.1].1, best.0, with.len())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn gauss(n: usize, c: f64, s: f64, h: f64) -> Vec<f64> {
        (0..n)
            .map(|i| h * (-((i as f64 - c).powi(2)) / (2.0 * s * s)).exp())
            .collect()
    }

    #[test]
    fn bell_is_good() {
        let y = gauss(60, 30.0, 3.0, 1e6);
        let rt: Vec<f64> = (0..60).map(|i| i as f64 * 0.01).collect();
        let (snr, shape, fwhm, asym, gaps) = measure(&rt, &y, 20, 40, 2e4);
        assert!(shape > 0.98, "shape {shape}");
        assert!((fwhm - 2.3548 * 3.0).abs() < 0.3, "fwhm {fwhm}");
        assert!(snr > 100.0 && gaps == 0);
        assert!((asym - 1.0).abs() < 0.1, "asym {asym}");
    }

    #[test]
    fn two_spikes_have_gaps_and_no_bell_shape() {
        // fixed size: a 40-point trace of a test
        let mut y = vec![0.0; 40];
        for (i, v) in [(10, 9e5), (25, 8.8e5)] {
            y[i] = v;
        }
        let rt: Vec<f64> = (0..40).map(|i| i as f64 * 0.01).collect();
        let (_, shape, _, _, gaps) = measure(&rt, &y, 9, 26, 2e4);
        assert!(shape < 0.8, "shape {shape}");
        assert_eq!(gaps, 16);
    }

    #[test]
    fn rows_stay_with_one_good_file() {
        assert!(keep_row(&[Some(1), Some(0), None]));
        assert!(!keep_row(&[Some(1), Some(2), None]));
        let q = QParams::default();
        assert_eq!(
            reason_text(&[Some(1), Some(1), Some(3)], &q),
            "Q_SHAPE<0.5 in 3/3 files"
        );
        assert_eq!(
            reason_text(&[Some(2), Some(1), Some(2)], &q),
            "Q_SNR<3 in 2/3 files"
        );
    }
}
