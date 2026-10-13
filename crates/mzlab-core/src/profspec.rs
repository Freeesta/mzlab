//! Spectra of PROFILE files from the index (`/api/spectrum` and `/api/spectra` of Python for a file whose scans are all profile): the points
//! of the window are summed on the grid of the instrument (`rint(m/z / step)`), the peaks are found by `specpeaks::profile_peaks` and the line
//! gets back the zeros the converter left out (`pad_zeros`). `Ok(None)` = Python's (a level with centroids or mixed, single-precision sections,
//! a spacing that cannot be read, a request Python must refuse).

use crate::error::Result;
use crate::idxread::{ReadAt, Scan};
use crate::json::{round, Out};
use crate::routes::{Query, Routes};
use crate::specpeaks::{pad_zeros, profile_peaks};

const MAX_SCANS: usize = 60;

impl<R: ReadAt> Routes<R> {
    /// m/z and intensity of the scans at the section positions `positions`, each in table order (m/z ascending).
    fn scan_points(&mut self, si: usize, positions: &[usize]) -> Result<Vec<(Vec<f64>, Vec<f64>)>> {
        let mut out = vec![(Vec::new(), Vec::new()); positions.len()];
        let at: std::collections::HashMap<usize, usize> = positions.iter().enumerate().map(|(k, &p)| (p, k)).collect();
        let n = self.idx.sections[si].n_peaks;
        self.idx.each_peak(si, 0, n, |p, mz, y| {
            if let Some(&k) = at.get(&p) {
                out[k].0.push(mz);
                out[k].1.push(y);
            }
        })?;
        Ok(out)
    }

    /// `Item._step`: the spacing of the points, read from up to 20 scans in the middle of the run (the shortest gap above 1e-4 Da, to 4 decimals).
    fn point_step(&mut self, si: usize) -> Result<Option<f64>> {
        let ids = self.idx.arrays(si)?.scan_ids.clone();
        let mid = ids.len() / 2;
        let positions: Vec<usize> = (mid..ids.len().min(mid + 20)).collect();
        for pts in self.scan_points(si, &positions)? {
            if pts.0.len() > 20 {
                return Ok(step_of(&pts.0));
            }
        }
        // A converter that leaves the zero bins out (the `.wiff` reader) gives baseline scans of a few points: when none of the 20 has more than 20, the
        // spacing is read from the scans of the whole section that do (up to 20, the shortest gap of all).
        let all: Vec<usize> = (0..ids.len()).collect();
        let mut gap = f64::INFINITY;
        let mut used = 0;
        for pts in self.scan_points(si, &all)? {
            if pts.0.len() > 20 && used < 20 {
                used += 1;
                gap = gap.min(pts.0.windows(2).map(|w| w[1] - w[0]).filter(|d| *d > 1e-4).fold(f64::INFINITY, f64::min));
            }
        }
        Ok(gap_step(gap))
    }

    /// The spacing of the profile points of `level` when ALL its scans are profile and the file can be answered here.
    pub(crate) fn profile_level(&mut self, level: u8) -> Result<Option<(usize, f64)>> {
        let of_level: Vec<&Scan> = self.idx.scans.iter().filter(|s| s.level == level).collect();
        let pool: Vec<&Scan> = if of_level.is_empty() { self.idx.scans.iter().collect() } else { of_level };
        if pool.is_empty() || pool.iter().any(|s| !s.profile) || of_level_count(&self.idx.scans, level) == 0 {
            return Ok(None);
        }
        let Some(si) = self.idx.section_of(level, self.pol) else { return Ok(None) };
        if self.idx.sections[si].n_scans != of_level_count(&self.idx.scans, level) {
            return Ok(None);
        }
        if let Some((_, st)) = self.steps.iter().find(|(l, _)| *l == level) {
            return Ok(st.map(|s| (si, s)));
        }
        if self.idx.sections[si].f4 {
            return Ok(None); // the points of a single-precision section are not the ones the spacing is read from: `prime_steps` reads them from the mzML
        }
        let st = self.point_step(si)?;
        self.steps.push((level, st));
        Ok(st.map(|s| (si, s)))
    }

