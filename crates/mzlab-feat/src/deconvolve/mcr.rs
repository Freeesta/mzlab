//! Linear algebra, SIMPLISMA seeding, the MCR-ALS loop and the profile interpolation used by `deconvolve_isomers`.

use super::*;

fn det_mat(mut a: Vec<Vec<f64>>) -> f64 {
    let n = a.len();
    let mut det = 1.0;
    for i in 0..n {
        let mut p = i;
        for r in i + 1..n {
            if a[r][i].abs() > a[p][i].abs() {
                p = r;
            }
        }
        if a[p][i].abs() < 1e-15 {
            return 0.0;
        }
        if p != i {
            a.swap(i, p);
            det = -det;
        }
        let piv = a[i][i];
        det *= piv;
        for r in i + 1..n {
            let f = a[r][i] / piv;
            for c in i..n {
                a[r][c] -= f * a[i][c];
            }
        }
    }
    det
}

fn inv_mat(a: &[Vec<f64>]) -> Option<Vec<Vec<f64>>> {
    let n = a.len();
    let mut m = vec![vec![0.0; 2 * n]; n];
    for i in 0..n {
        m[i][..n].copy_from_slice(&a[i]);
        m[i][n + i] = 1.0;
    }
    for i in 0..n {
        let mut p = i;
        for r in i + 1..n {
            if m[r][i].abs() > m[p][i].abs() {
                p = r;
            }
        }
        if m[p][i].abs() < 1e-15 {
            return None;
        }
        m.swap(i, p);
        let piv = m[i][i];
        for c in 0..2 * n {
            m[i][c] /= piv;
        }
        for r in 0..n {
            if r != i {
                let f = m[r][i];
                for c in 0..2 * n {
                    m[r][c] -= f * m[i][c];
                }
            }
        }
    }
    Some((0..n).map(|i| m[i][n..].to_vec()).collect())
}

fn simplisma(d: &[Vec<f64>], k: usize, offset: f64) -> Vec<usize> {
    let (n, m) = (d.len(), d.first().map_or(0, |r| r.len()));
    if n == 0 || m == 0 {
        return Vec::new();
    }
    let (mut mu, mut sd) = (vec![0.0; m], vec![0.0; m]);
    for j in 0..m {
        let sum: f64 = (0..n).map(|i| d[i][j]).sum();
        let mean = sum / (n as f64);
        mu[j] = mean;
        sd[j] = ((0..n).map(|i| (d[i][j] - mean).powi(2)).sum::<f64>() / (n as f64)).sqrt();
    }
    let alpha = offset * mu.iter().fold(0.0f64, |acc, &x| acc.max(x));
    let purity: Vec<f64> = (0..m).map(|j| sd[j] / (mu[j] + alpha)).collect();
    let dn: Vec<Vec<f64>> = (0..n)
        .map(|i| {
            (0..m)
                .map(|j| {
                    d[i][j] / ((mu[j].powi(2) + sd[j].powi(2) + 1e-300).sqrt() * (n as f64).sqrt())
                })
                .collect()
        })
        .collect();

    let mut sel: Vec<usize> = Vec::with_capacity(k);
    for _ in 0..k.min(m) {
        let (mut best, mut best_j) = (-1.0f64, 0);
        for j in 0..m {
            if sel.contains(&j) {
                continue;
            }
            let mut cols = vec![j];
            cols.extend_from_slice(&sel);
            let p = cols.len();
            let mut g = vec![vec![0.0; p]; p];
            for a in 0..p {
                for b in 0..p {
                    g[a][b] = (0..n).map(|i| dn[i][cols[a]] * dn[i][cols[b]]).sum();
                }
            }
            let sc = purity[j] * det_mat(g);
            if sc > best {
                best = sc;
                best_j = j;
            }
        }
        sel.push(best_j);
    }
    sel
}

fn transpose(a: &[Vec<f64>]) -> Vec<Vec<f64>> {
    let (n, m) = (a.len(), a.first().map_or(0, |r| r.len()));
    (0..m).map(|j| (0..n).map(|i| a[i][j]).collect()).collect()
}

fn ridge_solve(m_mat: &[Vec<f64>], f_mat: &[Vec<f64>]) -> Vec<Vec<f64>> {
    let (n, m, k) = (
        m_mat.len(),
        f_mat.len(),
        f_mat.first().map_or(0, |r| r.len()),
    );
    let mut ftf = vec![vec![0.0; k]; k];
    for a in 0..k {
        for b in 0..k {
            let sum: f64 = (0..m).map(|l| f_mat[l][a] * f_mat[l][b]).sum();
            ftf[a][b] = sum + if a == b { 1e-12 } else { 0.0 };
        }
    }
    let inv = inv_mat(&ftf).unwrap_or_else(|| {
        (0..k)
            .map(|a| (0..k).map(|b| if a == b { 1.0 } else { 0.0 }).collect())
            .collect()
    });
    let mut mf = vec![vec![0.0; k]; n];
    for i in 0..n {
        for j in 0..k {
            mf[i][j] = (0..m).map(|l| m_mat[i][l] * f_mat[l][j]).sum();
        }
    }
    let mut out = vec![vec![0.0; k]; n];
    for i in 0..n {
        for j in 0..k {
            out[i][j] = (0..k).map(|l| mf[i][l] * inv[l][j]).sum::<f64>().max(0.0);
        }
    }
    out
}

