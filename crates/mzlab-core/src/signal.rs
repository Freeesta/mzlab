//! Signal helpers of the peak picker: noise, Savitzky-Golay (with derivatives), SNIP and AsLS baselines, Ricker CWT.
//! Line-by-line ports of the helpers of `ionfamily.py`; the Python engine is the reference.
pub(crate) fn median(v: &[f64]) -> f64 {
    if v.is_empty() {
        return 0.0;
    }
    let mut s = v.to_vec();
    crate::sortc::by(&mut s, |a, b| {
        a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal)
    });
    let n = s.len();
    if n % 2 == 1 {
        s[n / 2]
    } else {
        (s[n / 2 - 1] + s[n / 2]) / 2.0
    }
}

/// Robust noise sigma from the first differences (MAD).
pub fn noise_level(y: &[f64]) -> f64 {
    if y.len() < 4 {
        return 0.0;
    }
    let d: Vec<f64> = y.windows(2).map(|w| w[1] - w[0]).collect();
    let m = median(&d);
    let dev: Vec<f64> = d.iter().map(|v| (v - m).abs()).collect();
    1.4826 * median(&dev) / 2f64.sqrt()
}

/// Solves A x = b (A square, row-major, n x n) by Gaussian elimination with partial pivoting. None if singular.
pub(crate) fn solve(mut a: Vec<f64>, mut b: Vec<f64>, n: usize) -> Option<Vec<f64>> {
    for c in 0..n {
        let mut p = c;
        for r in c + 1..n {
            if a[r * n + c].abs() > a[p * n + c].abs() {
                p = r;
            }
        }
        if a[p * n + c].abs() < 1e-300 {
            return None;
        }
        if p != c {
            for k in 0..n {
                a.swap(c * n + k, p * n + k);
            }
            b.swap(c, p);
        }
        let d = a[c * n + c];
        for r in c + 1..n {
            let f = a[r * n + c] / d;
            if f != 0.0 {
                for k in c..n {
                    a[r * n + k] -= f * a[c * n + k];
                }
                b[r] -= f * b[c];
            }
        }
    }
    let mut x = vec![0.0; n];
    for r in (0..n).rev() {
        let mut s = b[r];
        for k in r + 1..n {
            s -= a[r * n + k] * x[k];
        }
        x[r] = s / a[r * n + r];
    }
    Some(x)
}

/// Savitzky-Golay convolution coefficients (odd window), derivative order `deriv`, unit spacing, ordered like `np.convolve` wants.
pub(crate) fn savgol_coeffs(window: usize, poly: usize, deriv: usize) -> Vec<f64> {
    let half = (window / 2) as i64;
    let m = poly + 1;
    // pinv(A)[deriv] = row `deriv` of (A^T A)^-1 A^T, A[i][j] = x_i^j
    let xs: Vec<f64> = (-half..=half).map(|v| v as f64).collect();
    let mut ata = vec![0.0; m * m];
    for j in 0..m {
        for k in 0..m {
            ata[j * m + k] = xs.iter().map(|x| x.powi((j + k) as i32)).sum();
        }
    }
    let mut e = vec![0.0; m];
    e[deriv] = 1.0;
    let w = solve(ata, e, m).unwrap_or_else(|| vec![0.0; m]);
    let fact: f64 = (1..=deriv).map(|v| v as f64).product();
    let mut c: Vec<f64> = xs
        .iter()
        .map(|x| fact * (0..m).map(|j| w[j] * x.powi(j as i32)).sum::<f64>())
        .collect();
    c.reverse();
    c
}

pub(crate) fn reflect(i: i64, n: i64) -> usize {
    // numpy "reflect" padding (the edge is not repeated); valid while |offset| < n
    let mut j = i;
    if j < 0 {
        j = -j;
    }
    if j > n - 1 {
        j = 2 * (n - 1) - j;
    }
    j as usize
}

pub fn savgol(y: &[f64], window: usize, poly: usize) -> Vec<f64> {
    savgol_d(y, window, poly, 0)
}

/// `ionfamily.savgol` with the derivative order `deriv` (unit spacing).
pub fn savgol_d(y: &[f64], window: usize, poly: usize, deriv: usize) -> Vec<f64> {
    let n = y.len();
    let mut window = window | 1;
    if n < 3 {
        return y.to_vec();
    }
    window = window.min(if n % 2 == 1 { n } else { n - 1 });
    let poly = poly.min(window - 1);
    let half = window / 2;
    let c = savgol_coeffs(window, poly, deriv);
    let ni = n as i64;
    (0..n)
        .map(|i| {
            (0..window)
                .map(|k| {
                    let src = i as i64 + k as i64 - half as i64;
                    let v = if n > half {
                        y[reflect(src, ni)]
                    } else {
                        y[src.clamp(0, ni - 1) as usize]
                    };
                    v * c[window - 1 - k]
                })
                .sum()
        })
        .collect()
}

