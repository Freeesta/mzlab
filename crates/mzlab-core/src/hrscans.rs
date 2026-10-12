//! Single scans of CENTROID files read from the mzML by offset (the scan table of the index gives `start..end` of each `<spectrum>`; the whole mzML
//! is never held): `/api/spectra` (`App.spectra`: the scans `i0..=i1` of a level, each with its own centroids in high resolution, binned and
//! optionally merged by nominal mass otherwise), `/api/scan` (`Item.scan`: one scan as the file has it, with its parent and activation) and
//! `/api/scanavg` (`Item.scan_avg`: the mean of up to 200 scans on cells of 3 ppm or 0.1 Da). `Ok(None)` = Python's (profile scans, a request
//! Python must refuse, a scan the index does not know).

use crate::error::Result;
use crate::idxread::{ReadAt, Scan};
use crate::iteminfo::{hr_profile, HrProfile};
use crate::json::{round, Out};
use crate::routes::{Query, Routes};
use crate::specpeaks::merge_unit;
use std::collections::BTreeMap;

const MAX_SCANS: i64 = 60;
const MAX_AVERAGE: usize = 200;

impl<R: ReadAt> Routes<R> {
    /// The routes of this module; `None` = not answered here.
    pub fn mzml_route<M: ReadAt>(&mut self, path: &str, q: &Query, mzml: &mut M) -> Result<Option<String>> {
        if !self.has_scans() {
            return Ok(None);
        }
        match path {
            "/api/spectra" => self.spectra_centroids(q, mzml),
            "/api/scan" => self.scan_route(q, mzml),
            "/api/scanavg" => self.scanavg_route(q, mzml),
            _ => Ok(None),
        }
    }

    /// `/api/spectra` of a level with centroids only.
    fn spectra_centroids<M: ReadAt>(&mut self, q: &Query, mzml: &mut M) -> Result<Option<String>> {
        let (Some(i0), Some(i1)) = (int(q, "i0"), int(q, "i1")) else { return Ok(None) };
        let level = q.level();
        let bin = q.num("bin")?.unwrap_or(0.1);
        let of_level = self.idx.scans.iter().filter(|s| s.level == level);
        let nscans = of_level.clone().count();
        let pool_has_profile = if nscans > 0 { of_level.clone().any(|s| s.profile) } else { self.idx.scans.iter().any(|s| s.profile) };
        let hr = hr_profile(&self.idx.scans, level == 1, q.get("hr") != Some("0"));
        if pool_has_profile || nscans == 0 || !(hr.on || bin > 0.0) {
            return Ok(None);
        }
        let Some(si) = self.idx.section_of(level, None) else { return Ok(None) };
        let precursor = q.num("prec")?.or(q.num("precursor")?);
        let tol = match precursor {
            Some(p) if level > 1 && p != 0.0 && hr.on => (p * hr.ppm * 1e-6).max(1e-3),
            _ => 0.6,
        };
        let sel = self.selected(level, si, precursor, tol, q.some("filt"))?;
        if i0 > i1 || i1 - i0 + 1 > MAX_SCANS || i0 < 0 || i0 as usize >= sel.len() {
            return Ok(None);
        }
        let ar = self.idx.arrays(si)?.clone();
        let merge = q.get("merge") == Some("1") && !hr.on;
        let nd = if hr.on { hr.dec + 1 } else { 3 };
        let mut o = Out::default();
        o.raw("{\"n\":").int(sel.len() as i64).raw(",\"scans\":[");
        for (k, j) in (i0 as usize..=(i1 as usize).min(sel.len() - 1)).enumerate() {
            let (sid, rt) = (ar.scan_ids[sel[j]], ar.rt[sel[j]]);
            let (mut mz, mut y) = read_scan(&self.idx.scans[sid as usize], mzml)?;
            if !hr.on && !mz.is_empty() {
                (mz, y) = bin_cells(&mz, &y, bin);
            }
            if merge {
                (mz, y) = merge_unit(&mz, &y);
            }
            o.raw(if k > 0 { "," } else { "" }).raw("{\"i\":").int(j as i64).raw(",\"sid\":").int(sid);
            o.raw(",\"rt\":").num(round(rt, 4)).raw(",\"mode\":\"centroid\",\"mz\":").rounded(&mz, nd).raw(",\"y\":").rounded(&y, 1).raw("}");
        }
        o.raw("]}");
        Ok(Some(o.0))
    }

    /// `/api/scan`: one scan with its own centroids and what the file says about it.
    fn scan_route<M: ReadAt>(&mut self, q: &Query, mzml: &mut M) -> Result<Option<String>> {
        let Some(sid) = int(q, "sid").filter(|s| *s >= 0 && (*s as usize) < self.idx.scans.len()) else { return Ok(None) };
        let sid = sid as usize;
        let scans = &self.idx.scans;
        let s = &scans[sid];
        let parent = match s.parent {
            Some(p) if p < 0 || p as usize >= scans.len() => return Ok(None),
            p => p,
        };
        let d = hr_profile(scans, s.level == 1, q.get("hr") != Some("0")).dec + 1;
        let (mz, y) = read_scan(s, mzml)?;
        let mut o = Out::default();
        o.raw("{\"sid\":").int(sid as i64).raw(",\"no\":").int(s.number(sid)).raw(",\"parent_no\":");
        match parent {
            Some(p) => o.int(scans[p as usize].number(p as usize)),
            None => o.raw("null"),
        };
        o.raw(",\"rt\":").num(round(s.rt, 4)).raw(",\"level\":").int(s.level as i64).raw(",\"mz\":").rounded(&mz, d).raw(",\"y\":").rounded(&y, 1);
        let at = |v: f64| round(v, d);
        let prec = s.iso.map(|(a, b)| at((a + b) / 2.0)).or(s.precursor.filter(|p| *p != 0.0).map(at));
        o.raw(",\"prec\":").opt(prec).raw(",\"lo\":").opt(s.iso.map(|i| at(i.0))).raw(",\"hi\":").opt(s.iso.map(|i| at(i.1)));
        o.raw(",\"act\":");
        match &s.act {
            Some(a) => o.str(a),
            None => o.raw("null"),
        };
        o.raw(",\"ce\":").opt(s.collision_energy).raw(",\"nce\":").raw(if self.nce() { "true" } else { "false" });
        o.raw(",\"res\":").opt(s.res).raw(",\"an\":");
        match &s.an {
            Some(a) => o.str(a),
            None => o.raw("null"),
        };
        o.raw(",\"parent\":");
        match parent {
            Some(p) => o.int(p),
            None => o.raw("null"),
        };
        o.raw(",\"filter\":").str(&s.filter).raw(",\"profile\":").raw(if s.profile { "true" } else { "false" }).raw("}");
        Ok(Some(o.0))
    }

