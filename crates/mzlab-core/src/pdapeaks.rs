//! `dad peaks`, the route of the site, the app and the CLI: the UV peaks of a DAD matrix (retention time, height, lambda-max) and, for each, the ions of the MS
//! scan nearest to its time once the DAD-MS delay is taken off. The delay is the engine's estimate (`/api/delay`) or a value given; which one was used, and
//! the scan taken for each peak, are in the answer, never applied silently.

use crate::error::{Error, Result};
use crate::idxread::ReadAt;
use crate::json::{parse, round, Out};
use crate::mzmlhead::{Chrom, Mzml};
use crate::routes::{Query, Routes};
use crate::scanpick::{nearest, Pick};
use crate::wiff::dad::DadData;
use crate::wiff::dad_peaks::{uv_peaks, PeakOpts, UvPeak};

/// The MS side of the question: the MS level, how many ions per peak, and the offset in minutes (`None`: the delay the engine estimates).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MsOpts {
    pub level: u8,
    pub top: usize,
    pub offset: Option<f64>,
}

impl Default for MsOpts {
    fn default() -> Self {
        Self {
            level: 1,
            top: 5,
            offset: None,
        }
    }
}

/// The offset used and where it comes from: `given`, `delay` (estimated, with its peaks and correlation) or `none` (no UV peak matched the TIC, so 0).
struct Offset {
    min: f64,
    source: &'static str,
    evidence: Option<(f64, f64)>,
}

