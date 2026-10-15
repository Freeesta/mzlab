// Ported from Attributio src/attributio/review/raw.py.
//! What only the raw files know: scan window, isolation window, the MS1 around the precursor (isotopes, co-isolated ions), peak quality and co-elution.
//! The raw data is not read here: the caller supplies it through `RawSource` (the `.mzidx` of the campaign files in mzLab). Without raw files every proof
//! is `None` ("not available"), never an error.

use super::explain::{iso_pattern, Formula, C13};
use super::isotopes::round_to;
use super::purity::sig4;
use super::settings::ReviewSettings;

/// Scan window, isolation window and the MS1 peaks around the precursor of one MS2 scan.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct ScanInfo {
    pub low: f64,
    pub high: f64,
    pub target: f64,
    pub lo: f64,
    pub hi: f64,
    pub ms1_mz: Vec<f64>,
    pub ms1_it: Vec<f64>,
}

/// Where the raw spectra come from. All methods answer `None` when the file or the scan is not available.
pub trait RawSource {
    /// Retention time (min) of a scan of a file.
    fn scan_rt(&self, file: &str, scan: i64) -> Option<f64>;
    /// Scan window, isolation window and MS1 around the precursor of an MS2 scan.
    fn scan_info(&self, file: &str, scan: i64) -> Option<ScanInfo>;
    /// The XICs `(rt, one trace per m/z)` of `mzs` around `rt` ± `half` minutes of a file; `None` when the raw data cannot give them.
    fn xic(
        &self,
        _file: &str,
        _rt: f64,
        _half: f64,
        _mzs: &[f64],
    ) -> Option<(Vec<f64>, Vec<Vec<f64>>)> {
        None
    }
}

/// No raw files: every proof that needs them is "not available".
pub struct NoRaw;

impl RawSource for NoRaw {
    fn scan_rt(&self, _: &str, _: i64) -> Option<f64> {
        None
    }
    fn scan_info(&self, _: &str, _: i64) -> Option<ScanInfo> {
        None
    }
}

/// The folder (among `candidates`, each with the names of the files it holds) that holds most of `files`; `None` when none holds any.
pub fn find_raw_dir<'a>(
    candidates: &'a [(String, Vec<String>)],
    files: &[String],
) -> Option<&'a str> {
    let mut best: Option<(&str, usize)> = None;
    for (dir, held) in candidates {
        let n = files.iter().filter(|f| held.contains(f)).count();
        if n > best.map_or(0, |b| b.1) {
            best = Some((dir, n));
        }
    }
    best.map(|b| b.0)
}

/// (file, scan) of the merged spectrum closest in time to the feature apex, skipping the excluded samples unless nothing else is left.
pub fn best_scan<R: RawSource>(
    raw: &R,
    srcs: &[(String, i64)],
    rt_min: f64,
    exclude: &[String],
) -> Option<(String, i64)> {
    let kept: Vec<&(String, i64)> = srcs
        .iter()
        .filter(|s| !exclude.iter().any(|p| s.0.starts_with(p.as_str())))
        .collect();
    let use_: Vec<&(String, i64)> = if exclude.is_empty() || kept.is_empty() {
        srcs.iter().collect()
    } else {
        kept
    };
    let mut best: Option<(f64, &(String, i64))> = None;
    for s in use_ {
        let Some(t) = raw.scan_rt(&s.0, s.1) else {
            continue;
        };
        if best.map_or(true, |b| (t - rt_min).abs() < b.0) {
            best = Some(((t - rt_min).abs(), s));
        }
    }
    best.map(|b| b.1.clone())
}

/// Peak-quality numbers of an EIC around the apex.
#[derive(Clone, Debug, PartialEq)]
pub struct PeakQuality {
    pub apex_rt: f64,
    pub apex: f64,
    pub fwhm: f64,
    pub asym: Option<f64>,
    pub points: i64,
    pub points_half: i64,
    pub s2b: Option<f64>,
    pub blank_max: Option<f64>,
}

