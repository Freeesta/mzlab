//! Single scans of CENTROID files read from their source (`sorgente::Scansioni`: the mzML by offset, the scan table of the index giving `start..end`
//! of each `<spectrum>`, or a vendor reader; the whole file is never held): `/api/spectra` (`App.spectra`: the scans `i0..=i1` of a level, each with its own centroids in high resolution, binned and
//! optionally merged by nominal mass otherwise), `/api/scan` (`Item.scan`: one scan as the file has it, with its parent and activation) and
//! `/api/scanavg` (`Item.scan_avg`: the mean of up to 200 scans on cells of 3 ppm or 0.1 Da). `Ok(None)` = Python's (profile scans, a request
//! Python must refuse, a scan the index does not know).

use crate::error::Result;
use crate::idxread::ReadAt;
use crate::iteminfo::{hr_profile, HrProfile};
use crate::json::{round, Out};
use crate::routes::{Query, Routes};
use crate::sorgente::Scansioni;
use crate::specpeaks::merge_unit;
use std::collections::{BTreeMap, HashMap};
use std::hash::{BuildHasherDefault, Hasher};

const MAX_SCANS: i64 = 60;
const MAX_AVERAGE: usize = 200;

impl<R: ReadAt> Routes<R> {
    /// The routes of this module; `None` = not answered here.
    pub fn mzml_route<M: Scansioni + ?Sized>(&mut self, path: &str, q: &Query, mzml: &mut M) -> Result<Option<String>> {
        if !self.has_scans() {
            return Ok(None);
        }
        self.prime_steps(mzml)?;
        self.con_opz(q, |r| r.mzml_route_in(path, q, mzml))
    }

    fn mzml_route_in<M: Scansioni + ?Sized>(&mut self, path: &str, q: &Query, mzml: &mut M) -> Result<Option<String>> {
        match path {
            "/api/spectrum" => match self.spectrum_exact(q, mzml)? {
                Some(a) => Ok(Some(a)),
                None => self.handle(path, q),
            },
            "/api/spectra" => match self.spectra_centroids(q, mzml)? {
                Some(a) => Ok(Some(a)),
                None => self.spectra_profile_exact(q, mzml),
            },
            "/api/scan" => self.scan_route(q, mzml),
            "/api/scanavg" => self.scanavg_route(q, mzml),
            "/api/msntree" => self.msntree_route(q, mzml),
            _ => Ok(None),
        }
    }

    /// `/api/spectra` of a level with centroids only.
    fn spectra_centroids<M: Scansioni + ?Sized>(&mut self, q: &Query, mzml: &mut M) -> Result<Option<String>> {
        let bin = q.num("bin")?.unwrap_or(0.1);
        let Some(p) = self.picked_scans(q, mzml, bin, false)? else { return Ok(None) };
        let mut o = Out::default();
        o.raw("{\"n\":").int(p.n as i64).raw(",\"scans\":[");
        for (k, r) in p.rows.iter().enumerate() {
            o.raw(if k > 0 { "," } else { "" }).raw("{\"i\":").int(r.j as i64).raw(",\"sid\":").int(r.sid);
            o.raw(",\"rt\":").num(round(r.rt, 4)).raw(",\"mode\":\"centroid\",\"mz\":").rounded(&r.mz, p.nd).raw(",\"y\":").rounded(&r.y, 1).raw("}");
        }
        o.raw("]}");
        Ok(Some(o.0))
    }

