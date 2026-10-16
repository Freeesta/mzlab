//! Chromatographic evidence of the cells of a low-resolution Dig. A cell the pipeline marked as detected counts as a peak only when the XIC of its
//! file (m/z of the row ± the tolerance) has a real chromatographic peak at its apex (`peak1d::find_peak`): S/N over the baseline of the far points,
//! FWHM, scans above 5 % of the height, apex after the dead volume and edges inside the run. A cell that fails keeps its area but becomes
//! `CELL_FILLED` (not detected), so a baseline hump, a single spike or the unretained front never makes a product, its persistence or its growth.

use mzlab_core::idxread::{Index, ReadAt};

use super::detail::xic;
use super::labels::{crit, Crit};
use super::peak1d::{find_peak, smooth};
use super::plan::Settings;
use crate::aligned::{Aligned, CELL_DETECTED, CELL_FILLED};
use crate::legacy::Res;

pub const PEAK: &str = "chromatographic peak";

/// What the XIC says of one detected cell.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PeakQ {
    pub snr: f64,
    /// Full width at half maximum, seconds.
    pub fwhm_s: f64,
    pub points: usize,
    pub apex_rt: f64,
    /// `None` when the cell is a peak, else the first limit it misses.
    pub why: Option<&'static str>,
}

impl PeakQ {
    pub fn ok(&self) -> bool {
        self.why.is_none()
    }
}

/// The limits of `s` (`min_snr`, `max_fwhm_s`, `min_points`, `dead_factor`, `rt_min`) on the peak of the trace `(rt, y)` nearest to `apex`.
pub fn judge(rt: &[f64], y: &[f64], apex: f64, s: &Settings) -> PeakQ {
    let Some(f) = find_peak(rt, y, Some(apex), s.rt_tol_min.max(0.1), 3, s.min_points) else {
        return PeakQ {
            snr: 0.0,
            fwhm_s: 0.0,
            points: 0,
            apex_rt: apex,
            why: Some("no signal in the XIC"),
        };
    };
    // S/N, width and edges on the smoothed trace with a noise of its own: the one of `find_peak` counts another peak of the XIC as noise
    let ys = smooth(y, 3);
    let snr = f.height / noise(rt, &ys, f.apex_rt, s.rt_tol_min.max(0.3));
    let ia = rt.iter().position(|&t| t >= f.apex_rt).unwrap_or(0);
    let half = ys[ia] - f.height / 2.0;
    let (mut a, mut b) = (ia, ia);
    while a > 0 && ys[a - 1] >= half {
        a -= 1;
    }
    while b + 1 < ys.len() && ys[b + 1] >= half {
        b += 1;
    }
    let fwhm_s = (rt[b] - rt[a]) * 60.0;
    let why = if f.apex_rt < s.dead_factor * s.rt_min {
        Some("apex in the dead volume")
    } else if snr < s.min_snr {
        Some("S/N too low")
    } else if f.points < s.min_points || b == a {
        Some("too few scans")
    } else if fwhm_s > s.max_fwhm_s {
        Some("too broad for a chromatographic peak")
    } else if a == 0 || b + 1 == ys.len() {
        Some("cut by the start or the end of the run")
    } else {
        None
    };
    PeakQ {
        snr,
        fwhm_s,
        points: f.points,
        apex_rt: f.apex_rt,
        why,
    }
}

/// Noise of the smoothed trace `ys` away from `apex` (farther than `far` min): 1.4826 MAD, or, where most points are at the baseline (a sparse trace
/// of single noise ions, MAD 0), the 90th percentile of the deviations, so another peak of the same XIC never counts as noise.
fn noise(rt: &[f64], ys: &[f64], apex: f64, far: f64) -> f64 {
    let mut v: Vec<f64> = (0..rt.len())
        .filter(|&i| (rt[i] - apex).abs() > far)
        .map(|i| ys[i])
        .collect();
    if v.len() < 10 {
        v = ys.to_vec();
    }
    let q = |v: &mut Vec<f64>, p: f64| {
        mzlab_core::sortc::by(v, |a, b| a.total_cmp(b));
        v.get(((v.len() - 1) as f64 * p).round() as usize)
            .copied()
            .unwrap_or(0.0)
    };
    let base = q(&mut v, 0.5);
    let mut d: Vec<f64> = v.iter().map(|x| (x - base).abs()).collect();
    let mad = 1.4826 * q(&mut d, 0.5);
    let n = if mad > 0.0 { mad } else { q(&mut d, 0.9) };
    if n > 0.0 {
        n
    } else {
        1.0
    }
}

/// The checks of the cells of the table (`row * n_files + file`), `None` where the cell was not detected or not checked.
#[derive(Debug, Clone, Default)]
pub struct Checks {
    pub q: Vec<Option<PeakQ>>,
    done: Vec<bool>,
}

impl Checks {
    pub fn new(t: &Aligned) -> Checks {
        Checks {
            q: vec![None; t.len() * t.n_files],
            done: vec![false; t.len()],
        }
    }

    pub fn of(&self, t: &Aligned, r: usize, k: usize) -> Option<PeakQ> {
        self.q.get(t.cell(r, k)).copied().flatten()
    }
}

