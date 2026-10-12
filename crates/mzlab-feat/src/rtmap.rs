//! Map of retention time of a series: port of `TP_Mine/py/tpmine/hr/rtalign.py`. The anchors are the groups that every file shows and that
//! nothing else crowds: strong, alone in their m/z window and RT neighbourhood, with the apex of the same peak in all files. The apex of an
//! anchor in a file minus its median apex over the files is the RT shift of that file; the curve of a file is piecewise linear between the
//! medians of consecutive groups of anchors. It only tells where a feature is expected in a file; no signal is moved.

use crate::sortc;
use crate::table::Aligned;

/// Parameters of `build` (defaults of `rtalign` and of `features.MIN_HEIGHT`).
#[derive(Debug, Clone, Copy)]
pub struct RtParams {
    pub min_height: f64,
    pub min_anchors: usize,
    /// An anchor whose apexes differ more than this (minutes) between files is not one peak.
    pub max_spread: f64,
    pub crowd_ppm: f64,
    pub crowd_rt: f64,
}

impl Default for RtParams {
    fn default() -> Self {
        RtParams {
            min_height: 2e5,
            min_anchors: 8,
            max_spread: 0.5,
            crowd_ppm: 5.0,
            crowd_rt: 0.5,
        }
    }
}

/// Shift curves: `shift[k * nodes + j]` = apex RT in file k minus the reference RT, at the reference RT `grid[j]` (minutes).
#[derive(Debug, Clone, Default, PartialEq)]
pub struct RtMap {
    pub n_files: usize,
    pub grid: Vec<f64>,
    pub shift: Vec<f64>,
    pub n_anchors: usize,
    /// Residual of the anchors against the curve of each file, minutes.
    pub rms: Vec<f64>,
}

/// `np.interp`: clamped outside the grid.
fn interp(x: f64, xp: &[f64], fp: &[f64]) -> f64 {
    let n = xp.len();
    if x <= xp[0] {
        return fp[0];
    }
    if x >= xp[n - 1] {
        return fp[n - 1];
    }
    let j = xp.partition_point(|&v| v <= x) - 1;
    if x == xp[j] {
        return fp[j];
    }
    (fp[j + 1] - fp[j]) / (xp[j + 1] - xp[j]) * (x - xp[j]) + fp[j]
}

/// Median of the non-NaN values (NaN when there are none), like `np.nanmedian`.
fn nanmedian(v: impl Iterator<Item = f64>) -> f64 {
    let mut a: Vec<f64> = v.filter(|x| !x.is_nan()).collect();
    if a.is_empty() {
        return f64::NAN;
    }
    sortc::by(&mut a, f64::total_cmp);
    let n = a.len();
    if n % 2 == 1 {
        a[n / 2]
    } else {
        (a[n / 2 - 1] + a[n / 2]) / 2.0
    }
}

impl RtMap {
    pub fn ok(&self) -> bool {
        self.n_anchors >= 3 && !self.grid.is_empty()
    }

    /// Shift of file `k` at the reference RT `rt` (0 without a curve).
    pub fn at(&self, k: usize, rt: f64) -> f64 {
        if !self.ok() {
            return 0.0;
        }
        let nodes = self.grid.len();
        interp(rt, &self.grid, &self.shift[k * nodes..(k + 1) * nodes])
    }

    /// Where a peak whose reference RT is `ref_rt` is expected in file `k`.
    pub fn expected(&self, ref_rt: f64, k: usize) -> f64 {
        ref_rt + self.at(k, ref_rt)
    }

    /// Reference RT of a peak seen at `rt` in file `k` (inverse of `expected` by one fixed-point step).
    pub fn reference(&self, rt: f64, k: usize) -> f64 {
        rt - self.at(k, rt - self.at(k, rt))
    }
}

/// True for the rows that have no other row within `ppm` and `rt` minutes (an isomer or a neighbour would make the apex ambiguous).
pub fn isolated(al: &Aligned, ppm: f64, rt: f64) -> Vec<bool> {
    let n = al.len();
    let mut ord: Vec<usize> = (0..n).collect();
    sortc::by(&mut ord, |&a, &b| al.mz[a].total_cmp(&al.mz[b]));
    let mz: Vec<f64> = ord.iter().map(|&i| al.mz[i]).collect();
    let mut out = vec![false; n];
    let tol = ppm * 1e-6;
    for (s, &g) in ord.iter().enumerate() {
        if mz[s].is_nan() {
            continue;
        }
        let lo = mz.partition_point(|&v| v < mz[s] * (1.0 - tol));
        let hi = mz.partition_point(|&v| v <= mz[s] * (1.0 + tol));
        let crowded = (lo..hi).any(|j| j != s && (al.rt[ord[j]] - al.rt[g]).abs() <= rt);
        out[g] = !crowded;
    }
    out
}

