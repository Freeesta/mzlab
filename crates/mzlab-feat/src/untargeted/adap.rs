//! ADAP chromatogram builder. Rewritten from scratch after the algorithm of mzmine 3.9.0
//! `modules/dataprocessing/featdet_adapchromatogrambuilder/ModularADAPChromatogramBuilderTask.java` and `ADAPChromatogram.java`
//! (MIT, (c) 2004-2022 The mzmine Development Team); no code copied. Paper: Myers OD et al., Anal. Chem. 89:8696 (2017), doi:10.1021/acs.analchem.7b00947;
//! MZmine 3: Schmid R et al., Nat. Biotechnol. 41:447 (2023), doi:10.1038/s41587-023-01690-2.
//!
//! What the Java does, and so what this does:
//! - every point of every MS1 scan (already above the noise of the mass detection) is sorted by intensity, descending; equal intensities by m/z,
//!   descending (`DataPointSorter`); the sort is stable, so full ties keep the order scan, then position;
//! - a point joins the chromatogram whose m/z interval contains it (a range map of half-open intervals); with none, a point under `min_highest` is dropped and
//!   one above opens an interval of +-max(`tol_da`, m/z * `tol_ppm`) around ITS m/z, clipped to the end of the interval that holds its lower edge and
//!   to the start of the one that holds its upper edge (a Guava `put` also erases any interval that falls inside the new one);
//! - one point per scan: the first to arrive, the most intense; holes never split it; the m/z listed for the feature is the mean of its points weighted
//!   by intensity (the plain mean, `mz_mean`, is the field of the Java class);
//! - kept if `min_consecutive` consecutive scans have a point >= `min_group_intensity` while the highest point met so far (the Java never resets it
//!   between runs) is >= `min_highest`; then one zero scan on each side of every run of detected scans.
//!
//! Scan identity is the position in the `scans` slice. Output order: by m/z interval, like the Java (which then sorts the rows by RT).
//! Memory: `build_slabs` cuts the m/z axis where the gap between two points exceeds twice the tolerance, so no interval crosses a cut, and builds a slab at a
//! time (`docs/agenti/memoria.md`); the result equals `build`.

use std::collections::BTreeMap;

/// One MS1 scan of the mass list (points above the noise of the mass detection), m/z ascending.
#[derive(Debug, Clone, Default)]
pub struct Ms1Scan {
    pub index: usize,
    pub rt: f64,
    pub mz: Vec<f64>,
    pub int: Vec<f32>,
}