fn update_c(d: &[Vec<f64>], s: &[Vec<f64>], unimodal: bool) -> Vec<Vec<f64>> {
    let mut c = ridge_solve(d, s);
    if unimodal {
        let (n, k) = (c.len(), c.first().map_or(0, |r| r.len()));
        for j in 0..k {
            let col: Vec<f64> = (0..n).map(|i| c[i][j]).collect();
            if col.iter().fold(0.0f64, |acc, &x| acc.max(x)) > 0.0 {
                let iso = isotonic_up_down(&col);
                for (i, &v) in iso.iter().enumerate().take(n) {
                    c[i][j] = v;
                }
            }
        }
    }
    c
}

fn update_s(dt: &[Vec<f64>], c: &[Vec<f64>]) -> Vec<Vec<f64>> {
    let mut s = ridge_solve(dt, c);
    let (m, k) = (s.len(), s.first().map_or(0, |r| r.len()));
    for j in 0..k {
        let mx = (0..m).map(|i| s[i][j]).fold(0.0f64, |acc, x| acc.max(x));
        let scale = if mx <= 0.0 { 1.0 } else { mx };
        for i in 0..m {
            s[i][j] /= scale;
        }
    }
    s
}

fn calc_lof(d: &[Vec<f64>], c: &[Vec<f64>], s: &[Vec<f64>]) -> f64 {
    let (n, m, k) = (
        d.len(),
        d.first().map_or(0, |r| r.len()),
        c.first().map_or(0, |r| r.len()),
    );
    let (mut sum_r2, mut sum_d2) = (0.0, 0.0);
    for i in 0..n {
        for j in 0..m {
            let pred: f64 = (0..k).map(|l| c[i][l] * s[j][l]).sum();
            let r = d[i][j] - pred;
            sum_r2 += r * r;
            sum_d2 += d[i][j] * d[i][j];
        }
    }
    99.0 * (sum_r2 / sum_d2.max(1e-300)).sqrt()
}

pub(super) fn mcr_als(
    d: &[Vec<f64>],
    k: usize,
    unimodal: bool,
    s0: Option<Vec<Vec<f64>>>,
) -> (Vec<Vec<f64>>, Vec<Vec<f64>>, f64) {
    let (n, m) = (d.len(), d.first().map_or(0, |r| r.len()));
    let dt = transpose(d);
    let mut s = match s0 {
        Some(init) => init,
        None => {
            let idx = simplisma(d, k, 0.05);
            let mut init = vec![vec![0.0; k]; m];
            for (col, &j) in idx.iter().enumerate() {
                let col_vals: Vec<f64> = (0..n).map(|i| d[i][j]).collect();
                let bi = argmax(&col_vals);
                for f in 0..m {
                    init[f][col] = d[bi][f];
                }
            }
            init
        }
    };
    for row in &mut s {
        for v in row.iter_mut() {
            *v = v.max(1e-12);
        }
    }
    let mut prev: Option<f64> = None;
    let mut c = vec![vec![0.0; k]; n];
    for _ in 1..=300 {
        c = update_c(d, &s, unimodal);
        s = update_s(&dt, &c);
        let cur = calc_lof(d, &update_c(d, &s, unimodal), &s);
        if let Some(p) = prev {
            if (p - cur).abs() < 1e-6 * p.max(1e-12) {
                break;
            }
        }
        prev = Some(cur);
    }
    c = update_c(d, &s, unimodal);
    let lof = calc_lof(d, &c, &s);
    (c, s, lof)
}

pub(super) fn interp_linear(x: &[f64], xp: &[f64], fp: &[f64]) -> Vec<f64> {
    if xp.is_empty() || fp.is_empty() {
        return vec![0.0; x.len()];
    }
    let n = xp.len().min(fp.len());
    let mut out = Vec::with_capacity(x.len());
    for &t in x {
        if t <= xp[0] {
            out.push(fp[0]);
        } else if t >= xp[n - 1] {
            out.push(fp[n - 1]);
        } else {
            let mut idx = 0;
            while idx + 1 < n && xp[idx + 1] < t {
                idx += 1;
            }
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
