//! `robust = ransac`: the model that most observations agree with. Every subset of as many observations as the model has terms is tried (or a fixed number of subsets drawn with a
//! fixed generator when there are too many), the one with the most inliers wins (ties: the smaller sum of residuals, then the first), and the inliers are fitted by least squares.
//! Deterministic: no clock, no system generator.

use super::fit::{Fit, Model, Obs};
use super::ols::{mad_scale, residuals, wls};

/// Subsets tried at most.
const MAX_SUBSETS: usize = 500;
/// An observation within this many ppm of the model (or three robust standard deviations of the errors, if more) is an inlier.
const MIN_TOL: f64 = 1.0;
const ROUNDS: usize = 5;

/// SplitMix64: a fixed, portable generator.
struct Rng(u64);

impl Rng {
    fn next(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }
}

/// How many subsets of `p` among `n` there are, capped just above `MAX_SUBSETS`.
fn count(n: usize, p: usize) -> usize {
    let mut c = 1usize;
    for i in 0..p {
        c = c.saturating_mul(n - i) / (i + 1);
        if c > MAX_SUBSETS {
            return MAX_SUBSETS + 1;
        }
    }
    c
}

/// The subsets to try, each a sorted list of `p` positions: all of them in lexicographic order, or `MAX_SUBSETS` drawn.
fn subsets(n: usize, p: usize) -> Vec<Vec<usize>> {
    if count(n, p) <= MAX_SUBSETS {
        let mut out = Vec::new();
        let mut cur: Vec<usize> = (0..p).collect();
        loop {
            out.push(cur.clone());
            let Some(i) = (0..p).rev().find(|&i| cur[i] < n - p + i) else {
                break;
            };
            cur[i] += 1;
            for j in i + 1..p {
                cur[j] = cur[j - 1] + 1;
            }
        }
        return out;
    }
    let mut rng = Rng(0x5EED ^ (n as u64) << 8 ^ p as u64);
    (0..MAX_SUBSETS)
        .map(|_| {
            let mut pool: Vec<usize> = (0..n).collect();
            for i in 0..p {
                let j = i + (rng.next() % (n - i) as u64) as usize;
                pool.swap(i, j);
            }
            let mut s = pool[..p].to_vec();
            crate::sortc::ord(&mut s);
            s
        })
        .collect()
}

pub(super) fn fit(model: Model, obs: &[Obs]) -> Option<Fit> {
    let (n, p) = (obs.len(), model.n_params());
    if n < p || p == 0 {
        return None;
    }
    let errors: Vec<f64> = obs.iter().map(|o| o.err).collect();
    let tol = MIN_TOL.max(3.0 * mad_scale(&errors));
    // (inliers, sum of |residual| of the inliers, coefficients)
    let mut best: Option<(usize, f64, [f64; 3])> = None;
    for s in subsets(n, p) {
        let picked: Vec<Obs> = s.iter().map(|&i| obs[i]).collect();
        let Some(coef) = wls(model, &picked, &vec![1.0; p]) else {
            continue;
        };
        let r = residuals(model, &coef, obs);
        let inl = r.iter().filter(|x| x.abs() <= tol).count();
        let sum: f64 = r.iter().filter(|x| x.abs() <= tol).map(|x| x.abs()).sum();
        if best.is_none_or(|(bi, bs, _)| inl > bi || (inl == bi && sum < bs)) {
            best = Some((inl, sum, coef));
        }
    }
    let (_, _, mut coef) = best?;
    let mut inlier: Vec<bool> = Vec::new();
    for _ in 0..ROUNDS {
        let r = residuals(model, &coef, obs);
        let now: Vec<bool> = r.iter().map(|x| x.abs() <= tol).collect();
        let keep: Vec<Obs> = obs
            .iter()
            .zip(&now)
            .filter(|&(_, &k)| k)
            .map(|(o, _)| *o)
            .collect();
        let w: Vec<f64> = keep.iter().map(|o| o.w).collect();
        let refit = wls(model, &keep, &w);
        let same = now == inlier;
        inlier = now;
        match refit {
            Some(c) if !same => coef = c,
            _ => break,
        }
    }
    let r = residuals(model, &coef, obs);
    let kept: Vec<f64> = r
        .iter()
        .zip(&inlier)
        .filter(|&(_, &k)| k)
        .map(|(x, _)| *x)
        .collect();
    let dof = kept.len() as f64 - p as f64;
    let scale = if dof > 0.0 {
        (kept.iter().map(|x| x * x).sum::<f64>() / dof).sqrt()
    } else {
        0.0
    };
    Some(Fit {
        coef,
        inlier,
        scale,
        model,
    })
}
