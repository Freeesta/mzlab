//! The m/z offset of a unit-resolution instrument (about +0.24 Da on the 3200 QTRAP), given as a number or measured on a known ion from the index.
//! Convention: `offset_da = observed - true`; an XIC of the true m/z M integrates [M + offset - tol, M + offset + tol] and reports M; a corrected
//! m/z is observed - offset. The estimate needs no feature table (`mzlab-feat`'s `target::offset` does), so `trace xic` and `features` share it.

use crate::error::{Error, Result};
use crate::idxread::{Index, ReadAt};
use crate::json::{round, Out};

/// Largest offset taken as a calibration error, Da, and the half-width of the window the ion is looked for in (a bigger one is another ion).
pub const MAX_OFFSET: f64 = 0.6;
/// Half-width of the RT window around a given RT, minutes.
const RT_HALF: f64 = 0.5;
/// The apex must stand this far above the median of the trace; the scans around it (at most `MAX_SCANS`) hold at least `KEEP` of the apex.
const MIN_SNR: f64 = 3.0;
const KEEP: f64 = 0.5;
const MAX_SCANS: usize = 15;
/// A spread of the centroids above this is "wide": still applied, with a warning.
const WIDE_DA: f64 = 0.15;

/// `--mz-offset`: a number of Da, or the known ion to measure it on (and optionally the RT to look near).
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub enum Spec {
    #[default]
    Zero,
    Fixed(f64),
    Auto { mz: f64, rt: Option<f64> },
}

impl std::fmt::Display for Spec {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            Spec::Zero => write!(f, "0"),
            Spec::Fixed(d) => write!(f, "{d}"),
            Spec::Auto { mz, rt: None } => write!(f, "auto:{mz}"),
            Spec::Auto { mz, rt: Some(t) } => write!(f, "auto:{mz}@{t}"),
        }
    }
}

impl Spec {
    /// `0`, a number within the limit, or `auto:MZ[@RT]`. `Err` is `err.mz.offset.spec` (not that form) or `err.mz.offset.range` (above the limit).
    pub fn parse(s: &str) -> Result<Spec> {
        let bad = || Error::new("err.mz.offset.spec").with("text", s.to_string());
        let num = |t: &str| t.parse::<f64>().ok().filter(|x| x.is_finite());
        if let Some(a) = s.strip_prefix("auto:") {
            // byte search: a `char` pattern would pull the str searcher into the wasm
            let (m, t) = match a.bytes().position(|c| c == b'@') {
                Some(i) => (&a[..i], Some(&a[i + 1..])),
                None => (a, None),
            };
            let mz = num(m).filter(|m| *m > 0.0).ok_or_else(bad)?;
            let rt = match t {
                None => None,
                Some(t) => Some(num(t).filter(|t| *t >= 0.0).ok_or_else(bad)?),
            };
            return Ok(Spec::Auto { mz, rt });
        }
        let d = num(s).ok_or_else(bad)?;
        if d.abs() > MAX_OFFSET {
            return Err(Error::new("err.mz.offset.range").with("da", format!("{d}")));
        }
        Ok(if d == 0.0 { Spec::Zero } else { Spec::Fixed(d) })
    }

    pub fn is_zero(&self) -> bool {
        matches!(self, Spec::Zero)
    }

    /// The offset of the file `idx` (polarity `pol`): the number, or the estimate on its MS1 section.
    pub fn resolve<R: ReadAt>(&self, idx: &mut Index<R>, pol: Option<i8>) -> Result<Offset> {
        match *self {
            Spec::Zero => Ok(Offset::default()),
            Spec::Fixed(da) => Ok(Offset { da, auto: None }),
            Spec::Auto { mz, rt } => estimate(idx, pol, mz, rt),
        }
    }
}

/// What an estimate saw.
#[derive(Debug, Clone, PartialEq)]
pub struct Evidence {
    pub ion_mz: f64,
    pub rt_min: Option<f64>,
    /// `ok`, `wide` (spread above 0.15 Da, applied all the same) or `not_found` (offset 0).
    pub status: &'static str,
    pub n_scans: usize,
    pub apex_rt_min: Option<f64>,
    pub apex_intensity: f64,
    pub spread_da: Option<f64>,
    pub snr: f64,
}

