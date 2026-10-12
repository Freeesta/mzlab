//! Small dense linear algebra for the statistics of the pages (`mzlab/statistica.py`, `ionfamily.estimate_components`): the thin SVD
//! (`np.linalg.svd(full_matrices=False)`) by one-sided Jacobi and the symmetric eigenproblem (`np.linalg.eigh`) by cyclic Jacobi.
//! Pure Rust, no dependency. Matrices are row-major `&[f64]`.
//!
//! The singular values and eigenvalues agree with LAPACK to about 1e-12 (relative to the largest). The SIGN of a singular vector or of an
//! eigenvector is arbitrary in LAPACK and here: callers that show vectors fix it themselves (as `statistica._svd` does); only the 2×2
//! `eigh` reproduces numpy's signs, because the ellipses of the PCA use the angle of the first axis. A null singular value gets
//! an orthonormal completion of its vectors (numpy's choice for those is arbitrary too).

/// Thin SVD of an `m × n` matrix: `a = u · diag(s) · vt`, `k = min(m, n)`, `s` descending.
pub struct Svd {
    pub u: Vec<f64>,
    pub s: Vec<f64>,
    pub vt: Vec<f64>,
    pub m: usize,
    pub n: usize,
    pub k: usize,
}

const EPS: f64 = 1e-15;

/// Hestenes' one-sided Jacobi on the columns of a tall matrix (`m >= n`, column-major as `Vec<Vec<f64>>`). Returns (columns scaled to
/// unit norm, norms, right vectors as columns of an n×n matrix).
fn jacobi_columns(mut c: Vec<Vec<f64>>) -> (Vec<Vec<f64>>, Vec<f64>, Vec<Vec<f64>>) {
    let n = c.len();
    let mut v: Vec<Vec<f64>> = (0..n)
        .map(|j| (0..n).map(|i| if i == j { 1.0 } else { 0.0 }).collect())
        .collect();
    for _sweep in 0..60 {
        let mut rotated = false;
        for p in 0..n {
            for q in p + 1..n {
                let (mut alpha, mut beta, mut gamma) = (0.0, 0.0, 0.0);
                for (x, y) in c[p].iter().zip(&c[q]) {
                    alpha += x * x;
                    beta += y * y;
                    gamma += x * y;
                }
                if gamma == 0.0 || gamma.abs() <= EPS * (alpha * beta).sqrt() {
                    continue;
                }
                rotated = true;
                let zeta = (beta - alpha) / (2.0 * gamma);
                let t = zeta.signum() / (zeta.abs() + (1.0 + zeta * zeta).sqrt());
                let cs = 1.0 / (1.0 + t * t).sqrt();
                let sn = cs * t;
                rotate(&mut c, p, q, cs, sn);
                rotate(&mut v, p, q, cs, sn);
            }
        }
        if !rotated {
            break;
        }
    }
    let norms: Vec<f64> = c
        .iter()
        .map(|x| x.iter().map(|y| y * y).sum::<f64>().sqrt())
        .collect();
    (c, norms, v)
}

/// Columns `p`, `q` replaced by their rotation (the same Givens rotation for the data and for the accumulated right vectors).
fn rotate(c: &mut [Vec<f64>], p: usize, q: usize, cs: f64, sn: f64) {
    let (lo, hi) = c.split_at_mut(q);
    for (x, y) in lo[p].iter_mut().zip(hi[0].iter_mut()) {
        let (a, b) = (*x, *y);
        *x = cs * a - sn * b;
        *y = sn * a + cs * b;
    }
}

/// Replaces every null (or missing) vector of `cols` by a unit vector orthogonal to the others (the first axis that is not in their span).
fn complete(cols: &mut [Vec<f64>], null: &[bool]) {
    let len = cols.first().map_or(0, Vec::len);
    let mut axis = 0;
    for j in 0..cols.len() {
        if !null[j] {
            continue;
        }
        while axis < len {
            let mut e = vec![0.0; len];
            e[axis] = 1.0;
            axis += 1;
            for _ in 0..2 {
                for (i, c) in cols.iter().enumerate() {
                    if i == j {
                        continue;
                    }
                    let d: f64 = c.iter().zip(&e).map(|(a, b)| a * b).sum();
                    e.iter_mut().zip(c).for_each(|(x, a)| *x -= d * a);
                }
            }
            let nrm = e.iter().map(|x| x * x).sum::<f64>().sqrt();
            if nrm > 1e-8 {
                cols[j] = e.iter().map(|x| x / nrm).collect();
                break;
            }
        }
    }
}

