//! Regularised L2 logistic regression via IRLS for in-source fragmentation (ISF).
//! Port of `fit_logistic` and `predict_logit` from `TP_Mine/py/tpmine/isf.py`.

/// Fitted L2-regularised logistic regression model.
#[derive(Debug, Clone, PartialEq)]
pub struct LogisticModel {
    pub mean: Vec<f64>,
    pub std: Vec<f64>,
    pub coef: Vec<f64>,
    pub intercept: f64,
}

/// Validates input dimensions for logistic regression.
fn validate_inputs(
    x: &[Vec<f64>],
    y: &[f64],
    weights: Option<&[f64]>,
) -> Result<(usize, usize), &'static str> {
    let n = x.len();
    if n == 0 {
        return Err("x is empty");
    }
    let m = match x.first() {
        Some(first_row) => first_row.len(),
        None => return Err("x is empty"),
    };
    if m == 0 {
        return Err("features are empty");
    }
    for row in x {
        if row.len() != m {
            return Err("inconsistent feature row length");
        }
    }
    if y.len() != n {
        return Err("y length mismatch");
    }
    if let Some(w) = weights {
        if w.len() != n {
            return Err("weights length mismatch");
        }
    }
    Ok((n, m))
}

/// Computes column-wise nanmean and nanstd (ddof=0). If std < 1e-9, sets std to 1.0.
fn compute_mean_std(x: &[Vec<f64>], n: usize, m: usize) -> (Vec<f64>, Vec<f64>) {
    let mut mean = vec![0.0; m];
    let mut std = vec![0.0; m];
    for (j, (mu_out, sd_out)) in mean.iter_mut().zip(std.iter_mut()).enumerate() {
        let mut count = 0usize;
        let mut sum = 0.0;
        for row in x.iter().take(n) {
            let v = row[j];
            if !v.is_nan() {
                count += 1;
                sum += v;
            }
        }
        let mu = if count > 0 {
            sum / count as f64
        } else {
            f64::NAN
        };
        let mut sum_sq = 0.0;
        if count > 0 {
            for row in x.iter().take(n) {
                let v = row[j];
                if !v.is_nan() {
                    let diff = v - mu;
                    sum_sq += diff * diff;
                }
            }
        }
        let mut sd = if count > 0 {
            (sum_sq / count as f64).sqrt()
        } else {
            f64::NAN
        };
        if sd < 1e-9 {
            sd = 1.0;
        }
        *mu_out = mu;
        *sd_out = sd;
    }
    (mean, std)
}

/// Builds the flat row-major design matrix A (n x (m + 1)) with intercept column A[i][0] = 1.0.
fn build_design_matrix(x: &[Vec<f64>], mean: &[f64], std: &[f64], n: usize, m: usize) -> Vec<f64> {
    let k = m + 1;
    let mut a = vec![0.0; n * k];
    for (i, row) in x.iter().enumerate().take(n) {
        let row_offset = i * k;
        a[row_offset] = 1.0;
        for (j, &val) in row.iter().enumerate().take(m) {
            let z = if val.is_nan() {
                0.0
            } else {
                let diff = (val - mean[j]) / std[j];
                if diff.is_nan() {
                    0.0
                } else {
                    diff
                }
            };
            a[row_offset + 1 + j] = z;
        }
    }
    a
}

/// Computes residuals w * (y - p) and IRLS working weights diag(W).
fn compute_residuals_and_weights(
    a_mat: &[f64],
    beta: &[f64],
    y: &[f64],
    weights: Option<&[f64]>,
    n: usize,
    k: usize,
) -> (Vec<f64>, Vec<f64>) {
    let mut res = Vec::with_capacity(n);
    let mut w_diag = Vec::with_capacity(n);
    for i in 0..n {
        let row_offset = i * k;
        let mut eta = 0.0;
        for c in 0..k {
            eta += a_mat[row_offset + c] * beta[c];
        }
        let clipped = eta.clamp(-30.0, 30.0);
        let p_i = 1.0 / (1.0 + (-clipped).exp());
        let w_i = match weights {
            Some(w) => w[i],
            None => 1.0,
        };
        let cap_w = w_i * p_i * (1.0 - p_i) + 1e-9;
        let res_i = w_i * (y[i] - p_i);
        res.push(res_i);
        w_diag.push(cap_w);
    }
    (res, w_diag)
}

/// Computes the gradient vector g = A^T @ res - R @ beta.
fn compute_gradient(
    a_mat: &[f64],
    beta: &[f64],
    res: &[f64],
    lam: f64,
    n: usize,
    k: usize,
) -> Vec<f64> {
    let mut g = vec![0.0; k];
    for c in 0..k {
        let mut sum = 0.0;
        for (i, &r_val) in res.iter().enumerate().take(n) {
            sum += a_mat[i * k + c] * r_val;
        }
        if c >= 1 {
            sum -= lam * beta[c];
        }
        g[c] = sum;
    }
    g
}