pub(crate) fn snip_baseline(y: &[f64], iters: usize) -> Vec<f64> {
    let n = y.len();
    let mut v: Vec<f64> = y
        .iter()
        .map(|&x| (((x.max(0.0) + 1.0).sqrt() + 1.0).ln() + 1.0).ln())
        .collect();
    let top = iters.min((n / 2).saturating_sub(1).max(1));
    for w in 1..=top {
        let a: Vec<f64> = (0..n)
            .map(|i| {
                let right = if i + w < n { v[i + w] } else { v[n - 1] };
                let left = if i >= w { v[i - w] } else { v[0] };
                (right + left) / 2.0
            })
            .collect();
        for i in 0..n {
            v[i] = v[i].min(a[i]);
        }
    }
    v.iter()
        .map(|&x| ((x.exp() - 1.0).exp() - 1.0).powi(2) - 1.0)
        .collect()
}

fn interp(x: f64, xp: &[f64], fp: &[f64]) -> f64 {
    if x <= xp[0] {
        return fp[0];
    }
    if x >= xp[xp.len() - 1] {
        return fp[fp.len() - 1];
    }
    let j = xp.partition_point(|&v| v <= x) - 1;
    fp[j] + (x - xp[j]) * (fp[j + 1] - fp[j]) / (xp[j + 1] - xp[j])
}

pub(crate) fn asls_baseline(y: &[f64], lam: f64, p: f64, iters: usize, max_n: usize) -> Vec<f64> {
    let n = y.len();
    if n < 5 {
        let m = y.iter().cloned().fold(f64::INFINITY, f64::min);
        return vec![if n == 0 { 0.0 } else { m }; n];
    }
    if n > 20000 {
        return snip_baseline(y, 20);
    }
    let k = n.div_ceil(max_n).max(1);
    if k > 1 {
        let m = n / k;
        let yd: Vec<f64> = (0..m)
            .map(|b| y[b * k..(b + 1) * k].iter().sum::<f64>() / k as f64)
            .collect();
        let xd: Vec<f64> = (0..m).map(|b| (b as f64 + 0.5) * k as f64 - 0.5).collect();
        let zd = asls_baseline(&yd, lam / (k as f64).powi(4), p, iters, usize::MAX / 4);
        return (0..n)
            .map(|i| interp(i as f64, &xd, &zd).min(y[i]))
            .collect();
    }
    // P = lam * D2^T D2 (D2 = second differences)
    let mut pm = vec![0.0; n * n];
    for r in 0..n - 2 {
        let c = [(r, 1.0), (r + 1, -2.0), (r + 2, 1.0)];
        for &(i, vi) in &c {
            for &(j, vj) in &c {
                pm[i * n + j] += lam * vi * vj;
            }
        }
    }
    let mut w = vec![1.0; n];
    let mut z = y.to_vec();
    for _ in 0..iters {
        let mut a = pm.clone();
        for i in 0..n {
            a[i * n + i] += w[i];
        }
        let b: Vec<f64> = (0..n).map(|i| w[i] * y[i]).collect();
        z = match solve(a, b, n) {
            Some(s) => s,
            None => break,
        };
        let w_new: Vec<f64> = (0..n)
            .map(|i| if y[i] > z[i] { p } else { 1.0 - p })
            .collect();
        if w_new == w {
            break;
        }
        w = w_new;
    }
    (0..n).map(|i| z[i].min(y[i])).collect()
}

pub(crate) fn ricker(width: f64, n: usize) -> Vec<f64> {
    let h = (n / 2) as i64;
    (-h..=h)
        .map(|x| {
            let x = x as f64;
            (1.0 - (x / width).powi(2)) * (-0.5 * (x / width).powi(2)).exp() / width.sqrt()
        })
        .collect()
}

pub(crate) fn cwt_ricker(y: &[f64], widths: &[f64]) -> Vec<Vec<f64>> {
    let n = y.len();
    widths
        .iter()
        .map(|&w| {
            let k = ((10.0 * w).min(n as f64 - 1.0) as usize) | 1;
            let wv = ricker(w, k);
            let h = (k / 2) as i64;
            (0..n)
                .map(|i| {
                    // np.convolve(np.pad(y, h, edge), wv, valid): the kernel is symmetric
                    (0..k)
                        .map(|j| {
                            let src = (i as i64 + j as i64 - h).clamp(0, n as i64 - 1) as usize;
                            y[src] * wv[k - 1 - j]
                        })
                        .sum::<f64>()
                })
                .collect()
        })
        .collect()
}
