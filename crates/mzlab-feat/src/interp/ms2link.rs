//! The MS2 scans of a row in every file of the series (DDA only): which scans, with what isolation window, energy and scan range, how pure the
//! precursor was (MS1 before and after, interpolated to the time of the MS2), the consensus spectrum and the separation of isomers by the traces of
//! the main fragments. Used by `interp::build_row`.

use mzlab_core::error::Result;
use mzlab_core::idxread::{Index, ReadAt};

use crate::axis::half;
use crate::legacy::ms2::{consensus, read_scans, select, Consensus, Ms2Index};
use crate::sweep::windows;

/// Isolation centre within this many ppm of the m/z of the row (as mzFinder pools the MS2).
pub const SELECT_PPM: f64 = 10.0;
/// Precursor window of the purity, ppm.
pub const PURITY_PPM: f64 = 5.0;
/// Purity below which a scan is chimeric and does not vote.
pub const PURITY_MIN: f64 = 0.8;
/// Correlation between the traces of two fragments below which they belong to different isomers.
pub const ISO_CORR: f64 = 0.5;
/// Fewest clean scans to look for isomers.
pub const ISO_MIN_SCANS: usize = 4;
/// Fragments whose traces are compared.
pub const ISO_FRAGMENTS: usize = 5;

/// One MS2 scan linked to a row.
#[derive(Debug, Clone, PartialEq)]
pub struct Ms2Scan {
    pub file: usize,
    pub scan: usize,
    pub rt: f64,
    pub precursor: f64,
    pub iso: (f64, f64),
    pub ce: Option<f64>,
    pub scan_range: Option<(f64, f64)>,
    /// `NaN` when no MS1 scan lies around the MS2.
    pub purity: f64,
    pub chimeric: bool,
    pub mz: Vec<f64>,
    pub it: Vec<f64>,
}

/// Collision energy and scan range from a Thermo-style filter text (`... ms2 337.17@hcd30.00 [50.0000-355.0000]`), for the files whose index lacks them.
pub fn parse_filter(filter: &str) -> (Option<f64>, Option<(f64, f64)>) {
    let ce = filter.split('@').nth(1).and_then(|s| {
        let num: String = s
            .chars()
            .skip_while(|c| c.is_alphabetic())
            .take_while(|c| c.is_ascii_digit() || *c == '.')
            .collect();
        num.parse().ok()
    });
    let range = filter.rfind('[').zip(filter.rfind(']')).and_then(|(a, b)| {
        let (lo, hi) = filter.get(a + 1..b)?.split_once('-')?;
        Some((lo.trim().parse().ok()?, hi.trim().parse().ok()?))
    });
    (ce, range)
}

/// Purity at time `t` from the purities of the MS1 scans before and after: `(time, purity)`; linear between them, the one that exists otherwise.
pub fn interpolate(t: f64, before: Option<(f64, f64)>, after: Option<(f64, f64)>) -> f64 {
    match (before, after) {
        (Some(b), Some(a)) if a.0 > b.0 => {
            b.1 + (a.1 - b.1) * ((t - b.0) / (a.0 - b.0)).clamp(0.0, 1.0)
        }
        (Some(b), _) => b.1,
        (None, Some(a)) => a.1,
        (None, None) => f64::NAN,
    }
}

/// The scans of file `k` that belong to the row (`select` with the edges of the peak in this file), without peaks and purity yet.
pub fn pick_scans<R: ReadAt>(ix: &Index<R>, k: usize, mz: f64, limits: (f64, f64)) -> Vec<Ms2Scan> {
    let m2 = Ms2Index::of(ix);
    select(&[m2], mz, &[limits], SELECT_PPM)
        .into_iter()
        .map(|(_, g, rt)| {
            let s = &ix.scans[g];
            let (fce, frange) = parse_filter(&s.filter);
            let ce = s
                .collision_energy
                .or_else(|| s.path.as_ref().and_then(|p| p.last().map(|x| x.2)))
                .or(fce);
            Ms2Scan {
                file: k,
                scan: g,
                rt,
                precursor: s.prec(mz),
                iso: s.iso.unwrap_or((mz, mz)),
                ce,
                scan_range: frange,
                purity: f64::NAN,
                chimeric: false,
                mz: vec![],
                it: vec![],
            }
        })
        .collect()
}