/// Computes the Hessian matrix H = A^T @ diag(W) @ A + R (k x k, row-major).
fn compute_hessian(a_mat: &[f64], w_diag: &[f64], lam: f64, n: usize, k: usize) -> Vec<f64> {
    let mut h = vec![0.0; k * k];
    for (i, &wi) in w_diag.iter().enumerate().take(n) {
        let row_offset = i * k;
        for r in 0..k {
            let air = a_mat[row_offset + r];
            let factor = wi * air;
            for c in r..k {
                let aic = a_mat[row_offset + c];
                h[r * k + c] += factor * aic;
            }
        }
    }
    for r in 0..k {
        for c in 0..r {
            h[r * k + c] = h[c * k + r];
        }
    }
    for c in 1..k {
        h[c * k + c] += lam;
    }
    h
}

/// Solves linear system A x = b (square, row-major, n x n) with partial pivoting.
fn solve_linear_system(mut a: Vec<f64>, mut b: Vec<f64>, n: usize) -> Option<Vec<f64>> {
    for c in 0..n {
        let mut pivot = c;
        let mut max_val = a[c * n + c].abs();
        for r in (c + 1)..n {
            let val = a[r * n + c].abs();
            if val > max_val {
                max_val = val;
                pivot = r;
            }
        }
        if !(max_val >= 1e-300) {
            return None;
        }
        if pivot != c {
            for k in 0..n {
                a.swap(c * n + k, pivot * n + k);
            }
            b.swap(c, pivot);
        }
        let diag = a[c * n + c];
        for r in (c + 1)..n {
            let factor = a[r * n + c] / diag;
            if factor != 0.0 {
                for k in c..n {
                    a[r * n + k] -= factor * a[c * n + k];
                }
                b[r] -= factor * b[c];
            }
        }
    }
    let mut x = vec![0.0; n];
    for r in (0..n).rev() {
        let mut sum = b[r];
        for k in (r + 1)..n {
            sum -= a[r * n + k] * x[k];
        }
        x[r] = sum / a[r * n + r];
    }
    Some(x)
}

/// Fits L2-regularised logistic regression model using iteratively reweighted least squares (IRLS).
pub fn fit_logistic(
    x: &[Vec<f64>],
    y: &[f64],
    lam: f64,
    weights: Option<&[f64]>,
    n_iter: usize,
) -> Result<LogisticModel, &'static str> {
    let (n, m) = validate_inputs(x, y, weights)?;
    let (mean, std) = compute_mean_std(x, n, m);
    let a_mat = build_design_matrix(x, &mean, &std, n, m);
    let k = m + 1;
    let mut beta = vec![0.0; k];

    for _ in 0..n_iter {
        let (res, w_diag) = compute_residuals_and_weights(&a_mat, &beta, y, weights, n, k);
        let g = compute_gradient(&a_mat, &beta, &res, lam, n, k);
        let h = compute_hessian(&a_mat, &w_diag, lam, n, k);

        let step = match solve_linear_system(h, g, k) {
            Some(s) => s,
            None => return Err("singular matrix in IRLS"),
        };

        let mut max_abs_step: f64 = 0.0;
        for (b, &s) in beta.iter_mut().zip(&step) {
            *b += s;
            let abs_s = s.abs();
            if abs_s > max_abs_step {
                max_abs_step = abs_s;
            }
        }
        if max_abs_step < 1e-8 {
            break;
        }
    }

    let intercept = beta[0];
    let coef = beta[1..].to_vec();
    Ok(LogisticModel {
        mean,
        std,
        coef,
        intercept,
    })
}

/// Predicts log-odds and per-feature contributions from standardized features.
pub fn predict_logit(feat: &[f64], model: &LogisticModel) -> (f64, Vec<f64>) {
    let m = model.coef.len();
    let mut contrib = Vec::with_capacity(m);
    let mut sum_contrib = 0.0;
    for j in 0..m {
        let f = match feat.get(j) {
            Some(&v) => v,
            None => f64::NAN,
        };
        let mu = match model.mean.get(j) {
            Some(&v) => v,
            None => 0.0,
        };
        let sd = match model.std.get(j) {
            Some(&v) => v,
            None => 1.0,
        };
        let c = match model.coef.get(j) {
            Some(&v) => v,
            None => 0.0,
        };
        let z = if f.is_nan() {
            0.0
        } else {
            let diff = (f - mu) / sd;
            if diff.is_nan() {
                0.0
            } else {
                diff
            }
        };
        let contrib_j = z * c;
        sum_contrib += contrib_j;
        contrib.push(contrib_j);
    }
    let logit = model.intercept + sum_contrib;
    (logit, contrib)
}
