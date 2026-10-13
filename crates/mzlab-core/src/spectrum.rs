//! `/api/spectrum` for unit-resolution centroid files: the mean spectrum of a window of retention time (`Item.spectrum`, `_binned`, `window_scans`
//! and `_spec_json` of `mzlab/explore.py` / `mzlab/app.py`). Anything else (profile spectra, high resolution, a background to subtract) is Python's:
//! the route returns `None`.

use crate::error::Result;
use crate::idxread::ReadAt;
use crate::iteminfo::hr_profile;
use crate::json::{round, Out};
use crate::routes::{Query, Routes};
use crate::specpeaks::merge_unit;

impl<R: ReadAt> Routes<R> {
    pub(crate) fn spectrum(&mut self, q: &Query) -> Result<Option<String>> {
        let (Some(rt0), Some(rt1)) = (q.num("rt0")?, q.num("rt1")?) else {
            return Ok(None);
        };
        if q.some("bgk").is_some() {
            return Ok(None);
        }
        let level = q.level();
        let precursor = q.num("precursor")?;
        let bin = q.num("bin")?.unwrap_or(0.1);
        let merge = q.get("merge") == Some("1");
        let filt = q.some("filt");
        // the mode of the scans of this level (all of them when there is none): only centroids, and not high resolution
        let of_level: Vec<&crate::idxread::Scan> =
            self.idx.scans.iter().filter(|s| s.level == level).collect();
        let pool: Vec<&crate::idxread::Scan> = if of_level.is_empty() {
            self.idx.scans.iter().collect()
        } else {
            of_level
        };
        let nprof = pool.iter().filter(|s| s.profile).count();
        let (lo, hi) = (rt0.min(rt1), rt0.max(rt1));
        if nprof != 0 {
            return match self.profile_level(level)? {
                Some((_, step)) => Ok(Some(self.profile_spectrum(q, level, (lo, hi), step)?)),
                None => Ok(None),
            };
        }
        let hr = hr_profile(&self.idx.scans, level == 1, q.get("hr") != Some("0"));
        if hr.on {
            return self.hr_spectrum(level, (lo, hi), precursor, filt, &hr);
        }
        if !(bin > 0.0) {
            return Ok(None);
        }
        let Some(m) = self.binned(level, lo, hi, precursor, bin, filt)? else {
            return Ok(Some(EMPTY.into()));
        };
        Ok(Some(binned_json(m, 3, merge)))
    }
}

pub(crate) const EMPTY: &str = "{\"mode\":\"centroid\",\"mz\":[],\"y\":[],\"scans\":0,\"i0\":null,\"i1\":null,\"n\":0}";

impl<R: ReadAt> Routes<R> {
    /// Positions (in the section) of the scans that pass the filter and the precursor of a request (`_scan_ids`).
    pub(crate) fn selected(&mut self, level: u8, si: usize, precursor: Option<f64>, prec_tol: f64, filt: Option<&str>) -> Result<Vec<usize>> {
        let ar = self.idx.arrays(si)?.clone();
        let want_prec = if level > 1 { precursor } else { None };
        Ok((0..ar.rt.len())
            .filter(|&j| {
                let s = &self.idx.scans[ar.scan_ids[j] as usize];
                filt.is_none_or(|f| s.filter == f || s.key() == f) && want_prec.is_none_or(|p| (s.prec(-1.0) - p).abs() <= prec_tol)
            })
            .collect())
    }

    /// `None` when the file has no section of `level`.
    pub(crate) fn binned(&mut self, level: u8, lo: f64, hi: f64, precursor: Option<f64>, bin: f64, filt: Option<&str>) -> Result<Option<Binned>> {
        self.binned_cells(level, lo, hi, (precursor, 0.6), filt, |m| (m / bin).floor() as i64)
    }

