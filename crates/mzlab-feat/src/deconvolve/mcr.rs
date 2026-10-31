//! Linear algebra, SIMPLISMA seeding, the MCR-ALS loop and the profile interpolation used by `deconvolve_isomers`.
//! Matrices are row-major `&[f64]` with explicit dimensions; every scalar product keeps the summation order
//! of the nested-`Vec` version, so the results are bit-identical to it (`deconvolve/tests.rs` proves it).

use super::*;

fn dot(x: &[f64], y: &[f64]) -> f64 {
    x.iter().zip(y).map(|(a, b)| a * b).sum()
}

/// Determinant of the `n x n` matrix `a` by elimination with partial pivoting; destroys `a`.
fn det_mat(a: &mut [f64], n: usize) -> f64 {
    let mut det = 1.0;
    for i in 0..n {
        let mut p = i;
        for r in i + 1..n {
            if a[r * n + i].abs() > a[p * n + i].abs() {
                p = r;
            }
        }
        if a[p * n + i].abs() < 1e-15 {
            return 0.0;
        }
        if p != i {
            for c in 0..n {
                a.swap(i * n + c, p * n + c);
            }
            det = -det;
        }
        let piv = a[i * n + i];
        det *= piv;
        for r in i + 1..n {
            let f = a[r * n + i] / piv;
            for c in i..n {
                a[r * n + c] -= f * a[i * n + c];
            }
        }
    }
    det
}

/// Gauss-Jordan inverse of the `n x n` matrix `a`; `None` when a pivot is below 1e-15.
fn inv_mat(a: &[f64], n: usize) -> Option<Vec<f64>> {
    let w = 2 * n;
    let mut m = vec![0.0; n * w];
    for i in 0..n {
        m[i * w..i * w + n].copy_from_slice(&a[i * n..i * n + n]);
        m[i * w + n + i] = 1.0;
    }
    for i in 0..n {
        let mut p = i;
        for r in i + 1..n {
            if m[r * w + i].abs() > m[p * w + i].abs() {
                p = r;
            }
        }
        if m[p * w + i].abs() < 1e-15 {
            return None;
        }
        for c in 0..w {
            m.swap(i * w + c, p * w + c);
        }
        let piv = m[i * w + i];
        for c in 0..w {
            m[i * w + c] /= piv;
        }
        for r in 0..n {
            if r != i {
                let f = m[r * w + i];
                for c in 0..w {
                    m[r * w + c] -= f * m[i * w + c];
                }
            }
        }
    }
    let mut out = Vec::with_capacity(n * n);
    for i in 0..n {
        out.extend_from_slice(&m[i * w + n..i * w + w]);
    }
    Some(out)
}

/// Row-major transpose of the `n x m` matrix `a`.
fn transpose(a: &[f64], n: usize, m: usize) -> Vec<f64> {
    let mut out = vec![0.0; n * m];
    for i in 0..n {
        for j in 0..m {
            out[j * n + i] = a[i * m + j];
        }
    }
    out
}

/// Indices of the `k` purest columns of the `n x m` matrix whose transpose is `dt` (`m x n`).
fn simplisma(dt: &[f64], n: usize, m: usize, k: usize, offset: f64) -> Vec<usize> {
    if n == 0 || m == 0 {
        return Vec::new();
    }
    let (mut mu, mut sd) = (vec![0.0; m], vec![0.0; m]);
    for j in 0..m {
        let col = &dt[j * n..(j + 1) * n];
        let sum: f64 = col.iter().sum();
        let mean = sum / (n as f64);
        mu[j] = mean;
        sd[j] = (col.iter().map(|&x| (x - mean).powi(2)).sum::<f64>() / (n as f64)).sqrt();
    }
    let alpha = offset * mu.iter().fold(0.0f64, |acc, &x| acc.max(x));
    let purity: Vec<f64> = (0..m).map(|j| sd[j] / (mu[j] + alpha)).collect();
    // Column-normalised copy, one column per row so every scalar product walks contiguous memory.
    let mut dnt = vec![0.0; m * n];
    for j in 0..m {
        let den = (mu[j].powi(2) + sd[j].powi(2) + 1e-300).sqrt() * (n as f64).sqrt();
        for i in 0..n {
            dnt[j * n + i] = dt[j * n + i] / den;
        }
    }
    let col = |j: usize| &dnt[j * n..(j + 1) * n];

    let mut sel: Vec<usize> = Vec::with_capacity(k);
    // Gram matrix of the columns already chosen: it does not depend on the candidate.
    let mut gsel: Vec<f64> = Vec::new();
    let mut g: Vec<f64> = Vec::new();
    for _ in 0..k.min(m) {
        let (mut best, mut best_j) = (-1.0f64, 0);
        let (ps, p) = (sel.len(), sel.len() + 1);
        for j in 0..m {
            if sel.contains(&j) {
                continue;
            }
            g.clear();
            g.resize(p * p, 0.0);
            g[0] = dot(col(j), col(j));
            for b in 0..ps {
                let v = dot(col(j), col(sel[b]));
                g[b + 1] = v;
                g[(b + 1) * p] = v;
            }
            for a in 0..ps {
                for b in 0..ps {
                    g[(a + 1) * p + b + 1] = gsel[a * ps + b];
                }
            }
            let sc = purity[j] * det_mat(&mut g, p);
            if sc > best {
                best = sc;
                best_j = j;
            }
        }
        let ns = ps + 1;
        let mut next = vec![0.0; ns * ns];
        for a in 0..ps {
            for b in 0..ps {
                next[a * ns + b] = gsel[a * ps + b];
            }
        }
        for b in 0..ps {
            let v = dot(col(best_j), col(sel[b]));
            next[ps * ns + b] = v;
            next[b * ns + ps] = v;
        }
        next[ps * ns + ps] = dot(col(best_j), col(best_j));
        gsel = next;
        sel.push(best_j);
    }
    sel
}