/// Rows that anchor the map: strong, narrow in RT across files, isolated, seen in all files (or in 75 % when too few).
fn anchors(al: &Aligned, p: &RtParams) -> Vec<usize> {
    let nf = al.n_files;
    let iso = isolated(al, p.crowd_ppm, p.crowd_rt);
    let base: Vec<usize> = (0..al.len())
        .filter(|&r| {
            let cells = r * nf..(r + 1) * nf;
            let apex = &al.apex_rt[cells.clone()];
            let strong = nanmedian(
                apex.iter()
                    .zip(&al.height[cells])
                    .filter(|(a, _)| !a.is_nan())
                    .map(|(_, &h)| h as f64),
            );
            let seen = apex.iter().filter(|a| !a.is_nan()).map(|&a| a as f64);
            let (mn, mx) = seen.fold((f64::INFINITY, f64::NEG_INFINITY), |(l, h), v| {
                (l.min(v), h.max(v))
            });
            strong >= 10.0 * p.min_height && mx - mn <= p.max_spread && iso[r]
        })
        .collect();
    let seen_in = |r: usize| {
        al.apex_rt[r * nf..(r + 1) * nf]
            .iter()
            .filter(|a| !a.is_nan())
            .count()
    };
    let pick: Vec<usize> = base.iter().copied().filter(|&r| seen_in(r) == nf).collect();
    if pick.len() >= p.min_anchors {
        return pick;
    }
    let need = 3.max((0.75 * nf as f64).ceil() as usize);
    base.into_iter().filter(|&r| seen_in(r) >= need).collect()
}

/// Indices of the first row of each group of anchors of the curve (`np.unique(r_[arange(0, m, size), m])` with a short last group merged).
fn node_edges(m: usize) -> Vec<usize> {
    let size = 5.max(m / 12);
    let mut e: Vec<usize> = (0..m).step_by(size).collect();
    e.push(m);
    e.dedup();
    if e.len() > 2 && e[e.len() - 1] - e[e.len() - 2] < size / 2 {
        e.remove(e.len() - 2);
    }
    e
}

/// Builds the shift curves of the files of `al`. Without enough anchors the map is empty (`ok()` false, all shifts 0).
pub fn build(al: &Aligned, p: &RtParams) -> RtMap {
    let nf = al.n_files;
    let empty = RtMap {
        n_files: nf,
        rms: vec![0.0; nf],
        ..RtMap::default()
    };
    if al.is_empty() || nf < 2 {
        return empty;
    }
    let pick = anchors(al, p);
    if pick.len() < 3 {
        return empty;
    }
    let apex = |r: usize, k: usize| al.apex_rt[r * nf + k] as f64;
    let mut rows: Vec<(f64, usize)> = pick
        .iter()
        .map(|&r| (nanmedian((0..nf).map(|k| apex(r, k))), r))
        .collect();
    sortc::by(&mut rows, |a, b| a.0.total_cmp(&b.0));
    let m = rows.len();
    let d = |i: usize, k: usize| apex(rows[i].1, k) - rows[i].0;
    let edges = node_edges(m);
    let nodes = edges.len() - 1;
    let mut grid: Vec<f64> = (0..nodes)
        .map(|j| nanmedian((edges[j]..edges[j + 1]).map(|i| rows[i].0)))
        .collect();
    let mut cols: Vec<Vec<f64>> = (0..nf)
        .map(|k| {
            (0..nodes)
                .map(|j| {
                    let v = nanmedian((edges[j]..edges[j + 1]).map(|i| d(i, k)));
                    if v.is_nan() {
                        0.0
                    } else {
                        v
                    }
                })
                .collect()
        })
        .collect();
    if rows[0].0 < grid[0] {
        grid.insert(0, rows[0].0);
        cols.iter_mut().for_each(|c| c.insert(0, c[0]));
    }
    if rows[m - 1].0 > grid[grid.len() - 1] {
        grid.push(rows[m - 1].0);
        cols.iter_mut().for_each(|c| c.push(c[c.len() - 1]));
    }
    let rms = (0..nf)
        .map(|k| residual(&rows, &grid, &cols[k], |i| d(i, k)))
        .collect();
    RtMap {
        n_files: nf,
        shift: cols.concat(),
        grid,
        n_anchors: m,
        rms,
    }
}

/// Root mean square of the anchors of a file against its curve (0 without anchors in that file).
fn residual(rows: &[(f64, usize)], grid: &[f64], curve: &[f64], d: impl Fn(usize) -> f64) -> f64 {
    let (mut s, mut n) = (0.0, 0usize);
    for (i, r) in rows.iter().enumerate() {
        let v = d(i);
        if !v.is_nan() {
            let e = v - interp(r.0, grid, curve);
            s += e * e;
            n += 1;
        }
    }
    if n == 0 {
        0.0
    } else {
        (s / n as f64).sqrt()
    }
}
