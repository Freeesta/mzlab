//! MS2 spectra of the rows that need them: the scan with the highest purity linked to the peak (the first one on a tie), its strongest peaks.
//!
//! One pass over the peak table of the MS2 section keeps the peaks of the wanted scans only.

use crate::sortc;
use mzlab_core::error::Result;
use mzlab_core::idxread::{Index, ReadAt};

use super::evidence::{Rules, Spec};
use crate::ms2::Ms2Links;
use crate::table::FileFeatures;
use crate::trace::Params;

const NONE: u32 = u32::MAX;

#[derive(Debug, Clone, Default)]
pub struct Spectra {
    slot: Vec<u32>,
    specs: Vec<Spec>,
}

impl Spectra {
    pub fn empty(n_rows: usize) -> Self {
        Spectra {
            slot: vec![NONE; n_rows],
            specs: Vec::new(),
        }
    }

    /// Spectra given by hand (tests): `(row, spectrum)`.
    pub fn from_parts(n_rows: usize, parts: Vec<(usize, Spec)>) -> Self {
        let mut s = Spectra::empty(n_rows);
        for (row, spec) in parts {
            s.slot[row] = s.specs.len() as u32;
            s.specs.push(spec);
        }
        s
    }

    pub fn of(&self, row: usize) -> Option<&Spec> {
        let s = *self.slot.get(row)?;
        (s != NONE).then(|| &self.specs[s as usize])
    }
}

/// The scan (index in `Index::scans`) of the highest purity among the MS2 linked to row `i`.
fn best_scan(links: &Ms2Links, i: usize) -> Option<u32> {
    let (scans, pur) = links.of(i);
    let mut best: Option<(f32, u32)> = None;
    for (&s, &p) in scans.iter().zip(pur) {
        if best.is_none_or(|(bp, _)| p > bp) {
            best = Some((p, s));
        }
    }
    best.map(|(_, s)| s)
}

/// Strongest peaks of a scan while the table goes by in ascending m/z: at most `cap`, a stronger peak replaces the weakest kept one
/// (an equal one does not, so the earlier m/z stays).
#[derive(Default, Clone)]
struct Top {
    mz: Vec<f64>,
    it: Vec<f64>,
}

impl Top {
    fn push(&mut self, cap: usize, m: f64, y: f64) {
        if self.mz.len() < cap {
            self.mz.push(m);
            self.it.push(y);
            return;
        }
        let mut w = 0;
        for k in 1..self.it.len() {
            if self.it[k] < self.it[w] {
                w = k;
            }
        }
        if y > self.it[w] {
            self.mz[w] = m;
            self.it[w] = y;
        }
    }
}

/// Spectra of the best MS2 scan of every row of `rows`.
pub fn gather<R: ReadAt>(
    index: &mut Index<R>,
    f: &FileFeatures,
    links: &Ms2Links,
    rows: &[u32],
    p: &Params,
    rules: &Rules,
) -> Result<Spectra> {
    let mut out = Spectra::empty(f.len());
    let Some(si) = index.section_of(2, p.polarity) else {
        return Ok(out);
    };
    if links.start.len() != f.len() + 1 {
        return Ok(out);
    }
    let mut want: Vec<(u32, u32)> = rows
        .iter()
        .filter_map(|&r| best_scan(links, r as usize).map(|s| (s, r)))
        .collect();
    sortc::ord(&mut want);
    let ids = index.arrays(si)?.scan_ids.clone();
    let mut pos_slot = vec![NONE; ids.len()];
    let mut scans: Vec<u32> = Vec::new();
    for (k, &id) in ids.iter().enumerate() {
        if id >= 0 && want.binary_search_by_key(&(id as u32), |w| w.0).is_ok() {
            pos_slot[k] = scans.len() as u32;
            scans.push(id as u32);
        }
    }
    if scans.is_empty() {
        return Ok(out);
    }
    let mut top: Vec<Top> = vec![Top::default(); scans.len()];
    let n = index.sections[si].n_peaks;
    let cap = rules.max_peaks;
    index.each_peak(si, 0, n, |pos, mz, it| {
        if let Some(&s) = pos_slot.get(pos) {
            if s != NONE {
                top[s as usize].push(cap, mz, it);
            }
        }
    })?;
    out.specs = top
        .iter()
        .map(|t| rules.strongest(&t.mz, &t.it).unwrap_or_default())
        .collect();
    for &(scan, row) in &want {
        if let Ok(k) = scans.binary_search(&scan) {
            if !out.specs[k].mz.is_empty() {
                out.slot[row as usize] = k as u32;
            }
        }
    }
    Ok(out)
}
