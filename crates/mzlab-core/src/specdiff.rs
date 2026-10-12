//! `/api/spectrum` with a background (`bgk`): the mean spectrum of a window minus the mean spectrum of another window, of the same file or of
//! another one (`App.spectrum` with `bg`, `Item.spectrum`, `differenza.subtract_spectrum`). The answer is drawn as lines (`diff`), never as a
//! profile. `Ok(None)` = Python's (a request it must refuse, a profile level the index cannot answer).

use crate::error::Result;
use crate::idxread::ReadAt;
use crate::iteminfo::hr_profile;
use crate::json::Out;
use crate::profspec::{rint, window_tail};
use crate::routes::{Query, Routes};
use crate::specpeaks::nominal;
use crate::spectrum::Binned;
use std::collections::BTreeMap;

/// What a request with a background asks (everything the file of the background needs from the main one).
pub struct DiffReq {
    level: u8,
    window: (f64, f64),
    precursor: Option<f64>,
    filt: Option<String>,
    bin: f64,
    hr_flag: bool,
    tol: f64,
    bg_k: usize,
    bg_window: (f64, f64),
    factor: f64,
    ppm: f64,
    clip: bool,
}

impl DiffReq {
    /// Item number of the background file (`bgk`).
    pub fn bg_k(&self) -> usize {
        self.bg_k
    }
}

/// The mean spectrum of a window and whether its cells were those of high resolution.
pub struct Mean {
    b: Option<Binned>,
    hr_on: bool,
}

impl<R: ReadAt> Routes<R> {
    /// `None` = not this case (no background asked, or a request Python must answer).
    pub fn diff_request(&mut self, q: &Query) -> Result<Option<DiffReq>> {
        let Some(bg_k) = q.some("bgk").and_then(|v| v.trim().parse::<usize>().ok()) else { return Ok(None) };
        let (Some(rt0), Some(rt1), Some(b0), Some(b1)) = (q.num("rt0")?, q.num("rt1")?, q.num("bgrt0")?, q.num("bgrt1")?) else { return Ok(None) };
        let level = q.level();
        let hr_flag = q.get("hr") != Some("0");
        let precursor = q.num("precursor")?;
        let (tol, _) = self.window_tol(level, precursor, hr_flag);
        Ok(Some(DiffReq {
            level,
            window: (rt0.min(rt1), rt0.max(rt1)),
            precursor,
            filt: q.some("filt").map(String::from),
            bin: q.num("bin")?.unwrap_or(0.1),
            hr_flag,
            tol,
            bg_k,
            bg_window: (b0.min(b1), b0.max(b1)),
            factor: q.num("bgf")?.unwrap_or(1.0),
            ppm: q.num("bgtol")?.unwrap_or(5.0),
            clip: q.get("bgclip").unwrap_or("1") == "1",
        }))
    }

    /// `Item.ptol` and whether the centroids of the level are read as high resolution (`Item.hr_on`).
    fn window_tol(&self, level: u8, precursor: Option<f64>, hr_flag: bool) -> (f64, bool) {
        let scans = &self.idx.scans;
        let profile = scans.iter().filter(|s| s.level == level).any(|s| s.profile);
        let hr = hr_profile(scans, level == 1, hr_flag);
        let on = hr.on && !profile;
        match precursor {
            Some(p) if level > 1 && p != 0.0 && on => ((p * hr.ppm * 1e-6).max(1e-3), on),
            _ => (0.6, on),
        }
    }

    /// `Item._binned` of a window with the tolerance on the precursor given: `None` = Python's.
    pub fn mean_window(&mut self, r: &DiffReq, (lo, hi): (f64, f64), filt: Option<&str>) -> Result<Option<Mean>> {
        let level = r.level;
        let pool_profile = {
            let of: Vec<_> = self.idx.scans.iter().filter(|s| s.level == level).collect();
            if of.is_empty() { self.idx.scans.iter().any(|s| s.profile) } else { of.iter().any(|s| s.profile) }
        };
        let args = (r.precursor, r.tol);
        if pool_profile {
            let Some((_, step)) = self.profile_level(level)? else { return Ok(None) };
            return Ok(Some(Mean { b: self.binned_cells(level, lo, hi, args, filt, rint(step))?, hr_on: false }));
        }
        let (_, hr_on) = self.window_tol(level, r.precursor, r.hr_flag);
        if hr_on {
            let cell = (3e-6f64).ln_1p();
            return Ok(Some(Mean { b: self.binned_cells(level, lo, hi, args, filt, |m| (m.max(1e-9).ln() / cell).floor() as i64)?, hr_on }));
        }
        if !(r.bin > 0.0) {
            return Ok(None);
        }
        let bin = r.bin;
        Ok(Some(Mean { b: self.binned_cells(level, lo, hi, args, filt, |m| (m / bin).floor() as i64)?, hr_on }))
    }
}