    /// The scans `i0..=i1` of a level of centroids as `App.spectra` / `App.scanbin` have them (`raw_filt`: with a filter the peaks are not binned).
    pub(crate) fn picked_scans<M: Scansioni + ?Sized>(&mut self, q: &Query, mzml: &mut M, bin: f64, raw_filt: bool) -> Result<Option<Picked>> {
        let (Some(i0), Some(i1)) = (int(q, "i0"), int(q, "i1")) else { return Ok(None) };
        let level = q.level();
        // one pass: the scans of the level and whether any of them (or, with none, any scan of the file) is profile
        let (mut nscans, mut level_profile, mut any_profile) = (0usize, false, false);
        for s in &self.idx.scans {
            any_profile |= s.profile;
            if s.level == level {
                nscans += 1;
                level_profile |= s.profile;
            }
        }
        let pool_has_profile = if nscans > 0 {
            level_profile
        } else {
            any_profile
        };
        let hr = hr_profile(&self.idx.scans, level == 1, q.get("hr") != Some("0"));
        if pool_has_profile || nscans == 0 || !(hr.on || bin > 0.0) {
            return Ok(None);
        }
        let Some(si) = self.idx.section_of(level, self.pol) else { return Ok(None) };
        let precursor = q.num("prec")?.or(q.num("precursor")?);
        let tol = match precursor {
            Some(p) if level > 1 && p != 0.0 && hr.on => (p * hr.ppm * 1e-6).max(1e-3),
            _ => 0.6,
        };
        let filt = q.some("filt").or(q.some("filter"));
        let sel = self.selected(level, si, precursor, tol, filt)?;
        if i0 > i1 || i1 - i0 + 1 > MAX_SCANS || i0 < 0 || i0 as usize >= sel.len() {
            return Ok(None);
        }
        let (ar, scans) = self.idx.arrays_scans(si)?;
        let merge = q.get("merge") == Some("1") && !hr.on;
        let mut rows = Vec::new();
        for j in i0 as usize..=(i1 as usize).min(sel.len() - 1) {
            let (sid, rt) = (ar.scan_ids[sel[j]], ar.rt[sel[j]]);
            let (mut mz, mut y) = mzml.spettro(&scans[sid as usize])?;
            if !hr.on && !mz.is_empty() && !(raw_filt && filt.is_some()) {
                (mz, y) = bin_cells(&mz, &y, bin);
            }
            if merge {
                (mz, y) = merge_unit(&mz, &y);
            }
            rows.push(PickedScan { j, sid, rt, mz, y });
        }
        Ok(Some(Picked { n: sel.len(), nd: if hr.on { hr.dec + 1 } else { 3 }, rows }))
    }

    /// `/api/scan`: one scan with its own centroids and what the file says about it.
    fn scan_route<M: Scansioni + ?Sized>(&mut self, q: &Query, mzml: &mut M) -> Result<Option<String>> {
        let Some(sid) = int(q, "sid").filter(|s| *s >= 0 && (*s as usize) < self.idx.scans.len()) else { return Ok(None) };
        let sid = sid as usize;
        let scans = &self.idx.scans;
        let s = &scans[sid];
        let parent = match s.parent {
            Some(p) if p < 0 || p as usize >= scans.len() => return Ok(None),
            p => p,
        };
        let d = hr_profile(scans, s.level == 1, q.get("hr") != Some("0")).dec + 1;
        let ((mz, y), mobility) = mzml.spettro_mobilita(s)?;
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
        o.raw(",\"filter\":").str(&s.filter).raw(",\"profile\":").raw(if s.profile { "true" } else { "false" });
        // ion mobility, only for a scan that has it: `mob` per peak (when the file gives an array as long as the peaks), `mob_scan` for the whole scan
        if let Some(m) = mobility {
            if let Some(v) = m.values.as_ref().filter(|v| v.len() == mz.len()) {
                o.raw(",\"mob_unit\":").str(m.unit).raw(",\"mob\":").rounded(v, 6);
            }
            if let Some((v, u)) = m.scan {
                o.raw(",\"mob_scan\":").num(round(v, 6)).raw(",\"mob_scan_unit\":").str(u);
            }
        }
        o.raw("}");
        Ok(Some(o.0))
    }

