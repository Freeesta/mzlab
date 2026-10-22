//! Correction of the retention time between the files of a series: the same peak does not come out at the same RT in every file, and the drift can be
//! larger than the RT gap that groups features. A coarse alignment (wide RT gap) finds anchors, peaks that every file shows and nothing crowds;
//! for each file a curve of the apex RT minus the reference RT against the reference RT is fitted (piecewise linear by blocks, LOESS or RANSAC line)
//! and kept as an `RtMap`, which `align_grouped` then uses to bring every feature to the reference RT before it groups them. Used by `famalign`,
//! `series` and `slabs`; no signal is moved, only the RT the grouping reads.

use crate::align::{align_grouped, AlignParams};
use crate::rtmap::{anchors, nanmedian, residual, RtMap, RtParams};
use crate::sortc;
use crate::table::{Aligned, FileFeatures};
use mzlab_core::error::Result;

/// How the curve of a file is fitted to its anchors.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Method {
    /// No warping: the alignment reads the RT as the files give it (in "per family" mode the map of `rtmap`, as before).
    Off,
    /// Median of the anchors by blocks, joined by straight lines (`rtmap::build`).
    Piecewise,
    /// Local linear regression with tricube weights and two robust passes.
    Loess,
    /// Line through the largest set of anchors that agree (random sample consensus), refitted on them.
    Ransac,
}

impl Method {
    pub const NAMES: [&'static str; 4] = ["off", "piecewise", "loess", "ransac"];

    pub fn name(self) -> &'static str {
        Self::NAMES[self as usize]
    }

    pub fn parse(s: &str) -> Option<Method> {
        [
            Method::Off,
            Method::Piecewise,
            Method::Loess,
            Method::Ransac,
        ]
        .into_iter()
        .find(|m| m.name() == s)
    }
}

/// Parameters of the correction.
#[derive(Debug, Clone, Copy)]
pub struct WarpParams {
    pub method: Method,
    /// RT gap (minutes) of the coarse alignment that finds the anchors; also the widest RT spread of an anchor over the files.
    pub coarse_gap: f64,
    /// LOESS: fraction of the anchors each local fit takes.
    pub span: f64,
    /// RANSAC: largest distance (minutes) of an anchor from the line to count as agreeing with it.
    pub inlier: f64,
}

impl Default for WarpParams {
    fn default() -> Self {
        WarpParams {
            method: Method::Off,
            coarse_gap: 0.5,
            span: 0.5,
            inlier: 0.05,
        }
    }
}

impl WarpParams {
    pub fn on(&self) -> bool {
        self.method != Method::Off
    }
}

/// Nodes of the curve of a file.
const NODES: usize = 33;
/// Anchors a LOESS fit reads at most (evenly taken along the reference RT), so that the cost does not grow with the table.
const MAX_LOESS: usize = 400;
/// Samples of RANSAC.
const TRIALS: usize = 200;
/// Fewest anchors that make a curve.
const MIN_ANCHORS: usize = 3;

/// The parameters of the alignment that finds the anchors: the same but for the wide RT gap and no cut of close isomers.
pub fn coarse(ion: &AlignParams, w: &WarpParams) -> AlignParams {
    AlignParams {
        rt_gap: ion.rt_gap.max(w.coarse_gap),
        max_rt_span: None,
        split_gap: None,
        ..*ion
    }
}

/// Alignment "per ion" with the RT corrected by the curves of `w`; the files are aligned once coarsely to find the anchors.
pub fn align_warped(
    files: &[&FileFeatures],
    ion: &AlignParams,
    rt: &RtParams,
    w: &WarpParams,
) -> Result<Aligned> {
    let map = warp_map(files, ion, rt, w)?;
    Ok(align_grouped(files, ion, map.ok().then_some(&map), false)?.table)
}

/// The curves of the files of `files` (empty, `ok()` false, without enough anchors).
pub fn warp_map(
    files: &[&FileFeatures],
    ion: &AlignParams,
    rt: &RtParams,
    w: &WarpParams,
) -> Result<RtMap> {
    let first = align_grouped(files, &coarse(ion, w), None, false)?.table;
    Ok(map_from_table(&first, rt, w))
}

