//! Signal helpers of the peak picker: noise, Savitzky-Golay (with derivatives), SNIP and AsLS baselines, Ricker CWT.
//! Line-by-line ports of the helpers of `ionfamily.py`; the Python engine is the reference.
pub fn median(v: &[f64]) -> f64 {
    if v.is_empty() {
        return 0.0;
    }
    let mut s = v.to_vec();
    if let Some(m) = crate::sortc::select_median(&mut s) {
        return m;
    }
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

/// `median` of the values of `v` that goes through `scratch` (kept between calls, so a loop of medians allocates once).
pub fn median_in(v: impl Iterator<Item = f64>, scratch: &mut Vec<f64>) -> f64 {
    scratch.clear();
    scratch.extend(v);
    if scratch.is_empty() {
        return 0.0;
    }
    if let Some(m) = crate::sortc::select_median(scratch) {
        return m;
    }
    median(scratch)
}

/// Robust noise sigma from the first differences (MAD).
pub fn noise_level(y: &[f64]) -> f64 {
    if y.len() < 4 {
        return 0.0;
    }
    let mut buf: Vec<f64> = y.windows(2).map(|w| w[1] - w[0]).collect();
    // one buffer for both medians: a selection reorders it, and the sort of the NaN fallback depends on the order, so that case is rebuilt
    let (m, reordered) = match crate::sortc::select_median(&mut buf) {
        Some(m) => (m, true),
        None => (median(&buf), false),
    };
    for v in buf.iter_mut() {
        *v = (*v - m).abs();
    }
    let mad = match crate::sortc::select_median(&mut buf) {
        Some(mad) => mad,
        None => {
            if reordered {
                for (v, w) in buf.iter_mut().zip(y.windows(2)) {
                    *v = ((w[1] - w[0]) - m).abs();
                }
            }
            median(&buf)
        }
    };
    1.4826 * mad / 2f64.sqrt()
}

/// Gaussian elimination with partial pivoting on `a` (n x n, row-major) and `b`, both overwritten; the solution goes to `x`. False if singular.
fn solve_in(a: &mut [f64], b: &mut [f64], x: &mut [f64], n: usize) -> bool {
    for c in 0..n {
        let mut p = c;
        for r in c + 1..n {
            if a[r * n + c].abs() > a[p * n + c].abs() {
                p = r;
            }
        }
        if a[p * n + c].abs() < 1e-300 {
            return false;
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
    for r in (0..n).rev() {
        let mut s = b[r];
        for k in r + 1..n {
            s -= a[r * n + k] * x[k];
        }
        x[r] = s / a[r * n + r];
    }
    true
}

/// Solves A x = b (A square, row-major, n x n) by Gaussian elimination with partial pivoting. None if singular.
pub fn solve(mut a: Vec<f64>, mut b: Vec<f64>, n: usize) -> Option<Vec<f64>> {
    let mut x = vec![0.0; n];
    solve_in(&mut a, &mut b, &mut x, n).then_some(x)
}

/// `solve` for 3 x 3 on the stack (the quadratic fit of a peak apex): the same operations in the same order.
pub fn solve3(mut a: [f64; 9], mut b: [f64; 3]) -> Option<[f64; 3]> {
    let mut x = [0.0; 3];
    solve_in(&mut a, &mut b, &mut x, 3).then_some(x)
}

/// Slots per row of the band storage of the AsLS system: two sub-diagonals, the diagonal and four super-diagonals (two of the matrix and two of fill-in
/// from the row swaps of partial pivoting); the entry (r, k) is at `r * BAND + k + 2 - r`.
const BAND: usize = 7;

/// Slot of the entry (r, k) in band storage.
#[inline]
fn at(r: usize, k: usize) -> usize {
    r * BAND + k + 2 - r
}

/// Solves A x = b for the AsLS matrix held in band storage `ab` (n rows of `BAND` slots), overwriting `ab` and `b`. It runs the same operations in the same
/// order as `solve` on the dense matrix, skipping only the exact zeros outside the band that partial pivoting can reach (lower 2, upper 4), so the
/// result is bit-identical. None if singular.
fn solve_band(ab: &mut [f64], b: &mut [f64], n: usize) -> Option<Vec<f64>> {
    for c in 0..n {
        let last = (c + 2).min(n - 1);
        let mut p = c;
        for r in c + 1..=last {
            if ab[at(r, c)].abs() > ab[at(p, c)].abs() {
                p = r;
            }
        }
        if ab[at(p, c)].abs() < 1e-300 {
            return None;
        }
        let hi = (c + 4).min(n - 1);
        if p != c {
            for k in c..=hi {
                ab.swap(at(c, k), at(p, k));
            }
            b.swap(c, p);
        }
        let d = ab[at(c, c)];
        for r in c + 1..=last {
            let f = ab[at(r, c)] / d;
            if f != 0.0 {
                for k in c..=hi {
                    ab[at(r, k)] -= f * ab[at(c, k)];
                }
                b[r] -= f * b[c];
            }
        }
    }
    let mut x = vec![0.0; n];
    for r in (0..n).rev() {
        let mut s = b[r];
        for k in r + 1..=(r + 4).min(n - 1) {
            s -= ab[at(r, k)] * x[k];
        }
        x[r] = s / ab[at(r, r)];
    }
    Some(x)
}

/// The penalty `lam * D2^T D2` (D2 = second differences) in band storage, each entry summed over the rows of D2 in order.
fn penalty_band(n: usize, lam: f64) -> Vec<f64> {
    let mut pb = vec![0.0; n * BAND];
    for r in 0..n - 2 {
        let c = [(r, 1.0), (r + 1, -2.0), (r + 2, 1.0)];
        for &(i, vi) in &c {
            for &(j, vj) in &c {
                pb[at(i, j)] += lam * vi * vj;
            }
        }
    }
    pb
}

/// The dense n x n matrix of a band (the rare fallback of `asls_baseline`, and the tests).
fn dense_of_band(ab: &[f64], n: usize) -> Vec<f64> {
    let mut a = vec![0.0; n * n];
    for i in 0..n {
        for j in i.saturating_sub(2)..(i + 3).min(n) {
            a[i * n + j] = ab[at(i, j)];
        }
    }
    a
}

/// Savitzky-Golay convolution coefficients (odd window), derivative order `deriv`, unit spacing, ordered like `np.convolve` wants.
pub fn savgol_coeffs(window: usize, poly: usize, deriv: usize) -> Vec<f64> {
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

pub fn reflect(i: i64, n: i64) -> usize {
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
    // taps of the point `i` that reach outside the trace read reflected (or clamped) samples; inside, the window is a plain slice
    let edge = |i: usize| -> f64 {
        (0..window)
            .map(|k| {
                let src = i as i64 + k as i64 - half as i64;
                let v = if (0..ni).contains(&src) {
                    y[src as usize]
                } else if n > half {
                    y[reflect(src, ni)]
                } else {
                    y[src.clamp(0, ni - 1) as usize]
                };
                v * c[window - 1 - k]
            })
            .sum()
    };
    let (first, last) = (half.min(n), n.saturating_sub(half).max(half.min(n)));
    let mut out = Vec::with_capacity(n);
    out.extend((0..first).map(edge));
    out.extend((first..last).map(|i| {
        y[i - half..i - half + window]
            .iter()
            .zip(c.iter().rev())
            .map(|(&v, &ck)| v * ck)
            .sum::<f64>()
    }));
    out.extend((last..n).map(edge));
    out
}

pub fn snip_baseline(y: &[f64], iters: usize) -> Vec<f64> {
    let n = y.len();
    let mut v: Vec<f64> = y
        .iter()
        .map(|&x| (((x.max(0.0) + 1.0).sqrt() + 1.0).ln() + 1.0).ln())
        .collect();
    let top = iters.min((n / 2).saturating_sub(1).max(1));
    let mut a = vec![0.0; n];
    for w in 1..=top {
        for (i, ai) in a.iter_mut().enumerate() {
            let right = if i + w < n { v[i + w] } else { v[n - 1] };
            let left = if i >= w { v[i - w] } else { v[0] };
            *ai = (right + left) / 2.0;
        }
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

/// Asymmetric least squares baseline: `iters` weighted smoothing solves of a pentadiagonal system. More than `max_n` points are averaged down first.
pub fn asls_baseline(y: &[f64], lam: f64, p: f64, iters: usize, max_n: usize) -> Vec<f64> {
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
    let pb = penalty_band(n, lam);
    let mut ab = vec![0.0; n * BAND];
    let mut w = vec![1.0; n];
    let mut z = y.to_vec();
    for _ in 0..iters {
        ab.copy_from_slice(&pb);
        for i in 0..n {
            ab[at(i, i)] += w[i];
        }
        let mut b: Vec<f64> = (0..n).map(|i| w[i] * y[i]).collect();
        // a singular or non-finite band solve is redone on the dense matrix, where a stray infinity could still reach the zeros outside the band
        let solved = solve_band(&mut ab, &mut b, n).filter(|x| x.iter().all(|v| v.is_finite()));
        z = match solved {
            Some(s) => s,
            None => {
                let mut a = dense_of_band(&pb, n);
                for i in 0..n {
                    a[i * n + i] += w[i];
                }
                let b: Vec<f64> = (0..n).map(|i| w[i] * y[i]).collect();
                match solve(a, b, n) {
                    Some(s) => s,
                    None => break,
                }
            }
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

pub fn ricker(width: f64, n: usize) -> Vec<f64> {
    let h = (n / 2) as i64;
    (-h..=h)
        .map(|x| {
            let x = x as f64;
            (1.0 - (x / width).powi(2)) * (-0.5 * (x / width).powi(2)).exp() / width.sqrt()
        })
        .collect()
}

pub fn cwt_ricker(y: &[f64], widths: &[f64]) -> Vec<Vec<f64>> {
    let n = y.len();
    widths
        .iter()
        .map(|&w| {
            let k = ((10.0 * w).min(n as f64 - 1.0) as usize) | 1;
            let wv = ricker(w, k);
            let h = (k / 2) as i64;
            // np.convolve(np.pad(y, h, edge), wv, valid): the kernel is symmetric; near the ends the samples are clamped to the edge
            let edge = |i: usize| -> f64 {
                (0..k)
                    .map(|j| {
                        let src = (i as i64 + j as i64 - h).clamp(0, n as i64 - 1) as usize;
                        y[src] * wv[k - 1 - j]
                    })
                    .sum::<f64>()
            };
            let h = h as usize;
            let (first, last) = (h.min(n), n.saturating_sub(h).max(h.min(n)));
            let mut out = Vec::with_capacity(n);
            out.extend((0..first).map(edge));
            out.extend((first..last).map(|i| {
                y[i - h..i - h + k]
                    .iter()
                    .zip(wv.iter().rev())
                    .map(|(&v, &c)| v * c)
                    .sum::<f64>()
            }));
            out.extend((last..n).map(edge));
            out
        })
        .collect()
}

#[cfg(test)]
mod tests;
