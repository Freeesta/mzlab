//! Answers to the `api/...` routes that only need the `.mzidx` (scan table and per-scan arrays, windows of the peak arrays). The text of
//! every answer is the JSON that `mzlab/api.py` gives for the same query. `Ok(None)` = "this query is not mine, Python answers it"
//! (MRM files, parts of mixed files, unknown parameters): never a wrong answer.

use crate::error::{Error, Result};
use crate::idxread::{Index, ReadAt};
use crate::json::Out;

/// `a=1&b=x%20y` as a list of pairs.
#[derive(Debug, Default)]
pub struct Query(pub Vec<(String, String)>);

impl Query {
    pub fn parse(s: &str) -> Query {
        let s = s.strip_prefix('?').unwrap_or(s);
        Query(
            s.split('&')
                .filter(|p| !p.is_empty())
                .map(|p| {
                    let (k, v) = p.split_once('=').unwrap_or((p, ""));
                    (decode(k), decode(v))
                })
                .collect(),
        )
    }
    /// Python's `q.get(k)`: the value, even if empty.
    pub fn get(&self, k: &str) -> Option<&str> {
        self.0.iter().find(|(n, _)| n == k).map(|(_, v)| v.as_str())
    }
    /// A value that counts as present in Python's `if q.get(k)`: not empty.
    pub fn some(&self, k: &str) -> Option<&str> {
        self.get(k).filter(|v| !v.is_empty())
    }
    pub fn num(&self, k: &str) -> Result<Option<f64>> {
        match self.some(k) {
            None => Ok(None),
            Some(v) => v
                .parse()
                .map(Some)
                .map_err(|_| Error::new("err.request.badNumber").with("name", k)),
        }
    }
    pub(crate) fn level(&self) -> u8 {
        self.get("level").and_then(|v| v.parse().ok()).unwrap_or(1)
    }
}

fn decode(s: &str) -> String {
    let b = s.as_bytes();
    let hex = |c: u8| (c as char).to_digit(16);
    let mut out = Vec::with_capacity(b.len());
    let mut i = 0;
    while i < b.len() {
        match b[i] {
            b'+' => out.push(b' '),
            b'%' if i + 2 < b.len() && hex(b[i + 1]).is_some() && hex(b[i + 2]).is_some() => {
                out.push((hex(b[i + 1]).unwrap() * 16 + hex(b[i + 2]).unwrap()) as u8);
                i += 2;
            }
            c => out.push(c),
        }
        i += 1;
    }
    String::from_utf8_lossy(&out).into_owned()
}

/// The index of one whole file (not a part of a mixed file) and the routes it answers.
pub struct Routes<R: ReadAt> {
    pub idx: Index<R>,
    /// The last whole-file maps of the session grid (the grid, the cells): a difference asks for the same map of a file up to three times.
    pub(crate) maps: Vec<(String, std::sync::Arc<Vec<f32>>)>,
    /// The spacing of the points of a profile level, worked out once (`Item._step`).
    pub(crate) steps: Vec<(u8, Option<f64>)>,
}

impl<R: ReadAt> Routes<R> {
    pub fn new(idx: Index<R>) -> Self {
        Routes { idx, maps: Vec::new(), steps: Vec::new() }
    }

    /// True when the file has scans (an MRM file has only chromatograms: its routes are Python's).
    pub fn has_scans(&self) -> bool {
        !self.idx.scans.is_empty()
    }

    /// Answer for a path such as `/api/filters` and its query; `None` = not answered here.
    pub fn handle(&mut self, path: &str, q: &Query) -> Result<Option<String>> {
        if !self.has_scans() {
            return Ok(None);
        }
        match path {
            "/api/chrom" => self.chrom(q),
            "/api/filters" => self.filters().map(Some),
            "/api/scanlist" => self.scanlist(q),
            "/api/nearest_scan" => self.nearest_scan(q),
            "/api/scan_axis" => self.scan_axis(q),
            "/api/ms2tr" => self.ms2_transitions().map(Some),
            "/api/spectrum" => self.spectrum(q),
            "/api/dda" => self.dda(q),
            "/api/spectra" => self.spectra_profile(q),
            _ => Ok(None),
        }
    }

    /// `/api/scaninfo` for scan `sid`, reading the header of that scan from the mzML.
    pub fn scaninfo<M: ReadAt>(&self, sid: usize, mzml: &mut M) -> Result<String> {
        crate::scaninfo::scaninfo(&self.idx.scans, sid, mzml)
    }

    /// The trace of one file: `{"rt":[...],"y":[...]}` (the multi-file wrapper of `api/xic` is the page's job).
    pub fn xic(&mut self, mz: f64, tol: f64, level: u8) -> Result<String> {
        let (rt, y) = self.xic_raw(mz, tol, level)?;
        Ok(trace(&rt, &y))
    }

