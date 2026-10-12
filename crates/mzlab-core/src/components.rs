//! Number of chemical components of a matrix `D` (scans × channels) and the scan where each starts to appear
//! (`ionfamily.estimate_components`: singular values against the Marchenko-Pastur edge of the noise, and the forward singular values of
//! expanding windows, the evolving factor analysis). Pure Rust on top of `statmat::svd`; matrices are row-major `&[f64]`.

use crate::statmat::svd;

/// Result of [`estimate_components`]. `sigma` and `edge` are `None` when the matrix is too small to estimate them (`k = 1`, nothing else).
#[derive(Debug, PartialEq)]
pub struct Components {
    pub k: usize,
    pub sv: Vec<f64>,
    pub sigma: Option<f64>,
    pub edge: Option<f64>,
    pub efa_start: Vec<usize>,
}

/// Components of the `n × m` matrix `d`: a component counts when its singular value is above 1.3 times the noise edge
/// `sigma (sqrt(n) + sqrt(m))` and carries at least `min_var` of the variance; `k` is kept in `1..=max_k`.
pub fn estimate_components(d: &[f64], n: usize, m: usize, max_k: usize, min_var: f64) -> Components {
    if n < 4 || m < 2 {
        return Components { k: 1, sv: Vec::new(), sigma: None, edge: None, efa_start: Vec::new() };
    }
    let sv = svd(d, n, m).s;
    let sig = noise_sigma(d, n, m);
    let edge = sig * ((n as f64).sqrt() + (m as f64).sqrt());
    let total = sv.iter().map(|s| s * s).sum::<f64>().max(1e-300);
    let counted = if sig > 0.0 {
        sv.iter().filter(|&&s| s > 1.3 * edge && s * s / total >= min_var).count()
    } else {
        sv.iter().filter(|&&s| s > 0.02 * sv[0]).count()
    };
    let k = counted.clamp(1, max_k.min(sv.len()).max(1));
    let efa_start = evolving_starts(d, (n, m), edge, k);
    Components { k, sv: sv.iter().take(max_k + 1).copied().collect(), sigma: Some(sig), edge: Some(edge), efa_start }
}

/// The scans (0-based, the last row of the window) where the 1st, 2nd... component rises above the edge as the window `d[..t]` grows.
fn evolving_starts(d: &[f64], (n, m): (usize, usize), edge: f64, k: usize) -> Vec<usize> {
    let (mut starts, mut prev) = (Vec::new(), 0);
    let mut t = (n / 20).max(3);
    while t <= n {
        let s = svd(&d[..t * m], t, m).s;
        let c = s.iter().filter(|&&x| x > 1.3 * edge * (t as f64 / n as f64).sqrt()).count();
        while prev < c.min(k) {
            starts.push(t - 1);
            prev += 1;
        }
        t += (n / 60).max(1);
    }
    starts
}

/// Noise of the first differences along time: `1.4826 · MAD / sqrt(2)`, the median taken over the whole difference matrix.
fn noise_sigma(d: &[f64], n: usize, m: usize) -> f64 {
    let diff: Vec<f64> = (0..(n - 1) * m).map(|i| d[i + m] - d[i]).collect();
    let med = median(diff.clone());
    let dev: Vec<f64> = diff.iter().map(|x| (x - med).abs()).collect();
    1.4826 * median(dev) / 2f64.sqrt()
}

/// Median (mean of the two middle values for an even count), as `np.median`.
fn median(mut v: Vec<f64>) -> f64 {
    v.sort_by(|a, b| a.total_cmp(b));
    let h = v.len() / 2;
    if v.len() % 2 == 1 {
        v[h]
    } else {
        (v[h - 1] + v[h]) / 2.0
    }
}