/// The curves fitted to the anchors of the coarse table `al`.
pub fn map_from_table(al: &Aligned, rt: &RtParams, w: &WarpParams) -> RtMap {
    let rt = RtParams {
        max_spread: w.coarse_gap.max(rt.max_spread),
        ..*rt
    };
    if w.method == Method::Piecewise || w.method == Method::Off {
        return crate::rtmap::build(al, &rt);
    }
    let nf = al.n_files;
    let empty = RtMap {
        n_files: nf,
        rms: vec![0.0; nf],
        ..RtMap::default()
    };
    if al.is_empty() || nf < 2 {
        return empty;
    }
    let pick = anchors(al, &rt);
    if pick.len() < MIN_ANCHORS {
        return empty;
    }
    let apex = |r: usize, k: usize| al.apex_rt[r * nf + k] as f64;
    let mut rows: Vec<(f64, usize)> = pick
        .iter()
        .map(|&r| (nanmedian((0..nf).map(|k| apex(r, k))), r))
        .collect();
    sortc::by(&mut rows, |a, b| a.0.total_cmp(&b.0));
    let (lo, hi) = (rows[0].0, rows[rows.len() - 1].0);
    if hi - lo <= 0.0 {
        return empty;
    }
    let grid: Vec<f64> = (0..NODES)
        .map(|j| lo + (hi - lo) * j as f64 / (NODES - 1) as f64)
        .collect();
    let mut shift = Vec::with_capacity(nf * NODES);
    let mut rms = Vec::with_capacity(nf);
    for k in 0..nf {
        let pts: Vec<(f64, f64)> = rows
            .iter()
            .filter(|(_, r)| !apex(*r, k).is_nan())
            .map(|&(x, r)| (x, apex(r, k) - x))
            .collect();
        let curve = match w.method {
            Method::Loess => loess_curve(&pts, &grid, w.span),
            _ => ransac_curve(&pts, &grid, w.inlier, k as u64),
        };
        let all: Vec<(f64, usize)> = rows.clone();
        rms.push(residual(&all, &grid, &curve, |i| {
            let r = rows[i].1;
            if apex(r, k).is_nan() {
                f64::NAN
            } else {
                apex(r, k) - rows[i].0
            }
        }));
        shift.extend(curve);
    }
    RtMap {
        n_files: nf,
        grid,
        shift,
        n_anchors: rows.len(),
        rms,
    }
}

/// Weighted least-squares line `y = a + b x` of the points with weights `w`; the weighted mean when `x` does not vary.
fn line(pts: &[(f64, f64)], w: &[f64]) -> (f64, f64) {
    let (mut sw, mut sx, mut sy) = (0.0, 0.0, 0.0);
    for (&(x, y), &wi) in pts.iter().zip(w) {
        sw += wi;
        sx += wi * x;
        sy += wi * y;
    }
    if sw <= 0.0 {
        return (0.0, 0.0);
    }
    let (mx, my) = (sx / sw, sy / sw);
    let (mut sxx, mut sxy) = (0.0, 0.0);
    for (&(x, y), &wi) in pts.iter().zip(w) {
        sxx += wi * (x - mx) * (x - mx);
        sxy += wi * (x - mx) * (y - my);
    }
    if sxx < 1e-12 {
        return (my, 0.0);
    }
    let b = sxy / sxx;
    (my - b * mx, b)
}

/// Local linear fit at `at` with the `q` nearest points, tricube weights times the robustness weights `rw`.
fn local(pts: &[(f64, f64)], rw: &[f64], at: f64, q: usize) -> f64 {
    let mut d: Vec<f64> = pts.iter().map(|p| (p.0 - at).abs()).collect();
    sortc::by(&mut d, f64::total_cmp);
    let h = d[q.min(d.len()) - 1].max(1e-9);
    let w: Vec<f64> = pts
        .iter()
        .zip(rw)
        .map(|(p, r)| {
            let u = (p.0 - at).abs() / (h * 1.0000001);
            if u < 1.0 {
                (1.0 - u * u * u).powi(3) * r
            } else {
                0.0
            }
        })
        .collect();
    let (a, b) = line(pts, &w);
    a + b * at
}