    /// The mean spectrum of a window with the peaks summed per cell `cell(m/z)` (cells come in order because the peaks do).
    pub(crate) fn binned_cells(&mut self, level: u8, lo: f64, hi: f64, (precursor, prec_tol): (Option<f64>, f64), filt: Option<&str>, cell_of: impl Fn(f64) -> i64) -> Result<Option<Binned>> {
        let Some(si) = self.idx.section_of(level, self.pol) else {
            return Ok(None);
        };
        let ar = self.idx.arrays(si)?.clone();
        let want_prec = if level > 1 { precursor } else { None };
        let keep = |j: usize, idx: &crate::idxread::Index<R>| {
            let s = &idx.scans[ar.scan_ids[j] as usize];
            filt.is_none_or(|f| s.filter == f || s.key() == f)
                && want_prec.is_none_or(|p| (s.prec(-1.0) - p).abs() <= prec_tol)
        };
        let ok: Vec<bool> = (0..ar.rt.len())
            .map(|j| ar.rt[j] >= lo && ar.rt[j] <= hi && keep(j, &self.idx))
            .collect();
        let n = ok.iter().filter(|b| **b).count();
        let sel: Vec<usize> = (0..ar.rt.len()).filter(|&j| keep(j, &self.idx)).collect();
        let in_win: Vec<usize> = sel
            .iter()
            .enumerate()
            .filter(|(_, &j)| ar.rt[j] >= lo && ar.rt[j] <= hi)
            .map(|(i, _)| i)
            .collect();
        let (mut mz, mut y) = (vec![], vec![]);
        if n > 0 {
            // the peaks come in order of m/z, so the cells of `floor(mz / bin)` come in order too
            let (mut cell, mut sy, mut smy) = (i64::MIN, 0.0f64, 0.0f64);
            let (mut cm, mut cy) = (vec![], vec![]);
            let total = self.idx.sections[si].n_peaks;
            self.idx.each_peak(si, 0, total, |pos, m, v| {
                if !ok[pos] {
                    return;
                }
                let c = cell_of(m);
                if c != cell {
                    if cell != i64::MIN {
                        cm.push(smy / sy.max(1e-12));
                        cy.push(sy / n as f64);
                    }
                    cell = c;
                    sy = 0.0;
                    smy = 0.0;
                }
                sy += v;
                smy += v * m;
            })?;
            if cell != i64::MIN {
                cm.push(smy / sy.max(1e-12));
                cy.push(sy / n as f64);
            }
            mz = cm;
            y = cy;
        }
        Ok(Some(Binned { mz, y, n, sel, in_win, scan_ids: ar.scan_ids.iter().map(|&v| v as i64).collect() }))
    }
}

/// The mean spectrum of a window, peaks merged within `bin` (`Item._binned`), not rounded.
pub(crate) struct Binned {
    pub mz: Vec<f64>,
    pub y: Vec<f64>,
    /// Scans of the window.
    pub n: usize,
    /// Positions (in the section) of the scans that pass the filter, and the indices among them that fall in the window.
    pub sel: Vec<usize>,
    pub in_win: Vec<usize>,
    pub scan_ids: Vec<i64>,
}

/// The answer for a window: the peaks rounded to `nd` decimals (m/z) and 1 (intensity), the scans of the window and, for one scan, its number.
pub(crate) fn binned_json(m: Binned, nd: usize, merge: bool) -> String {
    let mut o = Out::default();
    let Binned { mut mz, mut y, n, sel, in_win, scan_ids } = m;
    if merge {
        let (a, b) = merge_unit(&mz, &y);
        mz = a;
        y = b;
    }
    o.raw("{\"mode\":\"centroid\",\"mz\":[");
    for (i, v) in mz.iter().enumerate() {
        o.raw(if i > 0 { "," } else { "" }).num(round(*v, nd));
    }
    o.raw("],\"y\":[");
    for (i, v) in y.iter().enumerate() {
        o.raw(if i > 0 { "," } else { "" }).num(round(*v, 1));
    }
    o.raw("],\"scans\":").int(n as i64).raw(",\"i0\":");
    match in_win.first() {
        Some(i) => o.int(*i as i64),
        None => o.raw("null"),
    };
    o.raw(",\"i1\":");
    match in_win.last() {
        Some(i) => o.int(*i as i64),
        None => o.raw("null"),
    };
    o.raw(",\"n\":").int(sel.len() as i64);
    if in_win.len() == 1 {
        o.raw(",\"sid\":").int(scan_ids[sel[in_win[0]]]);
    }
    o.raw("}");
    o.0
}