/// Fills `purity` and `chimeric` of `scans` (all of file `ix`) from the MS1 scans of the section of `polarity` around each of them.
pub fn fill_purity<R: ReadAt>(
    ix: &mut Index<R>,
    polarity: i8,
    scans: &mut [Ms2Scan],
) -> Result<()> {
    let Some(si) = ix.section_of(1, Some(polarity)) else {
        return Ok(());
    };
    let rt = ix.arrays(si)?.rt.clone();
    let mut asks: Vec<(f64, f64, usize, bool, u32)> = Vec::new();
    for (i, s) in scans.iter().enumerate() {
        let after = rt.partition_point(|&t| t <= s.rt);
        if after > 0 {
            asks.push((s.iso.0, s.iso.1, i, true, (after - 1) as u32));
        }
        if after < rt.len() {
            asks.push((s.iso.0, s.iso.1, i, false, after as u32));
        }
    }
    asks.sort_by(|a, b| a.0.total_cmp(&b.0));
    let ranges: Vec<(f64, f64)> = asks.iter().map(|a| (a.0, a.1)).collect();
    let mut found: Vec<(usize, bool, f64)> = Vec::new();
    windows(ix, si, &ranges, 1 << 20, |k, a, b, slab| {
        let (_, _, i, before, pos) = asks[k];
        let prec = scans[i].precursor;
        let tol = half(prec, PURITY_PPM);
        let (mut own, mut total) = (0.0, 0.0);
        for j in (a..b).filter(|&j| slab.pos[j] == pos) {
            total += slab.it[j];
            if (slab.mz[j] - prec).abs() <= tol {
                own += slab.it[j];
            }
        }
        if total > 0.0 {
            found.push((i, before, own / total));
        }
        Ok(())
    })?;
    for (i, s) in scans.iter_mut().enumerate() {
        let at = |before: bool| {
            let p = rt.partition_point(|&t| t <= s.rt);
            let pos = if before { p.checked_sub(1) } else { Some(p) };
            let v = found.iter().find(|f| f.0 == i && f.1 == before)?.2;
            pos.and_then(|p| rt.get(p)).map(|&t| (t, v))
        };
        s.purity = interpolate(s.rt, at(true), at(false));
        s.chimeric = s.purity.is_nan() || s.purity < PURITY_MIN;
    }
    Ok(())
}

/// Reads the peaks of `scans` from the file.
pub fn fill_peaks<R: ReadAt>(ix: &mut Index<R>, scans: &mut [Ms2Scan]) -> Result<()> {
    let wanted: Vec<usize> = scans.iter().map(|s| s.scan).collect();
    for (s, (mz, it)) in scans.iter_mut().zip(read_scans(ix, &wanted)?) {
        s.mz = mz;
        s.it = it;
    }
    Ok(())
}

/// How the clean scans split into isomers.
#[derive(Debug, Clone, PartialEq)]
pub enum Isomers {
    Single,
    Separated(Vec<Vec<usize>>),
    NotSeparated,
}

/// Pearson correlation; 0 when a trace is flat.
pub fn pearson(a: &[f64], b: &[f64]) -> f64 {
    let n = a.len() as f64;
    let (ma, mb) = (a.iter().sum::<f64>() / n, b.iter().sum::<f64>() / n);
    let (mut sab, mut saa, mut sbb) = (0.0, 0.0, 0.0);
    for (x, y) in a.iter().zip(b) {
        sab += (x - ma) * (y - mb);
        saa += (x - ma).powi(2);
        sbb += (y - mb).powi(2);
    }
    if saa <= 0.0 || sbb <= 0.0 {
        0.0
    } else {
        sab / (saa * sbb).sqrt()
    }
}