/// The LOESS curve of `pts` (reference RT, shift) at the nodes `grid`; constant 0 without points.
fn loess_curve(pts: &[(f64, f64)], grid: &[f64], span: f64) -> Vec<f64> {
    if pts.is_empty() {
        return vec![0.0; grid.len()];
    }
    let step = pts.len().div_ceil(MAX_LOESS);
    let thin: Vec<(f64, f64)> = pts.iter().copied().step_by(step).collect();
    let q = ((span.clamp(0.05, 1.0) * thin.len() as f64).ceil() as usize).max(3);
    let mut rw = vec![1.0; thin.len()];
    for _ in 0..2 {
        let res: Vec<f64> = thin
            .iter()
            .map(|p| p.1 - local(&thin, &rw, p.0, q))
            .collect();
        let s = nanmedian(res.iter().map(|e| e.abs()));
        if s.is_nan() || s <= 1e-9 {
            break;
        }
        rw = res
            .iter()
            .map(|e| {
                let u = e / (6.0 * s);
                if u.abs() < 1.0 {
                    (1.0 - u * u).powi(2)
                } else {
                    0.0
                }
            })
            .collect();
    }
    grid.iter().map(|&x| local(&thin, &rw, x, q)).collect()
}

/// xorshift64: the samples of RANSAC come from a fixed seed, so a run is the same every time.
fn next(s: &mut u64) -> u64 {
    *s ^= *s << 13;
    *s ^= *s >> 7;
    *s ^= *s << 17;
    *s
}

/// The RANSAC line of `pts` at the nodes `grid`: the line through two points that most points agree with (within `inlier`), refitted on them.
/// The median shift (a constant) when fewer than `MIN_ANCHORS` agree.
fn ransac_curve(pts: &[(f64, f64)], grid: &[f64], inlier: f64, salt: u64) -> Vec<f64> {
    let n = pts.len();
    if n == 0 {
        return vec![0.0; grid.len()];
    }
    let median = nanmedian(pts.iter().map(|p| p.1));
    let constant = || vec![median; grid.len()];
    if n < MIN_ANCHORS {
        return constant();
    }
    let span = pts.iter().map(|p| p.0).fold(f64::NEG_INFINITY, f64::max)
        - pts.iter().map(|p| p.0).fold(f64::INFINITY, f64::min);
    let mut seed = 0x9E37_79B9_7F4A_7C15u64 ^ salt.wrapping_mul(0x2545_F491_4F6C_DD1D) | 1;
    let mut best: Option<(usize, f64, f64, f64)> = None;
    for _ in 0..TRIALS {
        let (i, j) = (
            (next(&mut seed) % n as u64) as usize,
            (next(&mut seed) % n as u64) as usize,
        );
        if (pts[i].0 - pts[j].0).abs() < 0.05 * span {
            continue;
        }
        let b = (pts[j].1 - pts[i].1) / (pts[j].0 - pts[i].0);
        let a = pts[i].1 - b * pts[i].0;
        let (count, err) = pts.iter().fold((0usize, 0.0f64), |(c, e), p| {
            let r = (p.1 - (a + b * p.0)).abs();
            if r <= inlier {
                (c + 1, e + r)
            } else {
                (c, e)
            }
        });
        if best.is_none_or(|(c, e, _, _)| count > c || (count == c && err < e)) {
            best = Some((count, err, a, b));
        }
    }
    let Some((_, _, a, b)) = best.filter(|b| b.0 >= MIN_ANCHORS) else {
        return constant();
    };
    let inl: Vec<(f64, f64)> = pts
        .iter()
        .copied()
        .filter(|p| (p.1 - (a + b * p.0)).abs() <= inlier)
        .collect();
    let (a, b) = line(&inl, &vec![1.0; inl.len()]);
    grid.iter().map(|&x| a + b * x).collect()
}