    /// The XIC of one level as the arrays Python works on (not rounded): the reference difference subtracts these.
    pub fn xic_raw(&mut self, mz: f64, tol: f64, level: u8) -> Result<(Vec<f64>, Vec<f64>)> {
        let Some(si) = self.idx.section_of(level, None) else {
            return Ok((Vec::new(), Vec::new()));
        };
        let (lo, hi) = (mz - tol, mz + tol);
        let a = self.idx.search(si, lo, false)?;
        let b = self.idx.search(si, hi, true)?;
        let n = self.idx.sections[si].n_scans;
        let y = if b <= a {
            vec![0.0; n]
        } else {
            self.idx.sum_window(si, a, b)?
        };
        let rt = self.idx.arrays(si)?.rt.clone();
        Ok((rt, y))
    }

    fn chrom(&mut self, q: &Query) -> Result<Option<String>> {
        if q.some("ref").is_some() {
            return Ok(None);
        }
        Ok(self.chrom_raw(q)?.map(|(rt, y)| trace(&rt, &y)))
    }

    /// The TIC or base peak chromatogram of one file (the window, the scan type and the precursor of the query applied), not rounded;
    /// `None` = another kind of trace (Python's).
    pub fn chrom_raw(&mut self, q: &Query) -> Result<Option<(Vec<f64>, Vec<f64>)>> {
        let kind = q.get("kind").unwrap_or("tic");
        if kind != "tic" && kind != "bpc" {
            return Ok(None);
        }
        let level = q.level();
        let (mz0, mz1, prec) = (q.num("mz0")?, q.num("mz1")?, q.num("prec")?);
        let filt = q.some("filt");
        let Some(si) = self.idx.section_of(level, None) else {
            return Ok(Some((Vec::new(), Vec::new())));
        };
        let y = if mz0.is_some() || mz1.is_some() {
            let a = match mz0 {
                None => 0,
                Some(v) => self.idx.search(si, v, false)?,
            };
            let b = match mz1 {
                None => self.idx.sections[si].n_peaks,
                Some(v) => self.idx.search(si, v, true)?,
            };
            let b = b.max(a);
            if kind == "tic" {
                self.idx.sum_window(si, a, b)?
            } else {
                self.idx.max_window(si, a, b)?
            }
        } else {
            let ar = self.idx.arrays(si)?;
            if kind == "tic" {
                ar.sumint.clone()
            } else {
                ar.bp_inten.clone()
            }
        };
        let (rt, ids) = {
            let ar = self.idx.arrays(si)?;
            (ar.rt.clone(), ar.scan_ids.clone())
        };
        let mut keep = vec![true; rt.len()];
        if let Some(f) = filt {
            for (j, &id) in ids.iter().enumerate() {
                let s = &self.idx.scans[id as usize];
                keep[j] = s.key() == f || s.filter == f;
            }
        }
        if let (Some(p), true) = (prec, level > 1) {
            for (j, &id) in ids.iter().enumerate() {
                if keep[j] && (self.idx.scans[id as usize].prec(-1.0) - p).abs() > 0.6 {
                    keep[j] = false;
                }
            }
        }
        let pick = |v: &[f64]| -> Vec<f64> {
            v.iter()
                .zip(&keep)
                .filter(|(_, k)| **k)
                .map(|(x, _)| *x)
                .collect()
        };
        Ok(Some((pick(&rt), pick(&y))))
    }

    pub fn filters(&mut self) -> Result<String> {
        struct G {
            key: String,
            level: u8,
            n: u64,
            rt0: f64,
            rt1: f64,
            pol: i8,
        }
        let mut g: Vec<G> = Vec::new();
        let mut by: std::collections::HashMap<String, usize> = std::collections::HashMap::new();
        for s in &self.idx.scans {
            let k = s.key();
            match by.get(&k) {
                Some(&i) => {
                    g[i].n += 1;
                    g[i].rt0 = g[i].rt0.min(s.rt);
                    g[i].rt1 = g[i].rt1.max(s.rt);
                }
                None => {
                    by.insert(k.clone(), g.len());
                    g.push(G {
                        key: k,
                        level: s.level,
                        n: 1,
                        rt0: s.rt,
                        rt1: s.rt,
                        pol: s.polarity,
                    });
                }
            }
        }
        g.sort_by(|a, b| {
            a.level
                .cmp(&b.level)
                .then(b.n.cmp(&a.n))
                .then_with(|| a.key.cmp(&b.key))
        });
        let mut o = Out::default();
        o.raw("{\"filters\":[");
        for (i, x) in g.iter().enumerate() {
            if i > 0 {
                o.raw(",");
            }
            o.raw("{\"key\":")
                .str(&x.key)
                .raw(",\"level\":")
                .int(x.level as i64)
                .raw(",\"n\":")
                .int(x.n as i64);
            o.raw(",\"rt0\":")
                .num(crate::json::round(x.rt0, 4))
                .raw(",\"rt1\":")
                .num(crate::json::round(x.rt1, 4));
            o.raw(",\"pol\":").int(x.pol as i64).raw("}");
        }
        o.raw("]}");
        Ok(o.0)
    }