/// Checks the detected cells of `rows` (each row once) and turns the ones that are not peaks into `CELL_FILLED`.
pub fn gate<R: ReadAt>(
    t: &mut Aligned,
    c: &mut Checks,
    rows: &[usize],
    s: &Settings,
    open: &mut dyn FnMut(usize) -> Res<Index<R>>,
) -> Res<()> {
    let mut ix: Vec<Option<Index<R>>> = (0..t.n_files).map(|_| None).collect();
    for &r in rows {
        if c.done[r] {
            continue;
        }
        c.done[r] = true;
        for (k, slot) in ix.iter_mut().enumerate() {
            let cell = t.cell(r, k);
            if t.state[cell] != CELL_DETECTED || !t.apex_rt[cell].is_finite() {
                continue;
            }
            if slot.is_none() {
                *slot = Some(open(k)?);
            }
            let Some(i) = slot.as_mut() else { continue };
            let (rt, y) = xic(i, t.mz[r] - s.tol_da, t.mz[r] + s.tol_da, 0.0)?;
            let q = judge(&rt, &y, f64::from(t.apex_rt[cell]), s);
            if !q.ok() {
                t.state[cell] = CELL_FILLED;
            }
            c.q[cell] = Some(q);
        }
    }
    Ok(())
}

/// The limits in words.
pub fn limits(s: &Settings) -> String {
    format!(
        "S/N ≥ {}, FWHM ≤ {} s, ≥ {} scans, apex after {:.2} min",
        super::labels::g(s.min_snr),
        super::labels::g(s.max_fwhm_s),
        s.min_points,
        s.dead_factor * s.rt_min
    )
}

/// The criterion of a row: pass when at least one cell is a chromatographic peak, with the best one (or the best rejected one) in words.
pub fn criterion(
    t: &Aligned,
    c: &Checks,
    r: Option<usize>,
    s: &Settings,
    labels: &[String],
) -> Option<Crit> {
    let r = r?;
    let qs: Vec<(usize, PeakQ)> = (0..t.n_files)
        .filter_map(|k| c.of(t, r, k).map(|q| (k, q)))
        .collect();
    if qs.is_empty() {
        return None;
    }
    let good = qs.iter().filter(|x| x.1.ok()).count();
    let best = qs
        .iter()
        .filter(|x| x.1.ok() || good == 0)
        .max_by(|a, b| a.1.snr.total_cmp(&b.1.snr))?;
    let q = best.1;
    let what = format!(
        "S/N {:.0}, FWHM {:.0} s, {} scans, apex {:.2} min in {}",
        q.snr, q.fwhm_s, q.points, q.apex_rt, labels[best.0]
    );
    let text = if good > 0 {
        format!(
            "peak in {good} of the {} cells the pipeline found ({}); best: {what}",
            qs.len(),
            limits(s)
        )
    } else {
        format!(
            "none of the {} cells the pipeline found is a chromatographic peak ({}): {} ({what})",
            qs.len(),
            limits(s),
            q.why.unwrap_or("")
        )
    };
    Some(crit(PEAK, if good > 0 { "pass" } else { "fail" }, text))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn trace(f: impl Fn(f64) -> f64) -> (Vec<f64>, Vec<f64>) {
        let rt: Vec<f64> = (0..600).map(|i| 0.4 + i as f64 * 0.04).collect();
        let y = rt
            .iter()
            .enumerate()
            .map(|(i, &t)| f(t) + 1000.0 + 300.0 * (((i * 7919) % 13) as f64 / 13.0))
            .collect();
        (rt, y)
    }

    #[test]
    fn a_gaussian_peak_passes() {
        let (rt, y) = trace(|t| 2e5 * (-((t - 12.0) / 0.1f64).powi(2) / 2.0).exp());
        let q = judge(&rt, &y, 12.0, &Settings::default());
        assert!(q.ok(), "{q:?}");
        assert!(q.fwhm_s > 10.0 && q.fwhm_s < 25.0, "{q:?}");
    }

    #[test]
    fn a_small_isomer_in_a_sparse_trace_passes() {
        // single noise ions in one scan out of seven, a strong isomer 0.9 min before: the isomer is not noise
        let rt: Vec<f64> = (0..270).map(|i| 0.5 + i as f64 * 0.05).collect();
        let y: Vec<f64> = rt
            .iter()
            .enumerate()
            .map(|(i, &t)| {
                let g = |c: f64, a: f64| a * (-((t - c) / 0.07f64).powi(2) / 2.0).exp();
                g(7.4, 1.1e6) + g(8.3, 8.0e4) + if i % 7 == 3 { 3e3 } else { 0.0 }
            })
            .collect();
        let q = judge(&rt, &y, 8.3, &Settings::default());
        assert!(q.ok() && q.snr > 20.0, "{q:?}");
    }

    #[test]
    fn a_hump_a_spike_and_the_front_fail() {
        let s = Settings::default();
        let (rt, y) = trace(|t| 3e3 * (-((t - 10.0) / 6.0f64).powi(2) / 2.0).exp());
        assert_eq!(judge(&rt, &y, 10.0, &s).why, Some("S/N too low"));
        let (rt, y) = trace(|t| 2e6 * (-((t - 10.0) / 2.0f64).powi(2) / 2.0).exp());
        assert_eq!(
            judge(&rt, &y, 10.0, &s).why,
            Some("too broad for a chromatographic peak")
        );
        let (rt, y) = trace(|t| if (t - 8.0).abs() < 0.02 { 5e5 } else { 0.0 });
        assert_eq!(judge(&rt, &y, 8.0, &s).why, Some("too few scans"));
        let (rt, y) = trace(|t| 2e5 * (-((t - 0.5) / 0.1f64).powi(2) / 2.0).exp());
        assert_eq!(judge(&rt, &y, 0.5, &s).why, Some("apex in the dead volume"));
    }
}