    /// The spacing of the profile levels whose peak table is single precision (high-resolution survey scans), read from the exact points of the mzML
    /// as `Item._step` does (the index cannot give it: its m/z are rounded to single precision). A no-op for every other level.
    pub fn prime_steps<M: ReadAt>(&mut self, mzml: &mut M) -> Result<()> {
        let mut levels: Vec<u8> = self.idx.scans.iter().map(|s| s.level).collect();
        levels.sort_unstable();
        levels.dedup();
        for level in levels {
            let all_profile = self.idx.scans.iter().filter(|s| s.level == level).all(|s| s.profile);
            let f4 = self.idx.section_of(level, self.pol).is_some_and(|si| self.idx.sections[si].f4);
            if !all_profile || !f4 || self.steps.iter().any(|(l, _)| *l == level) {
                continue;
            }
            let ids: Vec<usize> = (0..self.idx.scans.len()).filter(|&i| self.idx.scans[i].level == level).collect();
            let mut step = None;
            for &i in ids.iter().skip(ids.len() / 2).take(20) {
                let (mz, _) = crate::hrscans::read_scan(&self.idx.scans[i], mzml)?;
                if mz.len() > 20 {
                    let gap = mz.windows(2).map(|w| w[1] - w[0]).filter(|d| *d > 1e-4).fold(f64::INFINITY, f64::min);
                    step = gap.is_finite().then(|| (gap * 1e4).round_ties_even() / 1e4).filter(|s| *s > 0.0);
                    break;
                }
            }
            self.steps.push((level, step));
        }
        Ok(())
    }

    /// `/api/spectrum` on a window of a profile file.
    pub(crate) fn profile_spectrum(&mut self, q: &Query, level: u8, (lo, hi): (f64, f64), step: f64) -> Result<String> {
        let b = self.binned_cells(level, lo, hi, (q.num("precursor")?, 0.6), q.some("filt"), rint(step))?;
        let mut o = Out::default();
        o.raw("{");
        let (mz, y, n) = b.as_ref().map_or((vec![], vec![], 0), |b| (b.mz.clone(), b.y.clone(), b.n));
        spec_json(&mut o, &mz, &y, step);
        o.raw(",\"scans\":").int(n as i64);
        match &b {
            Some(b) => window_tail(&mut o, b),
            None => {
                o.raw(",\"i0\":null,\"i1\":null,\"n\":0");
            }
        }
        o.raw("}");
        Ok(o.0)
    }

    /// `/api/spectra`: single scans `i0..=i1` of a profile file (points merged on the grid, as `Item.scans`). A level of single-precision peaks is
    /// the mzML's (`spectra_profile_exact`: the scans are read from the file, as Python does).
    pub(crate) fn spectra_profile(&mut self, q: &Query) -> Result<Option<String>> {
        self.spectra_profile_in::<std::fs::File>(q, None)
    }

    /// `spectra_profile` of a level of single-precision peaks: the points of each scan come from the mzML.
    pub(crate) fn spectra_profile_exact<M: ReadAt>(&mut self, q: &Query, mzml: &mut M) -> Result<Option<String>> {
        self.spectra_profile_in(q, Some(mzml))
    }

    fn spectra_profile_in<M: ReadAt>(&mut self, q: &Query, mut mzml: Option<&mut M>) -> Result<Option<String>> {
        let (Some(i0), Some(i1)) = (index(q, "i0"), index(q, "i1")) else { return Ok(None) };
        let level = q.level();
        let bin = q.num("bin")?.unwrap_or(0.1);
        let Some((si, step)) = self.profile_level(level)? else { return Ok(None) };
        if self.idx.sections[si].f4 != mzml.is_some() {
            return Ok(None);
        }
        let precursor = q.num("prec")?.or(q.num("precursor")?);
        let sel = self.selected(level, si, precursor, 0.6, q.some("filt"))?;
        if !(bin > 0.0) || i0 > i1 || i1 - i0 + 1 > MAX_SCANS as i64 || i0 < 0 || i0 as usize >= sel.len() {
            return Ok(None);
        }
        let i1 = (i1 as usize).min(sel.len() - 1);
        let ar = self.idx.arrays(si)?.clone();
        let positions: Vec<usize> = sel[i0 as usize..=i1].to_vec();
        let pts = match mzml.as_mut() {
            Some(m) => positions.iter().map(|&p| crate::hrscans::read_scan(&self.idx.scans[ar.scan_ids[p] as usize], &mut **m)).collect::<Result<Vec<_>>>()?,
            None => self.scan_points(si, &positions)?,
        };
        let mut o = Out::default();
        o.raw("{\"n\":").int(sel.len() as i64).raw(",\"scans\":[");
        for (k, (mz, y)) in pts.iter().enumerate() {
            let (mz, y) = merge_cells(mz, y, step);
            o.raw(if k > 0 { "," } else { "" }).raw("{\"i\":").int(i0 + k as i64).raw(",\"sid\":").int(ar.scan_ids[positions[k]]);
            o.raw(",\"rt\":").num(round(ar.rt[positions[k]], 4)).raw(",");
            spec_json(&mut o, &mz, &y, step);
            o.raw("}");
        }
        o.raw("]}");
        Ok(Some(o.0))
    }
}

