//! The MS2 spectrum of a row for the exports, read from the `.mzidx` of the file that has its best MS2 (`RowMeta::ms2_file`): among the MS2 scans of
//! that file whose isolation window holds the m/z of the row and whose time lies between the edges of the peak, the one with the highest TIC. One pass
//! over the MS2 peaks of each file reads all the scans wanted from it. Two options, both off by default (then the bytes are the ones of the single best scan):
//! `consensus` (all the fitting scans of the row, then across the files where the row has a peak, with `msnalbero::consenso`) and `background` (the MS2
//! peaks that recur in the blank files are taken out of every row, never the precursor).

use crate::sortc::SortC;
use std::collections::{HashMap, HashSet};

use mzlab_core::msnalbero::consenso;

use mzlab_core::error::Result;
use mzlab_core::idxread::{Index, ReadAt};

use crate::aligned::NO_FILE;
use crate::export::Ctx;
use crate::export_ms::Ms2;

/// Half window around the precursor of a scan that declares no isolation window, m/z (the one `ms2::link` assumes).
const HALF_WINDOW: f64 = 0.6;

/// The scan (index into `Index::scans`) chosen for an ion of `mz` between `rt_lo` and `rt_hi`, from the MS2 scans `by_rt` sorted by time.
pub(crate) fn pick<R: ReadAt>(
    index: &Index<R>,
    by_rt: &[u32],
    mz: f64,
    polarity: i8,
    rt_lo: f64,
    rt_hi: f64,
) -> Option<u32> {
    fitting(index, by_rt, mz, polarity, rt_lo, rt_hi).max_by(|&x, &y| {
        index.scans[x as usize]
            .tic
            .total_cmp(&index.scans[y as usize].tic)
    })
}

/// Every scan of `by_rt` whose isolation window holds `mz` and whose time lies in `rt_lo..=rt_hi`, in time order.
fn fitting<'a, R: ReadAt>(
    index: &'a Index<R>,
    by_rt: &'a [u32],
    mz: f64,
    polarity: i8,
    rt_lo: f64,
    rt_hi: f64,
) -> impl Iterator<Item = u32> + 'a {
    let a = by_rt.partition_point(|&g| index.scans[g as usize].rt < rt_lo);
    by_rt[a..]
        .iter()
        .take_while(move |&&g| index.scans[g as usize].rt <= rt_hi)
        .copied()
        .filter(move |&g| {
            let s = &index.scans[g as usize];
            let (lo, hi) = match (s.iso, s.precursor) {
                (Some((a, b)), _) => (a, b),
                (None, Some(p)) if p != 0.0 => (p - HALF_WINDOW, p + HALF_WINDOW),
                _ => return false,
            };
            (lo..=hi).contains(&mz) && (s.polarity == polarity || (polarity > 0 && s.polarity == 0))
        })
}

/// The peaks of the scans `wanted` (indices into `Index::scans`) of the MS2 section of a polarity, by scan.
pub(crate) fn read_peaks<R: ReadAt>(
    index: &mut Index<R>,
    polarity: i8,
    wanted: &[u32],
) -> Result<HashMap<u32, (Vec<f64>, Vec<f64>)>> {
    let mut out = HashMap::new();
    let Some(si) = index.section_of(2, Some(polarity)) else {
        return Ok(out);
    };
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

/// Consensus parameters, the ones of the Dig HR: ppm between peaks of a group, fraction of the scans with the peak, floor relative to the base peak.
const CONS_PPM: f64 = 8.0;
const CONS_FRAC: f64 = 0.4;
const CONS_REL: f64 = 0.005;
/// Background list: a blank scan counts from this base peak up; a peak is background in a blank when it is in this fraction of those scans, and in the
/// whole list when it is in this fraction of the blanks; at most `BG_TOP` peaks stay.
const BG_MIN_BASE: f64 = 5000.0;
const BG_SCAN_FRAC: f64 = 0.1;
const BG_BLANK_FRAC: f64 = 0.8;
const BG_TOP: usize = 25;
/// Peaks this close to the precursor (m/z) are never taken out as background.
const PREC_KEEP: f64 = 1.0;
/// Tolerance of the background match, Da: high resolution and unit resolution.
const TOL_HR: f64 = 0.01;
const TOL_UNIT: f64 = 0.5;

/// What makes the spectrum of a row; both off = the best scan of its file.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct SpecOpts {
    /// All the fitting scans of the row in its file, then the files where the row has a peak, instead of the scan with the highest TIC.
    pub consensus: bool,
    /// Take out the peaks that recur in the blank files.
    pub background: bool,
    /// Unit resolution: the background tolerance is 0.5 Da.
    pub unit: bool,
}

