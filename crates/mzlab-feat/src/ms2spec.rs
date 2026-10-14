//! The MS2 spectrum of a row for the exports, read from the `.mzidx` of the file that has its best MS2 (`RowMeta::ms2_file`): among the MS2 scans of
//! that file whose isolation window holds the m/z of the row and whose time lies between the edges of the peak, the one with the highest TIC. One pass
//! over the MS2 peaks of each file reads all the scans wanted from it.

use std::collections::{HashMap, HashSet};

use mzlab_core::error::Result;
use mzlab_core::idxread::{Index, ReadAt};

use crate::aligned::NO_FILE;
use crate::export::Ctx;
use crate::export_ms::Ms2;

/// Half window around the precursor of a scan that declares no isolation window, m/z (the one `ms2::link` assumes).
const HALF_WINDOW: f64 = 0.6;

/// The scan (index into `Index::scans`) chosen for an ion of `mz` between `rt_lo` and `rt_hi`, from the MS2 scans `by_rt` sorted by time.
fn pick<R: ReadAt>(index: &Index<R>, by_rt: &[u32], mz: f64, polarity: i8, rt_lo: f64, rt_hi: f64) -> Option<u32> {
    let a = by_rt.partition_point(|&g| index.scans[g as usize].rt < rt_lo);
    by_rt[a..]
        .iter()
        .take_while(|&&g| index.scans[g as usize].rt <= rt_hi)
        .copied()
        .filter(|&g| {
            let s = &index.scans[g as usize];
            let (lo, hi) = match (s.iso, s.precursor) {
                (Some((a, b)), _) => (a, b),
                (None, Some(p)) if p != 0.0 => (p - HALF_WINDOW, p + HALF_WINDOW),
                _ => return false,
            };
            (lo..=hi).contains(&mz) && (s.polarity == polarity || (polarity > 0 && s.polarity == 0))
        })
        .max_by(|&x, &y| index.scans[x as usize].tic.total_cmp(&index.scans[y as usize].tic))
}

/// The peaks of the scans `wanted` (indices into `Index::scans`) of the MS2 section of a polarity, by scan.
fn read_peaks<R: ReadAt>(index: &mut Index<R>, polarity: i8, wanted: &[u32]) -> Result<HashMap<u32, (Vec<f64>, Vec<f64>)>> {
    let mut out = HashMap::new();
    let Some(si) = index.section_of(2, Some(polarity)) else { return Ok(out) };
    let ids = index.arrays(si)?.scan_ids.clone();
    let want: HashSet<u32> = wanted.iter().copied().collect();
    let mut pos_of: HashMap<u32, u32> = HashMap::new();
    for (p, &g) in ids.iter().enumerate() {
        if g >= 0 && want.contains(&(g as u32)) {
            pos_of.insert(p as u32, g as u32);
        }
    }
    let n = index.sections[si].n_peaks;
    index.each_peak(si, 0, n, |p, mz, it| {
        if let Some(&g) = pos_of.get(&(p as u32)) {
            let e: &mut (Vec<f64>, Vec<f64>) = out.entry(g).or_default();
            e.0.push(mz);
            e.1.push(it);
        }
    })?;
    Ok(out)
}

/// The MS2 of every row of `Ctx::rows` (same order); `None` for a row with no MS2, or when no scan of its file fits. `open(k)` gives the index of file `k`.
pub fn spectra<R: ReadAt>(c: &Ctx, mut open: impl FnMut(usize) -> Result<Index<R>>) -> Result<Vec<Option<Ms2>>> {
    let (t, rows) = (c.table, c.rows());
    let mut out: Vec<Option<Ms2>> = vec![None; rows.len()];
    if t.meta.len() != t.len() {
        return Ok(out);
    }
    for k in 0..t.n_files {
        let mine: Vec<usize> = (0..rows.len()).filter(|&i| t.meta.ms2_file[rows[i]] == k as u32 && t.meta.ms2_file[rows[i]] != NO_FILE).collect();
        if mine.is_empty() {
            continue;
        }
        let mut index = open(k)?;
        let mut by_rt: Vec<u32> = (0..index.scans.len() as u32).filter(|&g| index.scans[g as usize].level == 2).collect();
        by_rt.sort_by(|&a, &b| index.scans[a as usize].rt.total_cmp(&index.scans[b as usize].rt));
        let chosen: Vec<(usize, u32)> = mine
            .iter()
            .filter_map(|&i| {
                let (r, q) = (rows[i], t.meta.get(rows[i]));
                pick(&index, &by_rt, t.mz[r], c.polarity, f64::from(q.rt_lo), f64::from(q.rt_hi)).map(|g| (i, g))
            })
            .collect();
        let wanted: Vec<u32> = chosen.iter().map(|x| x.1).collect();
        let peaks = read_peaks(&mut index, c.polarity, &wanted)?;
        for (i, g) in chosen {
            if let Some((mz, it)) = peaks.get(&g).cloned() {
                out[i] = Some(Ms2 { prec: index.scans[g as usize].prec(0.0), mz, it, files: 1 });
            }
        }
    }
    Ok(out)
}