    /// `/api/scanavg`: the scans `sids` (at most 200, all of the level of the first) summed per cell, the intensity divided by the number of scans.
    fn scanavg_route<M: ReadAt>(&mut self, q: &Query, mzml: &mut M) -> Result<Option<String>> {
        let Some(list) = q.get("sids") else { return Ok(None) };
        let mut sids = Vec::new();
        for t in list.split(',').filter(|t| !t.is_empty()) {
            let Ok(v) = t.trim().parse::<i64>() else { return Ok(None) };
            sids.push(v);
        }
        if sids.len() > MAX_AVERAGE {
            return Ok(None);
        }
        let n_all = self.idx.scans.len() as i64;
        sids.retain(|s| *s >= 0 && *s < n_all);
        let Some(&first) = sids.first() else { return Ok(None) };
        let level = self.idx.scans[first as usize].level;
        let hr = hr_profile(&self.idx.scans, level == 1, q.get("hr") != Some("0"));
        let cell = cell_of(&hr);
        // sums in the order of the concatenated arrays (np.bincount), the cells ordered (np.unique)
        let mut cells: BTreeMap<i64, (f64, f64)> = BTreeMap::new();
        for &sid in &sids {
            let (mz, y) = read_scan(&self.idx.scans[sid as usize], mzml)?;
            for (m, v) in mz.iter().zip(&y) {
                let e = cells.entry(cell(*m)).or_insert((0.0, 0.0));
                e.0 += v;
                e.1 += v * m;
            }
        }
        let n = sids.len() as f64;
        let rt = sids.iter().map(|&s| self.idx.scans[s as usize].rt).sum::<f64>() / n;
        let mz: Vec<f64> = cells.values().map(|(sy, smy)| smy / sy.max(1e-12)).collect();
        let y: Vec<f64> = cells.values().map(|(sy, _)| sy / n).collect();
        let mut o = Out::default();
        o.raw("{\"sids\":[");
        for (k, s) in sids.iter().enumerate() {
            o.raw(if k > 0 { "," } else { "" }).int(*s);
        }
        o.raw("],\"level\":").int(level as i64).raw(",\"n\":").int(sids.len() as i64).raw(",\"rt\":").num(round(rt, 4));
        o.raw(",\"mz\":").rounded(&mz, hr.dec + 1).raw(",\"y\":").rounded(&y, 1).raw("}");
        Ok(Some(o.0))
    }

    /// The `nce` flag of the run (`Run.nce`), as `/api/dda` has it.
    fn nce(&self) -> bool {
        self.idx.run.get("nce").and_then(|v| v.bool()).unwrap_or(false)
    }
}

/// Cells of 3 ppm in high resolution, of 0.1 Da otherwise (`Item.scan_avg`).
fn cell_of(hr: &HrProfile) -> Box<dyn Fn(f64) -> i64> {
    if hr.on {
        let w = (3e-6f64).ln_1p();
        Box::new(move |m| (m.max(1e-9).ln() / w).floor() as i64)
    } else {
        Box::new(|m| (m / 0.1).floor() as i64)
    }
}

fn int(q: &Query, k: &str) -> Option<i64> {
    q.some(k)?.trim().parse().ok()
}

/// The peaks of one scan merged per bin of `bin` Da (`Item.scans`), m/z weighted by intensity.
fn bin_cells(mz: &[f64], y: &[f64], bin: f64) -> (Vec<f64>, Vec<f64>) {
    let mut cells: BTreeMap<i64, (f64, f64)> = BTreeMap::new();
    for (m, v) in mz.iter().zip(y) {
        let e = cells.entry((m / bin).floor() as i64).or_insert((0.0, 0.0));
        e.0 += v;
        e.1 += v * m;
    }
    (cells.values().map(|(sy, smy)| smy / sy.max(1e-12)).collect(), cells.values().map(|(sy, _)| *sy).collect())
}

/// Centroids of one scan as the mzML has them (`Run.read`): m/z and intensity, as many as the shorter array.
pub(crate) fn read_scan<M: ReadAt>(sc: &Scan, mzml: &mut M) -> Result<(Vec<f64>, Vec<f64>)> {
    let mut buf = vec![0u8; (sc.end - sc.start) as usize];
    mzml.read_at(sc.start, &mut buf)?;
    let mut found = None;
    crate::mzml::parse(&buf[..], |it| {
        if let crate::mzml::Item::Spectrum(s) = it {
            found.get_or_insert(s);
        }
    })?;
    let s = found.ok_or_else(|| crate::error::Error::new("err.scan.outside"))?;
    let n = s.mz.len().min(s.intensity.len());
    Ok((s.mz[..n].to_vec(), (0..n).map(|i| s.intensity_f64(i)).collect()))
}