/// Non-negative ridge least squares: `a (n x m) * f (m x k) * (f'f + 1e-12 I)^-1`, clamped at 0.
fn ridge_solve(a: &[f64], n: usize, m: usize, f: &[f64], k: usize) -> Vec<f64> {
    let ft = transpose(f, m, k);
    let mut ftf = vec![0.0; k * k];
    for x in 0..k {
        for y in 0..k {
            let sum = dot(&ft[x * m..(x + 1) * m], &ft[y * m..(y + 1) * m]);
            ftf[x * k + y] = sum + if x == y { 1e-12 } else { 0.0 };
        }
    }
    let inv = inv_mat(&ftf, k).unwrap_or_else(|| {
        let mut id = vec![0.0; k * k];
        for x in 0..k {
            id[x * k + x] = 1.0;
        }
        id
    });
    let invt = transpose(&inv, k, k);
    let mut mf = vec![0.0; k];
    let mut out = vec![0.0; n * k];
    for i in 0..n {
        let row = &a[i * m..(i + 1) * m];
        for j in 0..k {
            mf[j] = dot(row, &ft[j * m..(j + 1) * m]);
        }
        for j in 0..k {
            out[i * k + j] = dot(&mf, &invt[j * k..(j + 1) * k]).max(0.0);
        }
    }
    out
}

fn pav(v: &[f64]) -> Vec<f64> {
    let (mut vals, mut wts): (Vec<f64>, Vec<usize>) =
        (Vec::with_capacity(v.len()), Vec::with_capacity(v.len()));
    for &x in v {
        vals.push(x);
        wts.push(1);
        while vals.len() > 1 && vals[vals.len() - 2] > vals[vals.len() - 1] {
            let l = vals.len();
            let (tot, wt) = (
                vals[l - 2] * wts[l - 2] as f64 + vals[l - 1] * wts[l - 1] as f64,
                wts[l - 2] + wts[l - 1],
            );
            vals.pop();
            vals.pop();
            wts.pop();
            wts.pop();
            vals.push(tot / wt as f64);
            wts.push(wt);
        }
    }
    let mut out = Vec::with_capacity(v.len());
    for (val, wt) in vals.into_iter().zip(wts) {
        out.extend(std::iter::repeat_n(val, wt));
    }
    out
}

fn isotonic_up_down(c: &[f64]) -> Vec<f64> {
    if c.is_empty() {
        return Vec::new();
    }
    let k = argmax(c);
    let left = pav(&c[..=k]);
    let right_rev: Vec<f64> = c[k..].iter().copied().rev().collect();
    let right: Vec<f64> = pav(&right_rev).into_iter().rev().collect();
    let mut out = Vec::with_capacity(c.len());
    out.extend_from_slice(&left[..k]);
    out.push(if let (Some(&l), Some(&r)) = (left.get(k), right.first()) {
        l.max(r)
    } else {
        c[k]
    });
    if right.len() > 1 {
        out.extend_from_slice(&right[1..]);
    }
    out
}

fn update_c(d: &[f64], n: usize, m: usize, s: &[f64], k: usize, unimodal: bool) -> Vec<f64> {
    let mut c = ridge_solve(d, n, m, s, k);
    if unimodal {
        let mut col = vec![0.0; n];
        for j in 0..k {
            for i in 0..n {
                col[i] = c[i * k + j];
            }
            if col.iter().fold(0.0f64, |acc, &x| acc.max(x)) > 0.0 {
                let iso = isotonic_up_down(&col);
                for (i, &v) in iso.iter().enumerate().take(n) {
                    c[i * k + j] = v;
                }
            }
        }
    }
    c
}