fn cross(rt: &[f64], y: &[f64], k: usize, level: f64, side: i64) -> (f64, usize) {
    let mut i = k as i64;
    let n = y.len() as i64;
    while i + side >= 0 && i + side < n && y[(i + side) as usize] > level {
        i += side;
    }
    let j = i + side;
    let iu = i as usize;
    if j >= 0 && j < n && y[iu] != y[j as usize] {
        let ju = j as usize;
        return (
            rt[iu] + (rt[ju] - rt[iu]) * (y[iu] - level) / (y[iu] - y[ju]),
            iu,
        );
    }
    (rt[iu], iu)
}

/// Simple peak-quality numbers of an EIC around `apex_rt`: apex (rt, intensity), FWHM (min), asymmetry at 10 % height (tail / front), points across the peak
/// (above 5 % of the apex, contiguous) and above half height, signal-to-blank (apex / largest blank intensity within the peak's 10 % width).
pub fn peak_quality(
    rt: &[f64],
    y: &[f64],
    apex_rt: f64,
    blank: Option<(&[f64], &[f64])>,
    window: f64,
) -> Option<PeakQuality> {
    if rt.len() < 3 || !y.iter().any(|&v| v > 0.0) {
        return None;
    }
    let near: Vec<usize> = (0..rt.len())
        .filter(|&i| (rt[i] - apex_rt).abs() <= window)
        .collect();
    let mut k = *near.first()?;
    for &i in &near {
        if y[i] > y[k] {
            k = i;
        }
    }
    let h = y[k];
    if h <= 0.0 {
        return None;
    }
    let (l50, _) = cross(rt, y, k, 0.5 * h, -1);
    let (r50, _) = cross(rt, y, k, 0.5 * h, 1);
    let (l10, _) = cross(rt, y, k, 0.1 * h, -1);
    let (r10, _) = cross(rt, y, k, 0.1 * h, 1);
    let (_, i5) = cross(rt, y, k, 0.05 * h, -1);
    let (_, j5) = cross(rt, y, k, 0.05 * h, 1);
    let (front, tail) = (rt[k] - l10, r10 - rt[k]);
    let mut out = PeakQuality {
        apex_rt: round_to(rt[k], 3),
        apex: sig4(h),
        fwhm: round_to(r50 - l50, 3),
        asym: (front > 0.0).then(|| round_to(tail / front, 2)),
        points: (j5 as i64) - (i5 as i64) + 1,
        points_half: y[i5..=j5].iter().filter(|&&v| v >= 0.5 * h).count() as i64,
        s2b: None,
        blank_max: None,
    };
    if let Some((brt, b)) = blank {
        let bm = (0..brt.len())
            .filter(|&i| brt[i] >= l10 - 0.05 && brt[i] <= r10 + 0.05)
            .map(|i| b[i])
            .fold(0.0, f64::max);
        out.blank_max = Some(sig4(bm));
        out.s2b = (bm > 0.0).then(|| round_to(h / bm, 1));
    }
    Some(out)
}

/// Apex offset (min, other - precursor) and Pearson correlation of two XICs.
#[derive(Clone, Debug, PartialEq)]
pub struct Coelution {
    pub offset: f64,
    pub r: f64,
}

fn argmax(v: &[f64]) -> usize {
    let mut k = 0;
    for i in 0..v.len() {
        if v[i] > v[k] {
            k = i;
        }
    }
    k
}