/// What `spectra_with` did besides the spectra.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct SpecInfo {
    /// `""` (background off), `"applied"` or `"no_blank"` (asked for, but the roles name no blank file).
    pub bg: &'static str,
    /// Per row of `Ctx::rows`: peaks taken out as background.
    pub lost: Vec<u32>,
}

/// The MS2 scans of an index in time order.
fn ms2_by_rt<R: ReadAt>(index: &Index<R>) -> Vec<u32> {
    let mut by_rt: Vec<u32> = (0..index.scans.len() as u32)
        .filter(|&g| index.scans[g as usize].level == 2)
        .collect();
    by_rt.sortc_by(|&a, &b| {
        index.scans[a as usize]
            .rt
            .total_cmp(&index.scans[b as usize].rt)
    });
    by_rt
}

/// The MS2 of every row of `Ctx::rows` (same order); `None` for a row with no MS2, or when no scan of its file fits. `open(k)` gives the index of file `k`.
pub fn spectra<R: ReadAt>(
    c: &Ctx,
    mut open: impl FnMut(usize) -> Result<Index<R>>,
) -> Result<Vec<Option<Ms2>>> {
    Ok(spectra_with(c, &mut open, &SpecOpts::default(), &[])?.0)
}

/// `spectra` with the options: `blanks` are the files with the role blank (used only with `background`).
pub fn spectra_with<R: ReadAt>(
    c: &Ctx,
    open: &mut impl FnMut(usize) -> Result<Index<R>>,
    o: &SpecOpts,
    blanks: &[usize],
) -> Result<(Vec<Option<Ms2>>, SpecInfo)> {
    let (t, rows) = (c.table, c.rows());
    let mut info = SpecInfo::default();
    if t.meta.len() != t.len() {
        return Ok((vec![None; rows.len()], info));
    }
    let mut out = if o.consensus {
        consensus_spectra(c, open, &rows)?
    } else {
        best_spectra(c, open, &rows)?
    };
    if o.background {
        info.lost = vec![0; rows.len()];
        if blanks.is_empty() {
            info.bg = "no_blank";
        } else {
            info.bg = "applied";
            let tol = if o.unit { TOL_UNIT } else { TOL_HR };
            let bg = background_list(open, blanks, c.polarity, tol)?;
            for (i, m) in out.iter_mut().enumerate() {
                if let Some(sp) = m {
                    info.lost[i] = take_out(sp, t.mz[rows[i]], &bg, tol);
                    if sp.mz.is_empty() {
                        *m = None;
                    }
                }
            }
        }
    }
    Ok((out, info))
}

/// The scan with the highest TIC of the file of each row.
fn best_spectra<R: ReadAt>(
    c: &Ctx,
    open: &mut impl FnMut(usize) -> Result<Index<R>>,
    rows: &[usize],
) -> Result<Vec<Option<Ms2>>> {
    let t = c.table;
    let mut out: Vec<Option<Ms2>> = vec![None; rows.len()];
    for k in 0..t.n_files {
        let mine: Vec<usize> = (0..rows.len())
            .filter(|&i| {
                t.meta.ms2_file[rows[i]] == k as u32 && t.meta.ms2_file[rows[i]] != NO_FILE
            })
            .collect();
        if mine.is_empty() {
            continue;
        }
        let mut index = open(k)?;
        let by_rt = ms2_by_rt(&index);
        let chosen: Vec<(usize, u32)> = mine
            .iter()
            .filter_map(|&i| {
                let (r, q) = (rows[i], t.meta.get(rows[i]));
                pick(
                    &index,
                    &by_rt,
                    t.mz[r],
                    c.polarity,
                    f64::from(q.rt_lo),
                    f64::from(q.rt_hi),
                )
                .map(|g| (i, g))
            })
            .collect();
        let wanted: Vec<u32> = chosen.iter().map(|x| x.1).collect();
        let peaks = read_peaks(&mut index, c.polarity, &wanted)?;
        for (i, g) in chosen {
            if let Some((mz, it)) = peaks.get(&g).cloned() {
                out[i] = Some(Ms2 {
                    prec: index.scans[g as usize].prec(0.0),
                    mz,
                    it,
                    files: 1,
                });
            }
        }
    }
    Ok(out)
}