/// The offset in force for one file and, when it was measured, how.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Offset {
    pub da: f64,
    pub auto: Option<Evidence>,
}

impl Offset {
    /// `{"mode":..,...}`: the evidence of an estimate, or `{"mode":"fixed"}`.
    pub fn evidence_json(&self) -> String {
        let mut o = Out::default();
        let Some(e) = &self.auto else {
            return "{\"mode\":\"fixed\"}".into();
        };
        o.raw("{\"mode\":\"auto\",\"ion_mz\":").num(e.ion_mz);
        o.raw(",\"rt_min\":").opt(e.rt_min);
        o.raw(",\"status\":").str(e.status);
        o.raw(",\"n_scans\":").int(e.n_scans as i64);
        o.raw(",\"apex_rt_min\":").opt(e.apex_rt_min);
        o.raw(",\"apex_intensity\":").num(e.apex_intensity);
        o.raw(",\"spread_da\":").opt(e.spread_da);
        o.raw(",\"snr\":").num(e.snr).raw("}");
        o.0
    }

    /// The members every output carries: `"mz_offset_da":D,"mz_offset":{..}`; nothing for an offset of 0 that was not measured (so today's bytes stay).
    pub fn members(&self) -> String {
        if self.da == 0.0 && self.auto.is_none() {
            return String::new();
        }
        let mut o = Out::default();
        o.raw("\"mz_offset_da\":").num(self.da);
        format!("{},\"mz_offset\":{}", o.0, self.evidence_json())
    }

    /// The stderr line of one file or sample.
    pub fn line(&self, name: &str) -> String {
        let how = match &self.auto {
            None => "fixed".to_string(),
            Some(e) if e.status == "not_found" => format!("auto on {}, not found", e.ion_mz),
            Some(e) => format!(
                "auto on {}, {} scans, apex {} min, spread {} Da",
                e.ion_mz,
                e.n_scans,
                e.apex_rt_min.unwrap_or(0.0),
                e.spread_da.unwrap_or(0.0)
            ),
        };
        format!("mz offset {name}: {:+} Da ({how})", self.da)
    }

    /// The `warn.*` key of an estimate that did not go cleanly.
    pub fn warning(&self) -> Option<&'static str> {
        match self.auto.as_ref().map(|e| e.status) {
            Some("not_found") => Some("warn.mz.offset.notFound"),
            Some("wide") => Some("warn.mz.offset.wide"),
            _ => None,
        }
    }
}

/// The value at which the weights reach half of their total (the mean of it and the next value on an exact half).
fn weighted_median(v: &mut [(f64, f64)]) -> f64 {
    crate::sortc::slice_by(v, |a, b| a.0.total_cmp(&b.0));
    let half = v.iter().map(|x| x.1).sum::<f64>() / 2.0;
    let mut acc = 0.0;
    for (i, &(x, w)) in v.iter().enumerate() {
        acc += w;
        if acc > half {
            return x;
        }
        if acc == half {
            return v.get(i + 1).map_or(x, |n| (x + n.0) / 2.0);
        }
    }
    0.0
}

fn median(v: &mut [f64]) -> f64 {
    crate::sortc::slice_by(v, f64::total_cmp);
    match v.len() {
        0 => 0.0,
        n if n % 2 == 1 => v[n / 2],
        n => (v[n / 2 - 1] + v[n / 2]) / 2.0,
    }
}

/// The scans around the apex `ap` that hold at least half of it, the higher neighbour first, at most `MAX_SCANS`: `(first, last)`.
fn around(ys: &[f64], ap: usize, inside: &dyn Fn(usize) -> bool) -> (usize, usize) {
    let (n, apex) = (ys.len(), ys[ap]);
    let (mut lo, mut hi) = (ap, ap);
    let ok = |p: Option<usize>| p.filter(|&p| p < n && inside(p) && ys[p] >= KEEP * apex);
    while hi - lo + 1 < MAX_SCANS {
        let (l, r) = (ok(lo.checked_sub(1)), ok(Some(hi + 1)));
        match (l, r) {
            (Some(l), Some(r)) if ys[l] > ys[r] => lo = l,
            (_, Some(r)) => hi = r,
            (Some(l), None) => lo = l,
            (None, None) => break,
        }
    }
    (lo, hi)
}