/// Does a co-isolated ion co-elute with the precursor? `None` when a trace is empty or the axis too short.
pub fn coelution(rt: &[f64], prec_trace: &[f64], other_trace: &[f64]) -> Option<Coelution> {
    let max_of = |v: &[f64]| v.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
    if rt.len() < 4 || max_of(prec_trace) <= 0.0 || max_of(other_trace) <= 0.0 {
        return None;
    }
    let n = prec_trace.len() as f64;
    let (ma, mb) = (
        prec_trace.iter().sum::<f64>() / n,
        other_trace.iter().sum::<f64>() / n,
    );
    let sa = prec_trace.iter().map(|v| (v - ma).powi(2)).sum::<f64>();
    let sb = other_trace.iter().map(|v| (v - mb).powi(2)).sum::<f64>();
    let r = if sa > 0.0 && sb > 0.0 {
        prec_trace
            .iter()
            .zip(other_trace)
            .map(|(a, b)| (a - ma) * (b - mb))
            .sum::<f64>()
            / (sa * sb).sqrt()
    } else {
        0.0
    };
    Some(Coelution {
        offset: round_to(rt[argmax(other_trace)] - rt[argmax(prec_trace)], 3),
        r: round_to(r, 2),
    })
}

/// Observed M+1 / M+2 against the pattern of the ion formula, and the co-isolated ions.
#[derive(Clone, Debug, PartialEq)]
pub struct Ms1Metrics {
    pub obs1: f64,
    pub obs2: f64,
    pub th1: f64,
    pub th2: f64,
    pub iso_ok: Option<bool>,
    pub chim_n: usize,
    pub chim_max: f64,
    pub chim: Vec<(f64, f64)>,
}

/// Observed M+1 / M+2 against the pattern of `ion` and the co-isolated ions (>= `chim_rel` of the precursor, not the precursor or its M+1/M+2) in the
/// isolation window. `None` without an MS1 or without the precursor peak in it.
pub fn ms1_metrics(
    info: Option<&ScanInfo>,
    prec: f64,
    ion: Option<&Formula>,
    s: &ReviewSettings,
) -> Option<Ms1Metrics> {
    let info = info.filter(|i| !i.ms1_mz.is_empty())?;
    let (mz, it) = (&info.ms1_mz, &info.ms1_it);
    let near: Vec<usize> = (0..mz.len())
        .filter(|&i| (mz[i] - prec).abs() <= prec * 10e-6 + 0.002)
        .collect();
    if near.is_empty() {
        return None;
    }
    let m_int = near
        .iter()
        .map(|&i| it[i])
        .fold(f64::NEG_INFINITY, f64::max);
    let sum_in = |a: f64, b: f64| {
        (0..mz.len())
            .filter(|&i| mz[i] >= prec + a && mz[i] <= prec + b)
            .map(|i| it[i])
            .sum::<f64>()
            / m_int
    };
    let (obs1, obs2) = (sum_in(0.999, 1.0085), sum_in(1.993, 2.011));
    let ion = ion.filter(|f| !f.is_empty());
    let (th1, th2) = ion.map_or((f64::NAN, f64::NAN), iso_pattern);
    let iso_ok = ion.map(|_| {
        let ok1 = (obs1 - th1).abs() <= f64::max(0.10, 0.5 * th1);
        let ok2 = th2 < 0.04 || (obs2 - th2).abs() <= f64::max(s.iso_tol_abs, s.iso_tol_rel * th2);
        ok1 && ok2
    });
    let mut co: Vec<(f64, f64)> = Vec::new();
    for i in 0..mz.len() {
        let m = mz[i];
        if m < info.target - info.lo || m > info.target + info.hi {
            continue;
        }
        if (m - prec).abs() <= 0.012
            || [1.0, 2.0]
                .iter()
                .any(|k| (m - (prec + k * C13)).abs() <= 0.012)
        {
            continue;
        }
        if it[i] / m_int >= s.chim_rel {
            co.push((m, it[i] / m_int));
        }
    }
    co.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
    Some(Ms1Metrics {
        obs1,
        obs2,
        th1,
        th2,
        iso_ok,
        chim_n: co.len(),
        chim_max: co.first().map_or(0.0, |c| c.1),
        chim: co.into_iter().take(3).collect(),
    })
}