/// The consensus of the fitting scans of each row in every file where it has a peak; the file of its best MS2 must give a spectrum, else the row has none.
/// Intensities are relative to the strongest peak (= 100), like `legacy::ms2::consensus`.
fn consensus_spectra<R: ReadAt>(
    c: &Ctx,
    open: &mut impl FnMut(usize) -> Result<Index<R>>,
    rows: &[usize],
) -> Result<Vec<Option<Ms2>>> {
    let t = c.table;
    let mut per_row: Vec<Vec<(Vec<f64>, Vec<f64>)>> = vec![Vec::new(); rows.len()];
    let mut prec: Vec<Option<f64>> = vec![None; rows.len()];
    for k in 0..t.n_files {
        let mine: Vec<usize> = (0..rows.len())
            .filter(|&i| {
                let f = t.meta.ms2_file[rows[i]];
                f != NO_FILE && (f == k as u32 || c.area(rows[i], k).is_some_and(|a| a > 0.0))
            })
            .collect();
        if mine.is_empty() {
            continue;
        }
        let mut index = open(k)?;
        let by_rt = ms2_by_rt(&index);
        let mut sets: Vec<(usize, Vec<u32>)> = Vec::new();
        for &i in &mine {
            let (r, q) = (rows[i], t.meta.get(rows[i]));
            let (lo, hi) = (f64::from(q.rt_lo), f64::from(q.rt_hi));
            let g: Vec<u32> = fitting(&index, &by_rt, t.mz[r], c.polarity, lo, hi).collect();
            if g.is_empty() {
                continue;
            }
            if t.meta.ms2_file[r] == k as u32 {
                prec[i] = pick(&index, &by_rt, t.mz[r], c.polarity, lo, hi)
                    .map(|b| index.scans[b as usize].prec(0.0));
            }
            sets.push((i, g));
        }
        let wanted: Vec<u32> = sets.iter().flat_map(|s| s.1.iter().copied()).collect();
        let peaks = read_peaks(&mut index, c.polarity, &wanted)?;
        for (i, g) in sets {
            let scans: Vec<(Vec<f64>, Vec<f64>)> =
                g.iter().filter_map(|x| peaks.get(x).cloned()).collect();
            if scans.is_empty() {
                continue;
            }
            let (mz, rel, _) = consenso(&scans, CONS_PPM, CONS_FRAC, CONS_REL);
            if !mz.is_empty() {
                per_row[i].push((mz, rel));
            }
        }
    }
    Ok(per_row
        .into_iter()
        .zip(prec)
        .map(|(files, prec)| {
            let prec = prec?;
            let n = files.len() as u32;
            let (mz, rel) = match files.len() {
                0 => return None,
                1 => files.into_iter().next()?,
                _ => {
                    let (mz, rel, _) = consenso(&files, CONS_PPM, CONS_FRAC, CONS_REL);
                    (mz, rel)
                }
            };
            let mx = rel.iter().copied().fold(f64::NEG_INFINITY, f64::max);
            if mz.is_empty() || !(mx > 0.0) {
                return None;
            }
            let it = rel.iter().map(|r| 100.0 * r / mx).collect();
            Some(Ms2 {
                prec,
                mz,
                it,
                files: n,
            })
        })
        .collect())
}

