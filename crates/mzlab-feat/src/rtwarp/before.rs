//! Verbatim copy of the pre-optimisation LOESS and RANSAC refit, kept only so the unit tests can prove the new code is bit-identical.

use super::{nanmedian, sortc, MAX_LOESS, MIN_ANCHORS, TRIALS};
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
pub(super) fn ransac_old(pts: &[(f64, f64)], grid: &[f64], inlier: f64, salt: u64) -> Vec<f64> {
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
