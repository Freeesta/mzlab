//! High-resolution centroid files from the index: the mean spectrum of a window with cells of 3 ppm (`Item._binned` with `hr`) and the table of the
//! MS2 scans of a data-dependent file (`Item.dda`, `/api/dda`). A window that holds one scan is Python's (its own centroids are read from the file,
//! the peak table of a high-resolution file is single precision).

use crate::error::Result;
use crate::idxread::ReadAt;
use crate::hrscans::read_scan;
use crate::iteminfo::{hr_profile, HrProfile};
use crate::json::{round, Out};
use crate::routes::{Query, Routes};
use crate::spectrum::{binned_json, Binned, EMPTY};

impl<R: ReadAt> Routes<R> {
    pub(crate) fn hr_spectrum(&mut self, level: u8, win: (f64, f64), precursor: Option<f64>, filt: Option<&str>, hr: &HrProfile) -> Result<Option<String>> {
        Ok(match self.hr_window(level, win, precursor, filt, hr)? {
            None => Some(EMPTY.into()),
            Some(m) if m.n == 1 => None,
            Some(m) => Some(binned_json(m, hr.dec + 1, false)),
        })
    }

    /// `/api/spectrum` of a window that holds ONE scan of a high-resolution centroid file: the centroids of the scan as the file has them (the peak
    /// table is single precision), read from the mzML. `None` = not this case (the index answers, or Python does).
    pub fn spectrum_exact<M: ReadAt>(&mut self, q: &Query, mzml: &mut M) -> Result<Option<String>> {
        let (Some(rt0), Some(rt1)) = (q.num("rt0")?, q.num("rt1")?) else {
            return Ok(None);
        };
        let level = q.level();
        let hr = hr_profile(&self.idx.scans, level == 1, q.get("hr") != Some("0"));
        if q.some("bgk").is_some() || !hr.on || self.idx.scans.iter().any(|s| s.level == level && s.profile) {
            return Ok(None);
        }
        let precursor = q.num("precursor")?;
        let Some(mut m) = self.hr_window(level, (rt0.min(rt1), rt0.max(rt1)), precursor, q.some("filt"), &hr)? else {
            return Ok(None);
        };
        if m.n != 1 || m.in_win.len() != 1 {
            return Ok(None);
        }
        let sid = m.scan_ids[m.sel[m.in_win[0]]] as usize;
        let Some(sc) = self.idx.scans.get(sid) else {
            return Ok(None);
        };
        if !m.mz.is_empty() {
            (m.mz, m.y) = read_scan(sc, mzml)?;
        }
        Ok(Some(binned_json(m, hr.dec + 1, false)))
    }

    /// The mean spectrum of a window of a high-resolution file: cells of 3 ppm, the precursor within the tolerance of the profile (ppm of the
    /// precursor, at least 1 mDa, as `Item.ptol`). `None` = the file has no scans of this level.
    fn hr_window(&mut self, level: u8, (lo, hi): (f64, f64), precursor: Option<f64>, filt: Option<&str>, hr: &HrProfile) -> Result<Option<Binned>> {
        let tol = match precursor {
            Some(p) if level > 1 && p != 0.0 => (p * hr.ppm * 1e-6).max(1e-3),
            _ => 0.6,
        };
        let cell = (3e-6f64).ln_1p();
        self.binned_cells(level, lo, hi, (precursor, tol), filt, |m| (m.max(1e-9).ln() / cell).floor() as i64)
    }

    /// `/api/dda`: every MS2 of the file with its parent, isolation window and activation, in columns, plus the survey scans.
    pub(crate) fn dda(&self, q: &Query) -> Result<Option<String>> {
        let scans = &self.idx.scans;
        let d = hr_profile(scans, false, q.get("hr") != Some("0")).dec + 1;
        let ms2: Vec<usize> = (0..scans.len()).filter(|&i| scans[i].level > 1).collect();
        let ms1: Vec<usize> = (0..scans.len()).filter(|&i| scans[i].level == 1).collect();
        let mut o = Out::default();
        let col = |o: &mut Out, key: &str, f: &dyn Fn(usize) -> Option<f64>, nd: Option<usize>| {
            o.raw(key).raw("[");
            for (n, &i) in ms2.iter().enumerate() {
                o.raw(if n > 0 { "," } else { "" }).opt(f(i).map(|v| nd.map_or(v, |nd| round(v, nd))));
            }
            o.raw("]");
        };
        o.raw("{\"sid\":[");
        for (n, i) in ms2.iter().enumerate() {
            o.raw(if n > 0 { "," } else { "" }).int(*i as i64);
        }
        o.raw("],");
        col(&mut o, "\"rt\":", &|i| Some(scans[i].rt), Some(4));
        o.raw(",\"no\":[");
        for (n, &i) in ms2.iter().enumerate() {
            o.raw(if n > 0 { "," } else { "" }).int(scans[i].number(i));
        }
        o.raw("],");
        col(&mut o, "\"prec\":", &|i| scans[i].precursor.filter(|p| *p != 0.0), Some(d));
        o.raw(",");
        col(&mut o, "\"tgt\":", &|i| scans[i].iso.map(|(a, b)| (a + b) / 2.0), Some(d));
        o.raw(",\"parent\":[");
        for (n, &i) in ms2.iter().enumerate() {
            o.raw(if n > 0 { "," } else { "" });
            match scans[i].parent {
                Some(p) => o.int(p),
                None => o.raw("null"),
            };
        }
        o.raw("],");
        let parent_rt = |i: usize| scans[i].parent.filter(|&p| p >= 0 && (p as usize) < scans.len()).map(|p| scans[p as usize].rt);
        col(&mut o, "\"prt\":", &parent_rt, Some(4));
        o.raw(",");
        col(&mut o, "\"lo\":", &|i| scans[i].iso.map(|(a, _)| a), Some(d));
        o.raw(",");
        col(&mut o, "\"hi\":", &|i| scans[i].iso.map(|(_, b)| b), Some(d));
        o.raw(",\"act\":[");
        for (n, &i) in ms2.iter().enumerate() {
            o.raw(if n > 0 { "," } else { "" });
            match &scans[i].act {
                Some(a) => o.str(a),
                None => o.raw("null"),
            };
        }
        o.raw("],");
        col(&mut o, "\"ce\":", &|i| scans[i].collision_energy, None);
        o.raw(",");
        col(&mut o, "\"pint\":", &|i| scans[i].pint, None);
        o.raw(",\"ms1\":{\"sid\":[");
        for (n, i) in ms1.iter().enumerate() {
            o.raw(if n > 0 { "," } else { "" }).int(*i as i64);
        }
        o.raw("],\"rt\":[");
        for (n, &i) in ms1.iter().enumerate() {
            o.raw(if n > 0 { "," } else { "" }).num(round(scans[i].rt, 4));
        }
        o.raw("]},\"nce\":").raw(if self.idx.run.get("nce").and_then(|v| v.bool()).unwrap_or(false) { "true" } else { "false" });
        o.raw("}");
        Ok(Some(o.0))
    }
}