fn update_s(dt: &[f64], m: usize, n: usize, c: &[f64], k: usize) -> Vec<f64> {
    let mut s = ridge_solve(dt, m, n, c, k);
    for j in 0..k {
        let mx = (0..m)
            .map(|i| s[i * k + j])
            .fold(0.0f64, |acc, x| acc.max(x));
        let scale = if mx <= 0.0 { 1.0 } else { mx };
        for i in 0..m {
            s[i * k + j] /= scale;
        }
    }
    s
}

/// Lack of fit of `d ~ c s'` as a percentage of the norm of `d`; `sum_d2` is the sum of squares of `d`.
fn calc_lof(d: &[f64], n: usize, m: usize, c: &[f64], s: &[f64], k: usize, sum_d2: f64) -> f64 {
    let mut sum_r2 = 0.0;
    for i in 0..n {
        let crow = &c[i * k..(i + 1) * k];
        for j in 0..m {
            let pred = dot(crow, &s[j * k..(j + 1) * k]);
            let r = d[i * m + j] - pred;
            sum_r2 += r * r;
        }
    }
    100.0 * (sum_r2 / sum_d2.max(1e-300)).sqrt()
}

/// MCR-ALS of the `n x m` matrix `d` with `k` components. Returns `(c, s, lof)` with `c` `n x k` and `s` `m x k`.
pub(super) fn mcr_als(
    d: &[f64],
    n: usize,
    m: usize,
    k: usize,
    unimodal: bool,
    s0: Option<Vec<f64>>,
) -> (Vec<f64>, Vec<f64>, f64) {
    let dt = transpose(d, n, m);
    let mut s = match s0 {
        Some(init) => init,
        None => {
            let idx = simplisma(&dt, n, m, k, 0.05);
            let mut init = vec![0.0; m * k];
            for (col, &j) in idx.iter().enumerate() {
                let bi = argmax(&dt[j * n..(j + 1) * n]);
                for f in 0..m {
                    init[f * k + col] = d[bi * m + f];
                }
            }
            init
        }
    };
    for v in s.iter_mut() {
        *v = v.max(1e-12);
    }
    let mut sum_d2 = 0.0;
    for i in 0..n {
        for j in 0..m {
            sum_d2 += d[i * m + j] * d[i * m + j];
        }
    }
    // The profiles of one iteration are the ones the convergence test measured at the end of the previous one.
    let mut prev: Option<f64> = None;
    let mut c = update_c(d, n, m, &s, k, unimodal);
    let mut lof = 0.0;
    for _ in 1..=300 {
        s = update_s(&dt, m, n, &c, k);
        c = update_c(d, n, m, &s, k, unimodal);
        let cur = calc_lof(d, n, m, &c, &s, k, sum_d2);
        lof = cur;
        if let Some(p) = prev {
            if (p - cur).abs() < 1e-6 * p.max(1e-12) {
                break;
            }
        }
        prev = Some(cur);
    }
    (c, s, lof)
}

/// Linear interpolation of `(xp, fp)` at `x`, clamped to the end values.
pub(super) fn interp_linear(x: &[f64], xp: &[f64], fp: &[f64]) -> Vec<f64> {
    if xp.is_empty() || fp.is_empty() {
        return vec![0.0; x.len()];
    }
    let n = xp.len().min(fp.len());
    let sorted = xp[..n].windows(2).all(|w| w[0] <= w[1]);
    let mut out = Vec::with_capacity(x.len());
    for &t in x {
        if t <= xp[0] {
            out.push(fp[0]);
        } else if t >= xp[n - 1] {
            out.push(fp[n - 1]);
        } else {
            // First xp[idx + 1] >= t, as the linear scan finds it; a non-monotone xp keeps the scan.
            let idx = if sorted {
                xp[..n].partition_point(|&v| v < t).saturating_sub(1)
            } else {
                let mut idx = 0;
                while idx + 1 < n && xp[idx + 1] < t {
                    idx += 1;
                }
                idx
            };
            let (x0, x1, y0, y1) = (xp[idx], xp[idx + 1], fp[idx], fp[idx + 1]);
            let frac = if (x1 - x0).abs() > 1e-12 {
                (t - x0) / (x1 - x0)
            } else {
                0.0
            };
            out.push(y0 + frac * (y1 - y0));
        }
    }
    out
}

#[cfg(test)]
mod lof_tests {
    use super::calc_lof;

    // `ionfamily._lof`: 100 * ||D - C S'|| / ||D||, here for a fixed 2 x 2 case worked by hand.
    #[test]
    fn lof_is_a_percentage_of_the_norm() {
        let d = [1.0, 2.0, 3.0, 4.0];
        let (c, s) = ([1.0, 0.0, 0.0, 1.0], [1.0, 0.0, 0.0, 1.0]);
        // Residual (0, 2, 3, 3): sqrt(22) / sqrt(30).
        let want = 100.0 * (22.0f64 / 30.0).sqrt();
        assert!((calc_lof(&d, 2, 2, &c, &s, 2, 30.0) - want).abs() < 1e-12);
    }
}
