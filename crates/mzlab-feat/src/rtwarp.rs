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
        rms.push(residual(&rows, &grid, &curve, |i| {
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

/// `line` with every weight 1 over the points `it` (walked twice), without the weight vector; the same sums in the same order.
fn line_unit<'a>(it: impl Iterator<Item = &'a (f64, f64)> + Clone) -> (f64, f64) {
    let (mut sw, mut sx, mut sy) = (0.0, 0.0, 0.0);
    for &(x, y) in it.clone() {
        sw += 1.0;
        sx += x;
        sy += y;
    }
    if sw <= 0.0 {
        return (0.0, 0.0);
    }
    let (mx, my) = (sx / sw, sy / sw);
    let (mut sxx, mut sxy) = (0.0, 0.0);
    for &(x, y) in it {
        sxx += (x - mx) * (x - mx);
        sxy += (x - mx) * (y - my);
    }
    if sxx < 1e-12 {
        return (my, 0.0);
    }
    let b = sxy / sxx;
    (my - b * mx, b)
}

/// Tricube scale of a point: its distance to `at` over the bandwidth `h` (a hair wider so the farthest of the `q` still counts).
fn scaled(x: f64, at: f64, h: f64) -> f64 {
    (x - at).abs() / (h * 1.0000001)
}

/// Weight of a point at scaled distance `u` with robustness weight `r`.
fn tricube(u: f64, r: f64) -> f64 {
    if u < 1.0 {
        (1.0 - u * u * u).powi(3) * r
    } else {
        0.0
    }
}

/// Local linear fit at `at` with the `q` nearest points, tricube weights times the robustness weights `rw`.
/// `pts` must be sorted by x with finite coordinates: the points that weigh anything are then one window, found by
/// walking out from `at`, and the fit adds them in the order of `pts` (the zero weights outside add nothing).
fn local(pts: &[(f64, f64)], rw: &[f64], at: f64, q: usize, wbuf: &mut Vec<f64>) -> f64 {
    let n = pts.len();
    let mut hi = pts.partition_point(|p| p.0 < at);
    let mut lo = hi;
    let mut h = 0.0;
    for _ in 0..q.min(n) {
        let dl = if lo > 0 {
            at - pts[lo - 1].0
        } else {
            f64::INFINITY
        };
        let dr = if hi < n {
            pts[hi].0 - at
        } else {
            f64::INFINITY
        };
        if dl <= dr {
            lo -= 1;
            h = dl;
        } else {
            h = dr;
            hi += 1;
        }
    }
    let h = h.max(1e-9);
    while lo > 0 && scaled(pts[lo - 1].0, at, h) < 1.0 {
        lo -= 1;
    }
    while hi < n && scaled(pts[hi].0, at, h) < 1.0 {
        hi += 1;
    }
    wbuf.clear();
    wbuf.extend((lo..hi).map(|i| tricube(scaled(pts[i].0, at, h), rw[i])));
    let (a, b) = line(&pts[lo..hi], wbuf);
    a + b * at
}

/// The same fit as `local` over every point (sort of the distances, one weight per point), for `pts` that are not sorted or finite.
fn local_all(pts: &[(f64, f64)], rw: &[f64], at: f64, q: usize) -> f64 {
    let mut d: Vec<f64> = pts.iter().map(|p| (p.0 - at).abs()).collect();
    sortc::by(&mut d, f64::total_cmp);
    let h = d[q.min(d.len()) - 1].max(1e-9);
    let w: Vec<f64> = pts
        .iter()
        .zip(rw)
        .map(|(p, r)| tricube(scaled(p.0, at, h), *r))
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
    let windowed = thin.iter().all(|p| p.0.is_finite() && p.1.is_finite())
        && thin.windows(2).all(|w| w[0].0 <= w[1].0);
    let mut wbuf = Vec::new();
    let mut fit = |rw: &[f64], at: f64| {
        if windowed {
            local(&thin, rw, at, q, &mut wbuf)
        } else {
            local_all(&thin, rw, at, q)
        }
    };
    let mut rw = vec![1.0; thin.len()];
    for _ in 0..2 {
        let res: Vec<f64> = thin.iter().map(|p| p.1 - fit(&rw, p.0)).collect();
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
    grid.iter().map(|&x| fit(&rw, x)).collect()
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
    let (a, b) = line_unit(pts.iter().filter(|p| (p.1 - (a + b * p.0)).abs() <= inlier));
    grid.iter().map(|&x| a + b * x).collect()
}

#[cfg(test)]
mod tests {
    //! The windowed LOESS and the weight-free RANSAC refit against the verbatim previous code, bit for bit.
    use super::*;

    mod before {
        use super::super::{nanmedian, sortc, MAX_LOESS, MIN_ANCHORS, TRIALS};
        // Weighted least-squares line `y = a + b x` of the points with weights `w`; the weighted mean when `x` does not vary.
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

        // Local linear fit at `at` with the `q` nearest points, tricube weights times the robustness weights `rw`.
        fn local_old(pts: &[(f64, f64)], rw: &[f64], at: f64, q: usize) -> f64 {
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

        // The LOESS curve of `pts` (reference RT, shift) at the nodes `grid`; constant 0 without points.
        pub(super) fn loess_old(pts: &[(f64, f64)], grid: &[f64], span: f64) -> Vec<f64> {
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
                    .map(|p| p.1 - local_old(&thin, &rw, p.0, q))
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
            grid.iter().map(|&x| local_old(&thin, &rw, x, q)).collect()
        }

        // xorshift64: the samples of RANSAC come from a fixed seed, so a run is the same every time.
        fn next_old(s: &mut u64) -> u64 {
            *s ^= *s << 13;
            *s ^= *s >> 7;
            *s ^= *s << 17;
            *s
        }

        // The RANSAC line of `pts` at the nodes `grid`: the line through two points that most points agree with (within `inlier`), refitted on them.
        // The median shift (a constant) when fewer than `MIN_ANCHORS` agree.
        pub(super) fn ransac_old(
            pts: &[(f64, f64)],
            grid: &[f64],
            inlier: f64,
            salt: u64,
        ) -> Vec<f64> {
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
                    (next_old(&mut seed) % n as u64) as usize,
                    (next_old(&mut seed) % n as u64) as usize,
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
    }

    struct Rng(u64);

    impl Rng {
        fn next(&mut self) -> u64 {
            self.0 ^= self.0 << 13;
            self.0 ^= self.0 >> 7;
            self.0 ^= self.0 << 17;
            self.0
        }
        fn unit(&mut self) -> f64 {
            (self.next() >> 11) as f64 / (1u64 << 53) as f64
        }
    }

    fn bits(v: &[f64]) -> Vec<u64> {
        v.iter().map(|x| x.to_bits()).collect()
    }

    /// Anchors sorted by reference RT, with many repeated RTs and a few outliers.
    fn random_points(rng: &mut Rng, n: usize, repeats: bool) -> Vec<(f64, f64)> {
        let mut pts: Vec<(f64, f64)> = (0..n)
            .map(|_| {
                let x = if repeats {
                    (rng.unit() * 12.0).floor() * 0.5
                } else {
                    rng.unit() * 20.0
                };
                let out = if rng.unit() < 0.05 { 0.4 } else { 0.0 };
                (
                    x,
                    0.1 * x + 0.02 * (x * 0.7).sin() + (rng.unit() - 0.5) * 0.03 + out,
                )
            })
            .collect();
        pts.sort_by(|a, b| a.0.total_cmp(&b.0));
        pts
    }

    #[test]
    fn loess_window_equals_the_full_scan_on_10000_curves() {
        let mut rng = Rng(0xC0FFEE);
        for case in 0..10_000 {
            let n = [1, 2, 3, 4, 7, 20, 60, 450][case % 8];
            let pts = random_points(&mut rng, n, case % 3 != 0);
            let grid: Vec<f64> = (0..NODES).map(|j| -1.0 + 22.0 * j as f64 / 32.0).collect();
            let span = [0.05, 0.2, 0.5, 1.0][case % 4];
            assert_eq!(
                bits(&before::loess_old(&pts, &grid, span)),
                bits(&loess_curve(&pts, &grid, span)),
                "case {case}"
            );
        }
    }

    #[test]
    fn loess_falls_back_on_non_finite_or_unsorted_points() {
        let grid = [0.0, 1.0, 2.0];
        let mut pts = vec![
            (0.0, 0.1),
            (1.0, 0.2),
            (f64::INFINITY, 0.3),
            (2.0, 0.4),
            (3.0, 0.5),
        ];
        assert_eq!(
            bits(&before::loess_old(&pts, &grid, 0.6)),
            bits(&loess_curve(&pts, &grid, 0.6))
        );
        pts = vec![(2.0, 0.4), (0.0, 0.1), (1.0, 0.2), (3.0, 0.5)];
        assert_eq!(
            bits(&before::loess_old(&pts, &grid, 0.6)),
            bits(&loess_curve(&pts, &grid, 0.6))
        );
    }

    #[test]
    fn ransac_refit_equals_the_weighted_one() {
        let mut rng = Rng(99);
        for case in 0..3000 {
            let n = [0, 2, 3, 5, 30, 200][case % 6];
            let pts = random_points(&mut rng, n, case % 2 == 0);
            let grid: Vec<f64> = (0..NODES).map(|j| 20.0 * j as f64 / 32.0).collect();
            let inlier = [0.01, 0.05, 0.2][case % 3];
            assert_eq!(
                bits(&before::ransac_old(&pts, &grid, inlier, case as u64)),
                bits(&ransac_curve(&pts, &grid, inlier, case as u64)),
                "case {case}"
            );
        }
    }
}
