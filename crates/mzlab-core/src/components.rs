//! Number of chemical components of a matrix `D` (scans × channels) and the scan where each starts to appear
//! (`ionfamily.estimate_components`: singular values against the Marchenko-Pastur edge of the noise, and the forward singular values of
//! expanding windows, the evolving factor analysis). Pure Rust on top of `statmat::singular_values`; matrices are row-major `&[f64]`.

use crate::statmat::singular_values;

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
pub fn estimate_components(
    d: &[f64],
    n: usize,
    m: usize,
    max_k: usize,
    min_var: f64,
) -> Components {
    if n < 4 || m < 2 {
        return Components {
            k: 1,
            sv: Vec::new(),
            sigma: None,
            edge: None,
            efa_start: Vec::new(),
        };
    }
    let sv = singular_values(d, n, m);
    let sig = noise_sigma(d, n, m);
    let edge = sig * ((n as f64).sqrt() + (m as f64).sqrt());
    let total = sv.iter().map(|s| s * s).sum::<f64>().max(1e-300);
    let counted = if sig > 0.0 {
        sv.iter()
            .filter(|&&s| s > 1.3 * edge && s * s / total >= min_var)
            .count()
    } else {
        sv.iter().filter(|&&s| s > 0.02 * sv[0]).count()
    };
    let k = counted.clamp(1, max_k.min(sv.len()).max(1));
    let efa_start = evolving_starts(d, (n, m), edge, k, &sv);
    Components {
        k,
        sv: sv.iter().take(max_k + 1).copied().collect(),
        sigma: Some(sig),
        edge: Some(edge),
        efa_start,
    }
}

/// The scans (0-based, the last row of the window) where the 1st, 2nd... component rises above the edge as the window `d[..t]` grows.
/// Stops at the window where the `k`-th one appears (the later windows cannot add a start); `full` are the singular values of `d`, which the
/// window `t = n` shares.
fn evolving_starts(
    d: &[f64],
    (n, m): (usize, usize),
    edge: f64,
    k: usize,
    full: &[f64],
) -> Vec<usize> {
    let (mut starts, mut prev) = (Vec::new(), 0);
    let mut t = (n / 20).max(3);
    while t <= n && prev < k {
        let windowed;
        let s = if t == n {
            full
        } else {
            windowed = singular_values(&d[..t * m], t, m);
            &windowed
        };
        let c = s
            .iter()
            .filter(|&&x| x > 1.3 * edge * (t as f64 / n as f64).sqrt())
            .count();
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
    let mut diff: Vec<f64> = (0..(n - 1) * m).map(|i| d[i + m] - d[i]).collect();
    let med = median(&mut diff);
    // the deviations overwrite the differences: one buffer for both medians
    diff.iter_mut().for_each(|x| *x = (*x - med).abs());
    1.4826 * median(&mut diff) / 2f64.sqrt()
}

/// Median (mean of the two middle values for an even count), as `np.median`; reorders `v`. By selection, the same values as the sorted order.
fn median(v: &mut [f64]) -> f64 {
    let (len, h) = (v.len(), v.len() / 2);
    let (lower, mid, _) = v.select_nth_unstable_by(h, f64::total_cmp);
    let mid = *mid;
    if len % 2 == 1 {
        mid
    } else {
        let lo = lower.iter().copied().fold(f64::NEG_INFINITY, |a, b| {
            if b.total_cmp(&a).is_gt() {
                b
            } else {
                a
            }
        });
        (lo + mid) / 2.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn median_by_selection_is_the_sorted_one() {
        let mut seed = 5u64;
        for len in [1usize, 2, 3, 4, 7, 8, 101, 200] {
            let v: Vec<f64> = (0..len)
                .map(|_| {
                    seed = seed
                        .wrapping_mul(6364136223846793005)
                        .wrapping_add(1442695040888963407);
                    ((seed >> 40) % 9) as f64 - 4.0 // many ties, both zeros below
                })
                .map(|x| if x == 0.0 { -0.0 } else { x })
                .collect();
            let mut sorted = v.clone();
            crate::sortc::by(&mut sorted, |a, b| a.total_cmp(b));
            let h = len / 2;
            let want = if len % 2 == 1 {
                sorted[h]
            } else {
                (sorted[h - 1] + sorted[h]) / 2.0
            };
            assert_eq!(
                median(&mut v.clone()).to_bits(),
                want.to_bits(),
                "len {len}"
            );
        }
    }

    #[test]
    fn the_early_stop_gives_the_starts_of_the_full_window_loop() {
        let mut seed = 11u64;
        for (n, m, bumps) in [
            (120usize, 40usize, 2usize),
            (90, 25, 1),
            (200, 30, 3),
            (30, 50, 2),
        ] {
            let mut rnd = || {
                seed = seed
                    .wrapping_mul(6364136223846793005)
                    .wrapping_add(1442695040888963407);
                (seed >> 11) as f64 / (1u64 << 53) as f64
            };
            let spectra: Vec<Vec<f64>> = (0..bumps)
                .map(|_| (0..m).map(|_| rnd() * 100.0).collect())
                .collect();
            let d: Vec<f64> = (0..n * m)
                .map(|x| {
                    let (i, j) = ((x / m) as f64, x % m);
                    let sig: f64 = (0..bumps)
                        .map(|b| {
                            (-((i - n as f64 * (0.25 + 0.25 * b as f64)) / (n as f64 * 0.05))
                                .powi(2))
                            .exp()
                                * spectra[b][j]
                        })
                        .sum();
                    sig + rnd()
                })
                .collect();
            let r = estimate_components(&d, n, m, 5, 0.002);
            // the loop as it ran without the early stop and without the shared last window
            let (mut want, mut prev, mut t) = (Vec::new(), 0, (n / 20).max(3));
            while t <= n {
                let s = crate::statmat::svd(&d[..t * m], t, m).s;
                let c = s
                    .iter()
                    .filter(|&&x| x > 1.3 * r.edge.unwrap() * (t as f64 / n as f64).sqrt())
                    .count();
                while prev < c.min(r.k) {
                    want.push(t - 1);
                    prev += 1;
                }
                t += (n / 60).max(1);
            }
            assert_eq!(r.efa_start, want, "{n}x{m}");
        }
    }
}