/// Thin SVD of the row-major `m × n` matrix `a`.
pub fn svd(a: &[f64], m: usize, n: usize) -> Svd {
    let k = m.min(n);
    if k == 0 {
        return Svd {
            u: Vec::new(),
            s: Vec::new(),
            vt: Vec::new(),
            m,
            n,
            k,
        };
    }
    let tall = m >= n;
    // work on a matrix with at least as many rows as columns: a itself, or its transpose
    let (rows, cols) = if tall { (m, n) } else { (n, m) };
    let c: Vec<Vec<f64>> = (0..cols)
        .map(|j| {
            (0..rows)
                .map(|i| if tall { a[i * n + j] } else { a[j * n + i] })
                .collect()
        })
        .collect();
    let (left, norms, mut right) = jacobi_columns(c);
    let mut order: Vec<usize> = (0..cols).collect();
    order.sort_by(|&x, &y| {
        norms[y]
            .partial_cmp(&norms[x])
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    let top = norms[order[0]];
    let null: Vec<bool> = order.iter().map(|&j| norms[j] <= top * 1e-14).collect();
    let mut l: Vec<Vec<f64>> = order
        .iter()
        .map(|&j| {
            if norms[j] > 0.0 {
                left[j].iter().map(|x| x / norms[j]).collect()
            } else {
                vec![0.0; rows]
            }
        })
        .collect();
    let mut r: Vec<Vec<f64>> = order
        .iter()
        .map(|&j| std::mem::take(&mut right[j]))
        .collect();
    complete(&mut l, &null);
    complete(&mut r, &vec![false; cols]);
    let s: Vec<f64> = order
        .iter()
        .zip(&null)
        .map(|(&j, &z)| if z { 0.0 } else { norms[j] })
        .collect();
    // a = l · s · rᵀ (tall) or a = r · s · lᵀ (wide)
    let (uc, vc) = if tall { (&l, &r) } else { (&r, &l) };
    let u: Vec<f64> = (0..m).flat_map(|i| uc.iter().map(move |c| c[i])).collect();
    let vt: Vec<f64> = vc.iter().flat_map(|c| c.iter().copied()).collect();
    Svd { u, s, vt, m, n, k }
}

/// Eigenvalues (ascending) and eigenvectors (columns of the row-major `n × n` second result) of the symmetric matrix `a`. For `n = 2` the
/// signs are those of numpy's `eigh` (the lower triangle is used); otherwise the sign is arbitrary.
pub fn eigh(a: &[f64], n: usize) -> (Vec<f64>, Vec<f64>) {
    if n == 2 {
        return eigh2(a[0], a[2], a[3]);
    }
    let mut m = a.to_vec();
    let mut v: Vec<f64> = (0..n * n)
        .map(|i| if i / n == i % n { 1.0 } else { 0.0 })
        .collect();
    for _sweep in 0..60 {
        let off: f64 = (0..n)
            .flat_map(|i| (0..i).map(move |j| (i, j)))
            .map(|(i, j)| m[i * n + j] * m[i * n + j])
            .sum();
        let diag: f64 = (0..n).map(|i| m[i * n + i] * m[i * n + i]).sum();
        if off <= 1e-32 * diag.max(1e-300) {
            break;
        }
        for p in 0..n {
            for q in p + 1..n {
                jacobi_step(&mut m, &mut v, n, p, q);
            }
        }
    }
    let mut order: Vec<usize> = (0..n).collect();
    order.sort_by(|&x, &y| {
        m[x * n + x]
            .partial_cmp(&m[y * n + y])
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    let w = order.iter().map(|&i| m[i * n + i]).collect();
    let vec = (0..n * n).map(|x| v[(x / n) * n + order[x % n]]).collect();
    (w, vec)
}

/// One Jacobi rotation that zeroes `m[p][q]` (the matrix and the accumulated eigenvectors are updated).
fn jacobi_step(m: &mut [f64], v: &mut [f64], n: usize, p: usize, q: usize) {
    let apq = m[p * n + q];
    if apq == 0.0 {
        return;
    }
    let theta = (m[q * n + q] - m[p * n + p]) / (2.0 * apq);
    let t = theta.signum() / (theta.abs() + (1.0 + theta * theta).sqrt());
    let c = 1.0 / (1.0 + t * t).sqrt();
    let s = t * c;
    for k in 0..n {
        let (x, y) = (m[k * n + p], m[k * n + q]);
        m[k * n + p] = c * x - s * y;
        m[k * n + q] = s * x + c * y;
    }
    for k in 0..n {
        let (x, y) = (m[p * n + k], m[q * n + k]);
        m[p * n + k] = c * x - s * y;
        m[q * n + k] = s * x + c * y;
    }
    for k in 0..n {
        let (x, y) = (v[k * n + p], v[k * n + q]);
        v[k * n + p] = c * x - s * y;
        v[k * n + q] = s * x + c * y;
    }
}

/// `[[a, b], [b, c]]`: eigenvalues ascending and the eigenvectors as numpy returns them (columns; row-major result).
fn eigh2(a: f64, b: f64, c: f64) -> (Vec<f64>, Vec<f64>) {
    if b == 0.0 {
        return if a > c {
            (vec![c, a], vec![0.0, 1.0, 1.0, 0.0])
        } else {
            (vec![a, c], vec![1.0, 0.0, 0.0, 1.0])
        };
    }
    let (mean, half) = ((a + c) / 2.0, ((a - c) / 2.0).hypot(b));
    let (lo, hi) = (mean - half, mean + half);
    // eigenvector of the larger eigenvalue ∝ (b, hi − a) (or (hi − c, b)); take the better conditioned one
    let (mut x, mut y) = if (hi - a).abs() >= (hi - c).abs() {
        (b, hi - a)
    } else {
        (hi - c, b)
    };
    let nrm = x.hypot(y);
    (x, y) = (x / nrm, y / nrm);
    // numpy (LAPACK, lower triangle): b < 0 → (−, +); b > 0 → (+, +) when a <= c, (−, −) when a > c
    let flip = if b < 0.0 {
        x > 0.0
    } else if a <= c {
        x < 0.0
    } else {
        x > 0.0
    };
    if flip {
        (x, y) = (-x, -y);
    }
    (vec![lo, hi], vec![-y, x, x, y])
}