/// The answer for `req`: `bg` = the routes of the background file (`None` = the file itself).
pub fn spectrum_diff<R: ReadAt>(main: &mut Routes<R>, bg: Option<&mut Routes<R>>, req: &DiffReq) -> Result<Option<String>> {
    let Some(m) = main.mean_window(req, req.window, req.filt.as_deref())? else { return Ok(None) };
    let (mut mz, mut y) = m.b.as_ref().map_or((vec![], vec![]), |b| (b.mz.clone(), b.y.clone()));
    if !mz.is_empty() {
        let r = match bg {
            Some(other) => other.mean_window(req, req.bg_window, None)?,
            None => main.mean_window(req, req.bg_window, None)?,
        };
        let Some(r) = r else { return Ok(None) };
        if let Some(rb) = r.b.filter(|b| b.n > 0 && !b.mz.is_empty()) {
            (mz, y) = subtract(&mz, &y, (&rb.mz, &rb.y), (req.factor, !m.hr_on, req.ppm, req.clip));
        }
    }
    let mut o = Out::default();
    o.raw("{\"mode\":\"centroid\",\"diff\":true,\"mz\":").rounded(&mz, 4).raw(",\"y\":").rounded(&y, 1);
    match &m.b {
        Some(b) => {
            o.raw(",\"scans\":").int(b.n as i64);
            window_tail(&mut o, b);
        }
        None => {
            o.raw(",\"scans\":0,\"i0\":null,\"i1\":null,\"n\":0");
        }
    }
    o.raw("}");
    Ok(Some(o.0))
}

/// `differenza.subtract_spectrum` with `(factor, unit, ppm, clip)`: lines only where the difference is not zero (and positive with `clip`), by m/z.
fn subtract(mz: &[f64], y: &[f64], (rmz, ry): (&[f64], &[f64]), (factor, unit, ppm, clip): (f64, bool, f64, bool)) -> (Vec<f64>, Vec<f64>) {
    let ry: Vec<f64> = ry.iter().map(|v| v * factor).collect();
    let (om, oy) = if unit { subtract_unit(mz, y, rmz, &ry) } else { subtract_hr(mz, y, rmz, &ry, ppm) };
    let mut rows: Vec<(f64, f64)> = om.into_iter().zip(oy).filter(|(_, v)| *v != 0.0 && (!clip || *v > 0.0)).collect();
    rows.sort_by(|a, b| a.0.total_cmp(&b.0));
    rows.into_iter().unzip()
}

/// The strongest line of each nominal mass (first one on a tie) of a list.
fn strongest(mz: &[f64], y: &[f64]) -> BTreeMap<i64, (f64, f64)> {
    let mut m: BTreeMap<i64, (f64, f64)> = BTreeMap::new();
    for (&x, &v) in mz.iter().zip(y) {
        let e = m.entry(nominal(x)).or_insert((x, v));
        if v > e.1 {
            *e = (x, v);
        }
    }
    m
}

/// Low resolution: a line of the sample and one of the reference on the same nominal mass cancel; the m/z kept is that of the stronger one.
fn subtract_unit(mz: &[f64], y: &[f64], rmz: &[f64], ry: &[f64]) -> (Vec<f64>, Vec<f64>) {
    let (a, b) = (strongest(mz, y), strongest(rmz, ry));
    let cells: std::collections::BTreeSet<i64> = a.keys().chain(b.keys()).copied().collect();
    let (mut om, mut oy) = (vec![], vec![]);
    for c in cells {
        let (ma, ya) = a.get(&c).copied().unzip();
        let (mb, yb) = b.get(&c).copied().unzip();
        let (va, vb) = (ya.unwrap_or(0.0), yb.unwrap_or(0.0));
        om.push(if ma.is_some() && (mb.is_none() || va >= vb) { ma.unwrap() } else { mb.unwrap() });
        oy.push(va - vb);
    }
    (om, oy)
}

/// High resolution: each sample line minus the strongest reference line within +-ppm, then the reference lines no sample line covers.
fn subtract_hr(mz: &[f64], y: &[f64], rmz: &[f64], ry: &[f64], ppm: f64) -> (Vec<f64>, Vec<f64>) {
    let mut o: Vec<usize> = (0..rmz.len()).collect();
    o.sort_by(|a, b| rmz[*a].total_cmp(&rmz[*b]));
    let (rm, rv): (Vec<f64>, Vec<f64>) = o.iter().map(|&i| (rmz[i], ry[i])).unzip();
    let mut covered = vec![false; rm.len()];
    let mut oy = Vec::with_capacity(mz.len() + rm.len());
    for (&m, &v) in mz.iter().zip(y) {
        let w = m * ppm * 1e-6;
        let (lo, hi) = (rm.partition_point(|x| *x < m - w), rm.partition_point(|x| *x <= m + w));
        let top = rv[lo.min(hi)..hi].iter().copied().fold(None, |a: Option<f64>, x| Some(a.map_or(x, |a| a.max(x))));
        covered[lo.min(hi)..hi].iter_mut().for_each(|c| *c = true);
        oy.push(v - top.unwrap_or(0.0));
    }
    let mut om = mz.to_vec();
    for (i, c) in covered.iter().enumerate() {
        if !c {
            om.push(rm[i]);
            oy.push(-rv[i]);
        }
    }
    (om, oy)
}