impl Ms1Scan {
    /// The centroid mass detector of mzmine: keeps the points with intensity >= `noise`.
    pub fn above_noise(index: usize, rt: f64, mz: &[f64], int: &[f32], noise: f64) -> Ms1Scan {
        let mut s = Ms1Scan {
            index,
            rt,
            ..Default::default()
        };
        for (m, i) in mz.iter().zip(int) {
            if *i as f64 >= noise {
                s.mz.push(*m);
                s.int.push(*i);
            }
        }
        s
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct AdapParams {
    /// Noise level of the mass detection (applied by the caller, see `Ms1Scan::above_noise`).
    pub noise: f64,
    pub min_consecutive: usize,
    pub min_group_intensity: f64,
    pub min_highest: f64,
    pub tol_da: f64,
    pub tol_ppm: f64,
}

impl Default for AdapParams {
    /// `tools/bench/parametri/mzmine/predefinita.json`.
    fn default() -> Self {
        AdapParams {
            noise: 1e4,
            min_consecutive: 5,
            min_group_intensity: 1e4,
            min_highest: 3e4,
            tol_da: 0.002,
            tol_ppm: 5.0,
        }
    }
}

impl AdapParams {
    fn tol(&self, mz: f64) -> f64 {
        self.tol_da.max(mz / 1e6 * self.tol_ppm)
    }
}

/// A chromatogram over all the scans of the run: `scan` ascending, `int` 0.0 on the padded zeros.
#[derive(Debug, Clone, PartialEq)]
pub struct Chrom {
    /// m/z of the feature: mean of the real points weighted by intensity (what mzmine lists).
    pub mz: f64,
    /// Plain mean of the m/z of the real points (the `mz` field of `ADAPChromatogram`).
    pub mz_mean: f64,
    pub scan: Vec<u32>,
    pub int: Vec<f32>,
    /// Real points (without the padded zeros).
    pub points: usize,
}

impl Chrom {
    /// Position in `scan` of the most intense point (the first on a tie).
    pub fn apex(&self) -> usize {
        let mut best = 0;
        for (k, v) in self.int.iter().enumerate() {
            if *v > self.int[best] {
                best = k;
            }
        }
        best
    }
}

#[derive(Clone, Copy)]
struct Pt {
    int: f32,
    mz: f64,
    scan: u32,
}

struct Building {
    pts: BTreeMap<u32, (f32, f64)>,
    sum: f64,
}

impl Building {
    fn add(&mut self, p: Pt) {
        if self.pts.contains_key(&p.scan) {
            return;
        }
        self.pts.insert(p.scan, (p.int, p.mz));
        self.sum += p.mz;
    }
}

#[derive(Clone, Copy, PartialEq, PartialOrd)]
struct Key(f64);
impl Eq for Key {}
impl Ord for Key {
    fn cmp(&self, o: &Self) -> std::cmp::Ordering {
        self.0.total_cmp(&o.0)
    }
}

/// Half-open intervals `[lo, hi)` that do not overlap, each with the id of its chromatogram (the `TreeRangeMap` of the Java).
#[derive(Default)]
struct Ranges {
    map: BTreeMap<Key, (f64, usize)>,
}

impl Ranges {
    /// The interval that holds `x`: (lo, hi, id).
    fn at(&self, x: f64) -> Option<(f64, f64, usize)> {
        let (k, (hi, id)) = self.map.range(..=Key(x)).next_back()?;
        (x < *hi).then_some((k.0, *hi, *id))
    }

    /// Guava `put`: whatever overlaps `[lo, hi)` is cut away first.
    fn put(&mut self, lo: f64, hi: f64, id: usize) {
        // The intervals do not overlap: the one that starts before `lo` is the only one that can reach into it, the others start inside.
        let before = self
            .map
            .range(..Key(lo))
            .next_back()
            .filter(|(_, (h, _))| *h > lo)
            .map(|(k, _)| *k);
        let over: Vec<Key> = before
            .into_iter()
            .chain(self.map.range(Key(lo)..Key(hi)).map(|(k, _)| *k))
            .collect();
        for k in over {
            let (h, old) = self.map.remove(&k).unwrap();
            if k.0 < lo {
                self.map.insert(k, (lo, old));
            }
            if h > hi {
                self.map.insert(Key(hi), (h, old));
            }
        }
        self.map.insert(Key(lo), (hi, id));
    }
}

fn gather(scans: &[Ms1Scan], range: Option<(f64, f64)>) -> Vec<Pt> {
    let mut v = Vec::new();
    for (s, sc) in scans.iter().enumerate() {
        for (m, i) in sc.mz.iter().zip(&sc.int) {
            if m.is_nan() || i.is_nan() {
                continue;
            }
            if range.is_none_or(|(a, b)| *m >= a && *m < b) {
                v.push(Pt {
                    int: *i,
                    mz: *m,
                    scan: s as u32,
                });
            }
        }
    }
    v
}

/// The intervals and chromatograms of the points, in the order of the intervals.
fn assemble(mut pts: Vec<Pt>, p: &AdapParams) -> Vec<Building> {
    pts.sort_by(|a, b| {
        (b.int as f64)
            .total_cmp(&(a.int as f64))
            .then(b.mz.total_cmp(&a.mz))
    });
    let (mut ranges, mut chroms) = (Ranges::default(), Vec::<Building>::new());
    for pt in pts {
        if let Some((_, _, id)) = ranges.at(pt.mz) {
            chroms[id].add(pt);
        } else if (pt.int as f64) >= p.min_highest {
            open(&mut ranges, &mut chroms, pt, p);
        }
    }
    let order: Vec<usize> = ranges.map.values().map(|(_, id)| *id).collect();
    let mut slots: Vec<Option<Building>> = chroms.into_iter().map(Some).collect();
    order
        .into_iter()
        .filter_map(|id| slots[id].take())
        .collect()
}

fn open(ranges: &mut Ranges, chroms: &mut Vec<Building>, pt: Pt, p: &AdapParams) {
    let t = p.tol(pt.mz);
    let (a, b) = (pt.mz - t, pt.mz + t);
    let lo = ranges.at(a).map_or(a, |(_, hi, _)| hi);
    let hi = ranges.at(b).map_or(b, |(l, _, _)| l);
    if lo < hi {
        let mut c = Building {
            pts: BTreeMap::new(),
            sum: 0.0,
        };
        c.add(pt);
        chroms.push(c);
        ranges.put(lo, hi, chroms.len() - 1);
    } else if let Some((_, _, id)) = ranges.at(b).filter(|_| lo == hi) {
        chroms[id].add(pt);
    }
}

/// `matchesMinContinuousDataPoints`: `max` is never reset between runs, as in the Java.
fn keeps(c: &Building, nscans: usize, p: &AdapParams) -> bool {
    if p.min_consecutive <= 1 && !c.pts.is_empty() {
        return true;
    }
    let (mut run, mut max) = (0usize, 0f64);
    for s in 0..nscans {
        match c.pts.get(&(s as u32)) {
            Some((i, _)) if *i as f64 >= p.min_group_intensity => {
                run += 1;
                max = max.max(*i as f64);
                if run >= p.min_consecutive && max >= p.min_highest {
                    return true;
                }
            }
            _ => run = 0,
        }
    }
    false
}

/// `addNZeros(scans, 1, 1)`: the scans of the run that touch a detected scan, one on each side of every run of detected scans.
fn zeros(c: &Building, nscans: usize) -> Vec<u32> {
    let det: Vec<u32> = c.pts.keys().copied().collect();
    let mut add = Vec::new();
    let (mut gap, mut prev) = (0usize, None::<usize>);
    for (k, d) in det.iter().map(|d| *d as usize).enumerate() {
        gap += d - prev.map_or(0, |q| q + 1);
        if gap >= 1 {
            add.push(d - 1);
            gap -= 1;
            if gap > 0 {
                if let Some(q) = prev {
                    add.push(q + 1);
                }
            }
        }
        gap = 0;
        prev = Some(d);
        if k + 1 == det.len() && d + 1 < nscans {
            add.push(d + 1);
        }
    }
    add.iter().map(|x| *x as u32).collect()
}

fn finish(c: Building, nscans: usize, p: &AdapParams) -> Option<Chrom> {
    if c.pts.len() < p.min_consecutive || !keeps(&c, nscans, p) {
        return None;
    }
    let extra = zeros(&c, nscans);
    let mut rows: Vec<(u32, f32)> = c.pts.iter().map(|(s, (i, _))| (*s, *i)).collect();
    rows.extend(extra.into_iter().map(|s| (s, 0.0)));
    rows.sort_by_key(|r| r.0);
    let wsum: f64 = c.pts.values().map(|(i, _)| *i as f64).sum();
    let weighted = c.pts.values().map(|(i, m)| *i as f64 * m).sum::<f64>() / wsum;
    Some(Chrom {
        mz: if wsum > 0.0 {
            weighted
        } else {
            c.sum / c.pts.len() as f64
        },
        mz_mean: c.sum / c.pts.len() as f64,
        points: c.pts.len(),
        scan: rows.iter().map(|r| r.0).collect(),
        int: rows.iter().map(|r| r.1).collect(),
    })
}

fn run(pts: Vec<Pt>, nscans: usize, p: &AdapParams) -> Vec<Chrom> {
    assemble(pts, p)
        .into_iter()
        .filter_map(|c| finish(c, nscans, p))
        .collect()
}

/// The chromatograms of a run.
pub fn build(scans: &[Ms1Scan], p: &AdapParams) -> Vec<Chrom> {
    run(gather(scans, None), scans.len(), p)
}

/// Bytes a point costs while it is sorted and assigned.
const POINT_BYTES: usize = 48;

/// Cuts of m/z, from `-inf` to `inf`: a slab holds about `per_slab` points and ends in a gap wider than twice the tolerance at its upper side.
fn cuts(scans: &[Ms1Scan], p: &AdapParams, per_slab: usize) -> Vec<f64> {
    let mut all: Vec<f64> = scans
        .iter()
        .flat_map(|s| s.mz.iter().copied())
        .filter(|m| !m.is_nan())
        .collect();
    all.sort_by(|a, b| a.total_cmp(b));
    let (mut cuts, mut since) = (vec![f64::NEG_INFINITY], 0usize);
    for i in 1..all.len() {
        since += 1;
        if since >= per_slab && all[i] - all[i - 1] > 2.0 * p.tol(all[i]) * (1.0 + 1e-9) {
            cuts.push(all[i]);
            since = 0;
        }
    }
    cuts.push(f64::INFINITY);
    cuts
}

/// How many slabs `build_slabs` uses for this budget.
pub fn slab_count(scans: &[Ms1Scan], p: &AdapParams, budget_bytes: usize) -> usize {
    cuts(scans, p, (budget_bytes / POINT_BYTES).max(1)).len() - 1
}

/// The same chromatograms as `build`, one slab of m/z at a time so that no more than about `budget_bytes` of points are alive.
pub fn build_slabs(scans: &[Ms1Scan], p: &AdapParams, budget_bytes: usize) -> Vec<Chrom> {
    let cuts = cuts(scans, p, (budget_bytes / POINT_BYTES).max(1));
    let mut out = Vec::new();
    for w in cuts.windows(2) {
        out.extend(run(gather(scans, Some((w[0], w[1]))), scans.len(), p));
    }
    out
}