    fn scanlist(&mut self, q: &Query) -> Result<Option<String>> {
        let level = q.level();
        let filt = q.some("filt");
        let mut o = Out::default();
        let Some(si) = self.idx.section_of(level, None) else {
            o.raw("{\"level\":").int(level as i64);
            o.raw(",\"n\":0,\"sid\":[],\"no\":[],\"rt\":[],\"tic\":[],\"prec\":[],\"bpmz\":[],\"bpint\":[]}");
            return Ok(Some(o.0));
        };
        let ar = self.idx.arrays(si)?.clone();
        let sel: Vec<usize> = (0..ar.scan_ids.len())
            .filter(|&j| match filt {
                None => true,
                Some(f) => {
                    let s = &self.idx.scans[ar.scan_ids[j] as usize];
                    s.key() == f || s.filter == f
                }
            })
            .collect();
        let sc = |j: usize| &self.idx.scans[ar.scan_ids[j] as usize];
        o.raw("{\"level\":")
            .int(level as i64)
            .raw(",\"n\":")
            .int(sel.len() as i64);
        o.raw(",\"sid\":[");
        for (i, &j) in sel.iter().enumerate() {
            o.raw(if i > 0 { "," } else { "" }).int(ar.scan_ids[j]);
        }
        o.raw("],\"no\":[");
        for (i, &j) in sel.iter().enumerate() {
            o.raw(if i > 0 { "," } else { "" })
                .int(sc(j).number(ar.scan_ids[j] as usize));
        }
        o.raw("],\"rt\":");
        o.rounded(&sel.iter().map(|&j| sc(j).rt).collect::<Vec<_>>(), 4);
        o.raw(",\"tic\":");
        o.floats(&sel.iter().map(|&j| sc(j).tic).collect::<Vec<_>>());
        o.raw(",\"prec\":[");
        for (i, &j) in sel.iter().enumerate() {
            o.raw(if i > 0 { "," } else { "" });
            let p = sc(j).prec(f64::NAN);
            o.opt(if p.is_nan() {
                None
            } else {
                Some(crate::json::round(p, 5))
            });
        }
        o.raw("],\"bpmz\":");
        o.rounded(&sel.iter().map(|&j| ar.bp_mz[j]).collect::<Vec<_>>(), 5);
        o.raw(",\"bpint\":");
        o.rounded(&sel.iter().map(|&j| ar.bp_inten[j]).collect::<Vec<_>>(), 1);
        o.raw("}");
        Ok(Some(o.0))
    }

    fn nearest_scan(&mut self, q: &Query) -> Result<Option<String>> {
        let Some(rt) = q.num("rt")? else {
            return Ok(None);
        };
        let level = q.level();
        let filter = q.get("filter");
        let precursor = q.num("precursor")?;
        let Some(si) = self.idx.section_of(level, None) else {
            return Ok(None);
        };
        let ar = self.idx.arrays(si)?.clone();
        let mut best: Option<(f64, usize)> = None;
        for &id in &ar.scan_ids {
            let s = &self.idx.scans[id as usize];
            let ok = match (filter, precursor) {
                (Some(f), _) => s.filter == f || s.key() == f,
                (None, Some(p)) => (s.prec(-100.0) - p).abs() <= 0.6,
                (None, None) => true,
            };
            if ok {
                let d = (s.rt - rt).abs();
                if best.is_none_or(|(b, _)| d < b) {
                    best = Some((d, id as usize));
                }
            }
        }
        let Some((_, id)) = best else { return Ok(None) };
        let s = &self.idx.scans[id];
        let mut o = Out::default();
        o.raw("{\"rt\":")
            .num(s.rt)
            .raw(",\"sid\":")
            .int(id as i64)
            .raw(",\"filter\":")
            .str(&s.filter)
            .raw("}");
        Ok(Some(o.0))
    }
}

fn trace(rt: &[f64], y: &[f64]) -> String {
    let mut o = Out::default();
    o.raw("{\"rt\":")
        .rounded(rt, 4)
        .raw(",\"y\":")
        .rounded(y, 1)
        .raw("}");
    o.0
}