/// The intensity-weighted centroid of the peaks of each scan in `lo..=hi` that has any, with the scan's total intensity.
fn centroids(peaks: &[(usize, f64, f64)], lo: usize, hi: usize) -> Vec<(f64, f64)> {
    let mut sums = vec![(0.0, 0.0); hi - lo + 1];
    for &(p, m, i) in peaks.iter().filter(|x| (lo..=hi).contains(&x.0)) {
        let s = &mut sums[p - lo];
        s.0 += m * i;
        s.1 += i;
    }
    sums.into_iter()
        .filter(|s| s.1 > 0.0)
        .map(|s| (s.0 / s.1, s.1))
        .collect()
}

/// The offset measured on the ion at `mz` (within ±`MAX_OFFSET` Da, inside ±0.5 min of `rt` when given) in the MS1 section of polarity `pol`:
/// the apex of the 3-point smoothed XIC (it needs a signal of at least `MIN_SNR` times the median of the trace and three scans at half the apex),
/// then the intensity-weighted centroid of the peaks in the window in each scan around it (at most `MAX_SCANS`), and the intensity-weighted median
/// of those centroids minus `mz`. `spread_da` is their median absolute deviation. Rounded to 0.1 mDa so that the number reported is the number applied.
pub fn estimate<R: ReadAt>(
    idx: &mut Index<R>,
    pol: Option<i8>,
    mz: f64,
    rt: Option<f64>,
) -> Result<Offset> {
    let miss = |snr: f64, apex: f64| Offset {
        da: 0.0,
        auto: Some(Evidence {
            ion_mz: mz,
            rt_min: rt,
            status: "not_found",
            n_scans: 0,
            apex_rt_min: None,
            apex_intensity: round(apex, 1),
            spread_da: None,
            snr: round(snr, 1),
        }),
    };
    let Some(si) = idx.section_of(1, pol) else {
        return Ok(miss(0.0, 0.0));
    };
    let (a, b) = (
        idx.search(si, mz - MAX_OFFSET, false)?,
        idx.search(si, mz + MAX_OFFSET, true)?,
    );
    let times = idx.arrays(si)?.rt.clone();
    let n = times.len();
    let inside = |p: usize| rt.is_none_or(|t| (times[p] - t).abs() <= RT_HALF);
    let mut y = vec![0.0; n];
    let mut peaks: Vec<(usize, f64, f64)> = Vec::new();
    if b > a {
        idx.each_peak(si, a, b, |p, m, i| {
            if p < n && inside(p) {
                y[p] += i;
                peaks.push((p, m, i));
            }
        })?;
    }
    let ys = crate::peak1d::smooth(&y, 3);
    let Some(ap) = (0..n)
        .filter(|&p| inside(p))
        .max_by(|&p, &q| ys[p].total_cmp(&ys[q]).then(q.cmp(&p)))
    else {
        return Ok(miss(0.0, 0.0));
    };
    let apex = ys[ap];
    let mut kept: Vec<f64> = (0..n).filter(|&p| inside(p)).map(|p| ys[p]).collect();
    let snr = apex / median(&mut kept).max(1.0);
    if apex <= 0.0 || snr < MIN_SNR {
        return Ok(miss(snr, apex));
    }
    let (lo, hi) = around(&ys, ap, &inside);
    if hi - lo + 1 < 3 {
        return Ok(miss(snr, apex));
    }
    let mut cents = centroids(&peaks, lo, hi);
    if cents.is_empty() {
        return Ok(miss(snr, apex));
    }
    let centre = weighted_median(&mut cents);
    let mut dev: Vec<f64> = cents.iter().map(|c| (c.0 - centre).abs()).collect();
    let spread = round(median(&mut dev), 4);
    Ok(Offset {
        da: round(centre - mz, 4),
        auto: Some(Evidence {
            ion_mz: mz,
            rt_min: rt,
            status: if spread > WIDE_DA { "wide" } else { "ok" },
            n_scans: cents.len(),
            apex_rt_min: Some(round(times[ap], 4)),
            apex_intensity: round(apex, 1),
            spread_da: Some(spread),
            snr: round(snr, 1),
        }),
    })
}
