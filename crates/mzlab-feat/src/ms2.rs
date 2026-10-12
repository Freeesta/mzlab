//! MS2 scans of a peak, with the purity of the precursor.
//!
//! An MS2 scan belongs to a peak when the MS1 scan it was triggered from (`Scan::parent`) lies between the edges of the peak (not only at
//! the apex) and the m/z of the peak is inside its isolation window; a co-isolated ion is linked as well, with its own share of the
//! window. Purity = intensity of the peak in that MS1 scan (peaks within `tol_ppm` of its m/z) over the total intensity of the MS1 scan
//! inside the isolation window. Below `weak` the MS2 is weak evidence (it is not dropped). When the index carries no isolation window the
//! scan gets `default_half_window` around its precursor and the peak is marked `FLAG_PURITY_EST`; the format of the index does not change.
//! The links are a CSR over the rows of the table: scans of the peak `start[i]..start[i + 1]` in `scan` (positions in `Index::scans`),
//! in scan order, with the purity of each in `purity`.

use crate::sortc;
use mzlab_core::error::Result;
use mzlab_core::idxread::{Index, ReadAt};

use crate::sweep::windows;
use crate::table::{FileFeatures, FLAG_PURITY_EST};
use crate::trace::{reserve, Params};

#[derive(Debug, Clone)]
pub struct Ms2Params {
    /// Half width of the isolation window assumed when the index has none, m/z.
    pub default_half_window: f64,
    pub tol_ppm: f64,
    /// Purity below which an MS2 is weak evidence.
    pub weak: f32,
}