/// Intensity of the peak nearest to `mz` within `tol_ppm` in a scan (0 when none).
fn at(s: &Ms2Scan, mz: f64, tol_ppm: f64) -> f64 {
    let tol = half(mz, tol_ppm);
    s.mz.iter()
        .zip(&s.it)
        .filter(|(m, _)| (**m - mz).abs() <= tol)
        .map(|(_, i)| *i)
        .fold(0.0, f64::max)
}

/// Splits the clean scans (given in time order) into isomers by the traces of the strongest fragments of their consensus `cons`:
/// two groups of fragments whose traces correlate below `ISO_CORR`, and the scans each group dominates. Returns indices into `scans`.
pub fn separate(scans: &[&Ms2Scan], cons: &Consensus) -> Isomers {
    if scans.len() < ISO_MIN_SCANS || cons.mz.len() < 2 {
        return Isomers::Single;
    }
    let mut order: Vec<usize> = (0..cons.mz.len()).collect();
    order.sort_by(|&a, &b| cons.rel[b].total_cmp(&cons.rel[a]));
    order.truncate(ISO_FRAGMENTS);
    let traces: Vec<Vec<f64>> = order
        .iter()
        .map(|&f| scans.iter().map(|s| at(s, cons.mz[f], 8.0)).collect())
        .collect();
    let mut seed = (0, 1, f64::INFINITY);
    for i in 0..traces.len() {
        for j in i + 1..traces.len() {
            let c = pearson(&traces[i], &traces[j]);
            if c < seed.2 {
                seed = (i, j, c);
            }
        }
    }
    if seed.2 >= ISO_CORR {
        return Isomers::Single;
    }
    let sum = |idx: &[usize]| -> Vec<f64> {
        (0..scans.len())
            .map(|s| idx.iter().map(|&i| traces[i][s]).sum())
            .collect()
    };
    let (mut ga, mut gb) = (vec![seed.0], vec![seed.1]);
    for i in (0..traces.len()).filter(|i| *i != seed.0 && *i != seed.1) {
        let (ca, cb) = (
            pearson(&traces[i], &traces[seed.0]),
            pearson(&traces[i], &traces[seed.1]),
        );
        if ca >= cb {
            ga.push(i)
        } else {
            gb.push(i)
        }
    }
    let (ta, tb) = (sum(&ga), sum(&gb));
    if pearson(&ta, &tb) >= ISO_CORR {
        return Isomers::Single;
    }
    let (ma, mb) = (
        ta.iter().copied().fold(0.0, f64::max),
        tb.iter().copied().fold(0.0, f64::max),
    );
    let mut groups = vec![vec![], vec![]];
    for s in 0..scans.len() {
        groups[usize::from(ta[s] / ma <= tb[s] / mb)].push(s);
    }
    if groups.iter().any(|g| g.len() < 2) {
        Isomers::NotSeparated
    } else {
        Isomers::Separated(groups)
    }
}

/// The consensus of the clean scans, and of each isomer group when they separate.
pub struct Pooled {
    pub consensus: Consensus,
    pub groups: Vec<Consensus>,
    pub isomers: Isomers,
}

pub fn pool(scans: &[Ms2Scan]) -> Pooled {
    let mut clean: Vec<&Ms2Scan> = scans.iter().filter(|s| !s.chimeric).collect();
    clean.sort_by_key(|s| (s.file, s.rt.to_bits()));
    let spectra: Vec<(Vec<f64>, Vec<f64>)> =
        clean.iter().map(|s| (s.mz.clone(), s.it.clone())).collect();
    let cons = consensus(&spectra);
    let isomers = separate(&clean, &cons);
    let groups = match &isomers {
        Isomers::Separated(g) => g
            .iter()
            .map(|idx| consensus(&idx.iter().map(|&i| spectra[i].clone()).collect::<Vec<_>>()))
            .collect(),
        _ => vec![],
    };
    Pooled {
        consensus: cons,
        groups,
        isomers,
    }
}