    /// `/api/scanavg`: the scans `sids` (at most 200, all of the level of the first) summed per cell, the intensity divided by the number of scans.
    fn scanavg_route<M: Scansioni + ?Sized>(&mut self, q: &Query, mzml: &mut M) -> Result<Option<String>> {
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
        let cell = Cell::of(&hr);
        // sums in the order of the concatenated arrays (np.bincount), the cells ordered (np.unique)
        let mut cells = Cells::default();
        for &sid in &sids {
            let (mz, y) = mzml.spettro(&self.idx.scans[sid as usize])?;
            for (m, v) in mz.iter().zip(&y) {
                cells.add(cell.at(*m), *v, *m);
            }
        }
        let n = sids.len() as f64;
        let rt = sids.iter().map(|&s| self.idx.scans[s as usize].rt).sum::<f64>() / n;
        let div = if self.msn.opz.somma { 1.0 } else { n };
        let (mz, y) = cells.finish(div);
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

/// What `picked_scans` found: the number of scans of the kind, the decimals of the m/z and the scans asked for.
pub(crate) struct Picked {
    pub n: usize,
    pub nd: usize,
    pub rows: Vec<PickedScan>,
}

pub(crate) struct PickedScan {
    pub j: usize,
    pub sid: i64,
    pub rt: f64,
    pub mz: Vec<f64>,
    pub y: Vec<f64>,
}

/// Cells of 3 ppm in high resolution, of 0.1 Da otherwise (`Item.scan_avg`).
enum Cell {
    Ppm(f64),
    Da,
}

impl Cell {
    fn of(hr: &HrProfile) -> Cell {
        if hr.on {
            Cell::Ppm((3e-6f64).ln_1p())
        } else {
            Cell::Da
        }
    }
    fn at(&self, m: f64) -> i64 {
        match self {
            Cell::Ppm(w) => (m.max(1e-9).ln() / w).floor() as i64,
            Cell::Da => (m / 0.1).floor() as i64,
        }
    }
}

/// Hashes an `i64` cell number with one multiplication: the keys are small integers and the default hasher would cost more than the sum they index.
#[derive(Default)]
struct CellHash(u64);

impl Hasher for CellHash {
    fn write(&mut self, bytes: &[u8]) {
        for b in bytes {
            self.0 = (self.0 ^ u64::from(*b)).wrapping_mul(0x100000001b3);
        }
    }
    fn write_i64(&mut self, k: i64) {
        let h = (k as u64).wrapping_mul(0x9E37_79B9_7F4A_7C15);
        self.0 = h ^ (h >> 32);
    }
    fn finish(&self) -> u64 {
        self.0
    }
}

/// The sums per cell of the peaks added one by one (intensity, intensity times m/z), each cell summed in the order its peaks come, read back with the cells ordered.
#[derive(Default, Clone)]
struct Cells {
    slot: HashMap<i64, usize, BuildHasherDefault<CellHash>>,
    keys: Vec<i64>,
    sums: Vec<(f64, f64)>,
}

impl Cells {
    fn add(&mut self, key: i64, v: f64, m: f64) {
        let at = *self.slot.entry(key).or_insert_with(|| {
            self.keys.push(key);
            self.sums.push((0.0, 0.0));
            self.sums.len() - 1
        });
        let e = &mut self.sums[at];
        e.0 += v;
        e.1 += v * m;
    }
    /// (m/z weighted by intensity, intensity divided by `div`) per cell, the cells in increasing order.
    fn finish(mut self, div: f64) -> (Vec<f64>, Vec<f64>) {
        let mut order: Vec<(i64, usize)> = self.keys.iter().copied().zip(0..).collect();
        crate::sortc::slice_by_key(&mut order, |k| k.0);
        let (mut mz, mut y) = (
            Vec::with_capacity(order.len()),
            Vec::with_capacity(order.len()),
        );
        for (_, at) in order {
            let (sy, smy) = std::mem::take(&mut self.sums[at]);
            mz.push(smy / sy.max(1e-12));
            y.push(sy / div);
        }
        (mz, y)
    }
}

fn int(q: &Query, k: &str) -> Option<i64> {
    q.some(k)?.trim().parse().ok()
}

/// The peaks of one scan merged per bin of `bin` Da (`Item.scans`), m/z weighted by intensity.
fn bin_cells(mz: &[f64], y: &[f64], bin: f64) -> (Vec<f64>, Vec<f64>) {
    let key = |m: f64| (m / bin).floor() as i64;
    let mut out = (Vec::new(), Vec::new());
    let mut push = |(sy, smy): (f64, f64)| {
        out.0.push(smy / sy.max(1e-12));
        out.1.push(sy);
    };
    if bin > 0.0 && mz.windows(2).all(|w| w[0] <= w[1]) {
        // ascending m/z: the cells come in order, one after the other, each summed in the order of its peaks
        let mut cur: Option<(i64, (f64, f64))> = None;
        for (m, v) in mz.iter().zip(y) {
            let k = key(*m);
            match &mut cur {
                Some((ck, e)) if *ck == k => {
                    e.0 += v;
                    e.1 += v * m;
                }
                _ => {
                    if let Some((_, e)) = cur.take() {
                        push(e);
                    }
                    cur = Some((k, (0.0 + v, 0.0 + v * m)));
                }
            }
        }
        if let Some((_, e)) = cur {
            push(e);
        }
        return out;
    }
    let mut cells: BTreeMap<i64, (f64, f64)> = BTreeMap::new();
    for (m, v) in mz.iter().zip(y) {
        let e = cells.entry(key(*m)).or_insert((0.0, 0.0));
        e.0 += v;
        e.1 += v * m;
    }
    cells.values().for_each(|e| push(*e));
    out
}

#[cfg(test)]
mod tests;