impl Default for Ms2Params {
    fn default() -> Self {
        Ms2Params {
            default_half_window: 0.6,
            tol_ppm: 6.0,
            weak: 0.5,
        }
    }
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct Ms2Links {
    pub start: Vec<u32>,
    pub scan: Vec<u32>,
    pub purity: Vec<f32>,
    /// Some scans used the default isolation window.
    pub default_window: bool,
}

impl Ms2Links {
    /// Scans and purities linked to row `i`.
    pub fn of(&self, i: usize) -> (&[u32], &[f32]) {
        let (a, b) = (self.start[i] as usize, self.start[i + 1] as usize);
        (&self.scan[a..b], &self.purity[a..b])
    }
}

/// An MS2 scan of the polarity, triggered from an MS1 scan of the section.
struct Cand {
    scan: u32,
    ppos: u32,
    lo: f64,
    hi: f64,
    est: bool,
}

fn candidates<R: ReadAt>(
    index: &Index<R>,
    ids: &[i64],
    pol: Option<i8>,
    q: &Ms2Params,
) -> Result<Vec<Cand>> {
    let mut pos_of = Vec::new();
    reserve(&mut pos_of, index.scans.len())?;
    pos_of.resize(index.scans.len(), u32::MAX);
    for (k, &g) in ids.iter().enumerate() {
        if g >= 0 && (g as usize) < pos_of.len() {
            pos_of[g as usize] = k as u32;
        }
    }
    let mut out = Vec::new();
    for (g, s) in index.scans.iter().enumerate() {
        let ok_pol = match pol {
            None => true,
            Some(1) => s.polarity >= 0,
            Some(x) => s.polarity == x,
        };
        let ppos = s
            .parent
            .and_then(|p| pos_of.get(p as usize).copied())
            .unwrap_or(u32::MAX);
        if s.level != 2 || !ok_pol || ppos == u32::MAX {
            continue;
        }
        let (lo, hi, est) = match (s.iso, s.precursor) {
            (Some((a, b)), _) => (a, b, false),
            (None, Some(c)) if c != 0.0 => {
                (c - q.default_half_window, c + q.default_half_window, true)
            }
            _ => continue,
        };
        reserve(&mut out, 1)?;
        out.push(Cand {
            scan: g as u32,
            ppos,
            lo,
            hi,
            est,
        });
    }
    Ok(out)
}

/// (feature, candidate) pairs: the peak spans the MS1 scan of the candidate and its m/z is inside the window. Sorted by candidate.
fn pairs(f: &FileFeatures, cands: &[Cand], nscan: usize) -> Result<Vec<(u32, u32)>> {
    let mut start = vec![0u32; nscan + 2];
    for c in cands {
        start[c.ppos as usize + 1] += 1;
    }
    for k in 0..=nscan {
        start[k + 1] += start[k];
    }
    let mut fill = start.clone();
    let mut by_pos = vec![0u32; cands.len()];
    for (ci, c) in cands.iter().enumerate() {
        by_pos[fill[c.ppos as usize] as usize] = ci as u32;
        fill[c.ppos as usize] += 1;
    }
    let mut out = Vec::new();
    for i in 0..f.len() {
        for p in f.scan_lo[i] as usize..=(f.scan_hi[i] as usize).min(nscan - 1) {
            for &ci in &by_pos[start[p] as usize..start[p + 1] as usize] {
                let c = &cands[ci as usize];
                if f.mz[i] >= c.lo && f.mz[i] <= c.hi {
                    reserve(&mut out, 1)?;
                    out.push((ci, i as u32));
                }
            }
        }
    }
    sortc::by_key(&mut out, |&(c, _)| c);
    Ok(out)
}

/// Purities of the pairs `(candidate, feature)` (sorted by candidate): `(feature, scan, purity, estimated)` of those with signal.
/// The isolation windows are read in one pass over the peak table, in order of their lower bound.
fn purities<R: ReadAt>(
    index: &mut Index<R>,
    si: usize,
    f: &FileFeatures,
    cands: &[Cand],
    prs: &[(u32, u32)],
    q: &Ms2Params,
    max_peaks: usize,
) -> Result<Vec<(u32, u32, f32, bool)>> {
    // the candidates that have pairs, ascending, with the range of their pairs
    let mut used: Vec<(u32, usize, usize)> = Vec::new();
    let mut k = 0;
    while k < prs.len() {
        let e = k + prs[k..].iter().take_while(|p| p.0 == prs[k].0).count();
        reserve(&mut used, 1)?;
        used.push((prs[k].0, k, e));
        k = e;
    }
    sortc::by(&mut used, |a, b| {
        let (ca, cb) = (&cands[a.0 as usize], &cands[b.0 as usize]);
        ca.lo.total_cmp(&cb.lo).then(a.0.cmp(&b.0))
    });
    let ranges: Vec<(f64, f64)> = used
        .iter()
        .map(|u| (cands[u.0 as usize].lo, cands[u.0 as usize].hi))
        .collect();
    let mut out = Vec::new();
    windows(index, si, &ranges, max_peaks, |k, a, b, slab| {
        let (cid, from, to) = used[k];
        let c = &cands[cid as usize];
        let win: Vec<(f64, f64)> = (a..b)
            .filter(|&j| slab.pos[j] == c.ppos)
            .map(|j| (slab.mz[j], slab.it[j]))
            .collect();
        let total: f64 = win.iter().map(|w| w.1).sum();
        for &(_, fi) in &prs[from..to] {
            let i = fi as usize;
            let tol = f.mz[i] * q.tol_ppm * 1e-6;
            let own: f64 = win
                .iter()
                .filter(|w| (w.0 - f.mz[i]).abs() <= tol)
                .map(|w| w.1)
                .sum();
            if own > 0.0 && total > 0.0 {
                reserve(&mut out, 1)?;
                out.push((fi, c.scan, (own / total) as f32, c.est));
            }
        }
        Ok(())
    })?;
    Ok(out)
}

/// Link the MS2 scans to the peaks of `f` and fill `ms2_n`, `purity` and `FLAG_PURITY_EST`.
pub fn link<R: ReadAt>(
    index: &mut Index<R>,
    f: &mut FileFeatures,
    p: &Params,
    q: &Ms2Params,
) -> Result<Ms2Links> {
    let n = f.len();
    let mut links = Ms2Links {
        start: vec![0; n + 1],
        ..Default::default()
    };
    let Some(si) = index.section_of(1, p.polarity) else {
        return Ok(links);
    };
    let ids = index.arrays(si)?.scan_ids.clone();
    let cands = candidates(index, &ids, p.polarity, q)?;
    let prs = pairs(f, &cands, ids.len())?;
    let max_peaks = (p.budget / 24).clamp(1024, crate::trace::SLAB_PEAKS);
    let mut tri = purities(index, si, f, &cands, &prs, q, max_peaks)?;
    sortc::by(&mut tri, |a, b| a.0.cmp(&b.0).then(a.1.cmp(&b.1)));
    for &(i, scan, pur, est) in &tri {
        let i = i as usize;
        links.start[i + 1] += 1;
        links.scan.push(scan);
        links.purity.push(pur);
        links.default_window |= est;
        f.ms2_n[i] = f.ms2_n[i].saturating_add(1);
        if pur > f.purity[i] {
            f.purity[i] = pur;
            if est {
                f.flags[i] |= FLAG_PURITY_EST;
            } else {
                f.flags[i] &= !FLAG_PURITY_EST;
            }
        }
    }
    for i in 0..n {
        links.start[i + 1] += links.start[i];
    }
    Ok(links)
}