/// The scan chosen for a peak, its ions `(mz, intensity)` and the warnings of the choice.
type Taken = (Option<Pick>, Vec<(f64, f64)>, Vec<&'static str>);

/// The `top` strongest ions of a `/api/spectrum` answer, strongest first: `(mz, intensity)`.
fn top_ions(spectrum: &str, top: usize) -> Vec<(f64, f64)> {
    let Ok(v) = parse(spectrum.as_bytes()) else {
        return Vec::new();
    };
    let col = |k: &str| {
        v.get(k)
            .map(|a| a.arr().iter().filter_map(|x| x.f64()).collect::<Vec<_>>())
            .unwrap_or_default()
    };
    let mut ions: Vec<(f64, f64)> = col("mz").into_iter().zip(col("y")).collect();
    crate::sortc::slice_by(&mut ions, |a, b| {
        b.1.total_cmp(&a.1).then(a.0.total_cmp(&b.0))
    });
    ions.truncate(top);
    ions
}

impl<R: ReadAt> Routes<R> {
    fn dad_offset<M: ReadAt>(
        &mut self,
        m: &mut Mzml<M>,
        chroms: &[Chrom],
        given: Option<f64>,
    ) -> Result<Offset> {
        if let Some(min) = given {
            return Ok(Offset {
                min,
                source: "given",
                evidence: None,
            });
        }
        let answer = self.delay(m, chroms)?;
        let d = parse(answer.as_bytes())
            .ok()
            .and_then(|v| v.get("delay").cloned())
            .filter(|d| d.get("delay").is_some());
        let num = |d: &crate::json::Value, k: &str| d.get(k).and_then(|x| x.f64()).unwrap_or(0.0);
        Ok(match d {
            Some(d) if num(&d, "peaks") >= 1.0 => Offset {
                min: num(&d, "delay"),
                source: "delay",
                evidence: Some((num(&d, "peaks"), num(&d, "corr"))),
            },
            _ => Offset {
                min: 0.0,
                source: "none",
                evidence: None,
            },
        })
    }

    /// The scan times (minutes) of the MS level; empty when the file has none of it.
    fn ms_times(&mut self, level: u8) -> Result<Vec<f64>> {
        match self.idx.section_of(level, self.pol) {
            Some(si) => Ok(self.idx.arrays(si)?.rt.clone()),
            None => Ok(Vec::new()),
        }
    }

    /// `pda_peaks` of the item `item` (`name` or `name#tag`), the matrix read from the file itself: the entry of the site and of the app. A part of a mixed file has no PDA of its own
    /// (`err.route.unavailable`, as `delay_item`).
    pub fn pda_peaks_item<M: ReadAt>(
        &mut self,
        m: &mut Mzml<M>,
        chroms: &[Chrom],
        item: &str,
        po: &PeakOpts,
        mo: &MsOpts,
    ) -> Result<String> {
        if item.contains('#') && chroms.iter().any(|c| c.kind == "pda") {
            return Err(Error::new("err.route.unavailable"));
        }
        let dad = crate::itemroutes::pda_dad(m)?;
        self.pda_peaks(m, chroms, &dad, po, mo)
    }

    /// The scan nearest to `ms` and its strongest ions, with the warnings of the choice: a time outside the scans, or an empty scan, gives no ions and says why.
    fn ions_at(&mut self, times: &[f64], ms: f64, mo: &MsOpts) -> Result<Taken> {
        if times.is_empty() {
            return Ok((None, Vec::new(), Vec::new()));
        }
        let Ok(pk) = nearest(times, ms, mo.level) else {
            return Ok((None, Vec::new(), vec!["ms_out_of_range"]));
        };
        let q = Query::parse(&format!("rt0={0}&rt1={0}&level={1}", pk.rt, mo.level));
        let ions = top_ions(
            &self.handle("/api/spectrum", &q)?.unwrap_or_default(),
            mo.top,
        );
        let mut warn: Vec<&'static str> = pk.warnings.clone();
        if ions.is_empty() {
            warn.push("ms_scan_empty");
        }
        Ok((Some(pk), ions, warn))
    }

    /// `{"peaks":[{rt_uv, height, area, snr, left, right, lambda_max_nm, abs_max, rt_ms, scan_rt, gap_min, warnings, ions:[{mz, y, rel}]}], trace, offset, level, top, warnings}`.
    /// A peak whose time lies outside the MS scans, or whose scan is empty, has no ions and says why in its `warnings`; it is never dropped.
    pub fn pda_peaks<M: ReadAt>(
        &mut self,
        m: &mut Mzml<M>,
        chroms: &[Chrom],
        dad: &DadData,
        po: &PeakOpts,
        mo: &MsOpts,
    ) -> Result<String> {
        let peaks = uv_peaks(dad, po).ok_or_else(|| Error::new("err.pda.none"))?;
        let off = self.dad_offset(m, chroms, mo.offset)?;
        let times = self.ms_times(mo.level)?;
        let mut notes: Vec<&str> = Vec::new();
        if off.source == "none" {
            notes.push("delay_unmatched");
        }
        if times.is_empty() {
            notes.push("ms_level_missing");
        }
        let mut o = Out::default();
        o.raw("{\"peaks\":[");
        for (k, p) in peaks.iter().enumerate() {
            if k > 0 {
                o.raw(",");
            }
            let ms = p.rt_min + off.min;
            let (pick, ions, warn) = self.ions_at(&times, ms, mo)?;
            write_peak(&mut o, k, p, ms, pick.as_ref(), &ions, &warn);
        }
        o.raw("],\"trace\":");
        match po.nm {
            Some((a, b)) => o
                .raw("{\"kind\":\"band\",\"nm\":[")
                .num(a.min(b))
                .raw(",")
                .num(a.max(b))
                .raw("]}"),
            None => o.raw("{\"kind\":\"twc\"}"),
        };
        write_offset(&mut o, &off);
        o.raw(",\"level\":")
            .int(i64::from(mo.level))
            .raw(",\"top\":")
            .int(mo.top as i64)
            .raw(",\"warnings\":");
        write_list(&mut o, &notes);
        o.raw("}");
        Ok(o.0)
    }
}

/// A list of codes as a JSON array of strings.
fn write_list(o: &mut Out, codes: &[&str]) {
    o.raw("[");
    for (i, w) in codes.iter().enumerate() {
        o.raw(if i > 0 { ",\"" } else { "\"" }).raw(w).raw("\"");
    }
    o.raw("]");
}

/// The `offset` member: the minutes used, where they come from and, for the estimate, the peaks and correlation behind it.
fn write_offset(o: &mut Out, off: &Offset) {
    o.raw(",\"offset\":{\"min\":")
        .num(off.min)
        .raw(",\"source\":\"")
        .raw(off.source)
        .raw("\"");
    if let Some((n, c)) = off.evidence {
        o.raw(",\"peaks\":").int(n as i64).raw(",\"corr\":").num(c);
    }
    o.raw("}");
}

/// One peak of the answer: the UV numbers, the scan taken and its ions (`rel` is the percentage of the strongest of them).
fn write_peak(
    o: &mut Out,
    k: usize,
    p: &UvPeak,
    ms: f64,
    pick: Option<&Pick>,
    ions: &[(f64, f64)],
    warn: &[&str],
) {
    o.raw("{\"n\":")
        .int(k as i64 + 1)
        .raw(",\"rt_uv\":")
        .num(round(p.rt_min, 4))
        .raw(",\"height\":")
        .num(round(p.height, 3));
    o.raw(",\"area\":")
        .num(round(p.area, 3))
        .raw(",\"snr\":")
        .num(round(p.snr, 1));
    o.raw(",\"left\":")
        .num(round(p.left, 4))
        .raw(",\"right\":")
        .num(round(p.right, 4));
    o.raw(",\"lambda_max_nm\":")
        .num(p.lambda_max_nm)
        .raw(",\"abs_max\":")
        .num(round(p.abs_max, 3))
        .raw(",\"rt_ms\":")
        .num(round(ms, 4));
    o.raw(",\"scan_rt\":")
        .opt(pick.map(|pk| round(pk.rt, 4)))
        .raw(",\"gap_min\":")
        .opt(pick.map(|pk| pk.gap));
    o.raw(",\"warnings\":");
    write_list(o, warn);
    o.raw(",\"ions\":[");
    let base = ions.iter().map(|i| i.1).fold(0.0, f64::max);
    for (i, (mz, y)) in ions.iter().enumerate() {
        o.raw(if i > 0 { ",{\"mz\":" } else { "{\"mz\":" })
            .num(round(*mz, 4))
            .raw(",\"y\":")
            .num(round(*y, 1));
        o.raw(",\"rel\":")
            .num(if base > 0.0 {
                (y / base * 1e4).round() / 100.0
            } else {
                0.0
            })
            .raw("}");
    }
    o.raw("]}");
}

impl PeakOpts {
    /// The options as the site and the app pass them: a non-finite `nm0` or `nm1` is the total trace, `top` of 0 is the default.
    pub fn from_args(nm0: f64, nm1: f64) -> Self {
        Self {
            nm: (nm0.is_finite() && nm1.is_finite()).then_some((nm0, nm1)),
            ..Self::default()
        }
    }
}

impl MsOpts {
    /// The options as the site and the app pass them: a non-finite `offset` is the delay the engine estimates; a `top` of 0 or a `level` of 0 is the default.
    pub fn from_args(top: u32, level: u32, offset: f64) -> Self {
        let d = Self::default();
        Self {
            level: u8::try_from(level)
                .ok()
                .filter(|l| *l > 0)
                .unwrap_or(d.level),
            top: if top == 0 { d.top } else { top as usize },
            offset: offset.is_finite().then_some(offset),
        }
    }
}