fn of_level_count(scans: &[Scan], level: u8) -> usize {
    scans.iter().filter(|s| s.level == level).count()
}

fn index(q: &Query, k: &str) -> Option<i64> {
    q.some(k)?.trim().parse().ok()
}

/// The points of one scan summed per cell of the grid (`np.unique` + `bincount`), m/z weighted by intensity.
fn merge_cells(mz: &[f64], y: &[f64], step: f64) -> (Vec<f64>, Vec<f64>) {
    let cell = rint(step);
    let (mut om, mut oy) = (Vec::new(), Vec::new());
    let mut i = 0;
    while i < mz.len() {
        let c = cell(mz[i]);
        let (mut sy, mut smy) = (0.0, 0.0);
        while i < mz.len() && cell(mz[i]) == c {
            sy += y[i];
            smy += y[i] * mz[i];
            i += 1;
        }
        om.push(smy / f64::max(sy, 1e-12));
        oy.push(sy);
    }
    (om, oy)
}

/// `"i0":..,"i1":..,"n":..,"sid":..` of `window_scans`, the part both kinds of spectrum share.
pub(crate) fn window_tail(o: &mut Out, b: &crate::spectrum::Binned) {
    o.raw(",\"i0\":");
    match b.in_win.first() {
        Some(i) => o.int(*i as i64),
        None => o.raw("null"),
    };
    o.raw(",\"i1\":");
    match b.in_win.last() {
        Some(i) => o.int(*i as i64),
        None => o.raw("null"),
    };
    o.raw(",\"n\":").int(b.sel.len() as i64);
    if b.in_win.len() == 1 {
        o.raw(",\"sid\":").int(b.scan_ids[b.sel[b.in_win[0]]]);
    }
}

/// `np.rint(mz / step)`.
pub(crate) fn rint(step: f64) -> impl Fn(f64) -> i64 {
    move |m| (m / step).round_ties_even() as i64
}

/// `"mode":"profile","mz":..,"y":..,"pmz":..,"py":..` of `_spec_json`: peaks (m/z to 3 decimals, height to 1) and the padded profile line.
fn spec_json(o: &mut Out, mz: &[f64], y: &[f64], step: f64) {
    let (pm, py) = profile_peaks(mz, y, 0.002);
    let (lm, ly) = pad_zeros(mz, y, step);
    o.raw("\"mode\":\"profile\",\"mz\":").rounded(&pm, 3).raw(",\"y\":").rounded(&py, 1);
    o.raw(",\"pmz\":").rounded(&lm, 3).raw(",\"py\":").rounded(&ly, 1);
}

/// The spacing from the m/z of one scan: its shortest gap above 1e-4 Da, to 4 decimals.
fn step_of(mz: &[f64]) -> Option<f64> {
    gap_step(mz.windows(2).map(|w| w[1] - w[0]).filter(|d| *d > 1e-4).fold(f64::INFINITY, f64::min))
}

fn gap_step(gap: f64) -> Option<f64> {
    if !gap.is_finite() {
        return None;
    }
    let step = (gap * 1e4).round_ties_even() / 1e4;
    (step > 0.0).then_some(step)
}