/// Groups of consecutive peaks of `v` (sorted by m/z) that start within `tol` of the first of the group: ranges into `v`.
fn groups(v: &[(f64, f64, u32)], tol: f64) -> Vec<(usize, usize)> {
    let mut out = Vec::new();
    let mut a = 0;
    for i in 1..=v.len() {
        if i == v.len() || v[i].0 - v[a].0 > tol {
            out.push((a, i));
            a = i;
        }
    }
    out
}

/// The m/z that recur in the blank files: in a blank, peaks in at least `BG_SCAN_FRAC` of its scans with a base peak of `BG_MIN_BASE` or more; across
/// the blanks, those in at least `BG_BLANK_FRAC` of them, the `BG_TOP` strongest.
fn background_list<R: ReadAt>(
    open: &mut impl FnMut(usize) -> Result<Index<R>>,
    blanks: &[usize],
    polarity: i8,
    tol: f64,
) -> Result<Vec<f64>> {
    // (m/z, intensity, blank) of the recurring peaks of every blank
    let mut all: Vec<(f64, f64, u32)> = Vec::new();
    for (bi, &b) in blanks.iter().enumerate() {
        let mut index = open(b)?;
        let wanted: Vec<u32> = ms2_by_rt(&index)
            .into_iter()
            .filter(|&g| {
                let s = &index.scans[g as usize];
                s.polarity == polarity || (polarity > 0 && s.polarity == 0)
            })
            .collect();
        let peaks = read_peaks(&mut index, polarity, &wanted)?;
        let mut flat: Vec<(f64, f64, u32)> = Vec::new();
        let mut n_scans = 0usize;
        for g in &wanted {
            let Some((mz, it)) = peaks.get(g) else {
                continue;
            };
            if it.iter().copied().fold(0.0, f64::max) < BG_MIN_BASE {
                continue;
            }
            n_scans += 1;
            flat.extend(mz.iter().zip(it).map(|(&m, &i)| (m, i, *g)));
        }
        if n_scans == 0 {
            continue;
        }
        flat.sortc_by(|a, b| a.0.total_cmp(&b.0));
        let need = (BG_SCAN_FRAC * n_scans as f64).ceil().max(1.0) as usize;
        for (a, z) in groups(&flat, tol) {
            let seen: HashSet<u32> = flat[a..z].iter().map(|p| p.2).collect();
            if seen.len() >= need {
                let w: f64 = flat[a..z].iter().map(|p| p.1).sum();
                let mz = flat[a..z].iter().map(|p| p.0 * p.1).sum::<f64>() / w;
                all.push((mz, w / n_scans as f64, bi as u32));
            }
        }
    }
    all.sortc_by(|a, b| a.0.total_cmp(&b.0));
    let need = (BG_BLANK_FRAC * blanks.len() as f64).ceil().max(1.0) as usize;
    let mut kept: Vec<(f64, f64)> = Vec::new();
    for (a, z) in groups(&all, tol) {
        let seen: HashSet<u32> = all[a..z].iter().map(|p| p.2).collect();
        if seen.len() >= need {
            let w: f64 = all[a..z].iter().map(|p| p.1).sum();
            kept.push((all[a..z].iter().map(|p| p.0 * p.1).sum::<f64>() / w, w));
        }
    }
    kept.sortc_by(|a, b| b.1.total_cmp(&a.1));
    kept.truncate(BG_TOP);
    Ok(kept.into_iter().map(|p| p.0).collect())
}

/// Removes from `sp` the peaks within `tol` of an m/z of `bg`, except those within `PREC_KEEP` of the precursor `prec`; the number removed.
fn take_out(sp: &mut Ms2, prec: f64, bg: &[f64], tol: f64) -> u32 {
    let n = sp.mz.len();
    let keep: Vec<bool> = sp
        .mz
        .iter()
        .map(|&m| (m - prec).abs() <= PREC_KEEP || !bg.iter().any(|&b| (m - b).abs() <= tol))
        .collect();
    let mut k = keep.iter();
    sp.mz.retain(|_| *k.next().unwrap_or(&true));
    let mut k = keep.iter();
    sp.it.retain(|_| *k.next().unwrap_or(&true));
    (n - sp.mz.len()) as u32
}
