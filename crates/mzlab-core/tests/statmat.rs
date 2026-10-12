//! `statmat`: the SVD reconstructs the matrix with orthonormal factors and descending values; `eigh` solves symmetric matrices;
//! the 2×2 case has numpy's signs (checked against numpy in `stat_parity.rs`).

use mzlab_core::statmat::{eigh, svd};

fn lcg(seed: &mut u64) -> f64 {
    *seed = seed.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
    ((*seed >> 11) as f64 / (1u64 << 53) as f64) - 0.5
}

fn matrix(m: usize, n: usize, seed: u64) -> Vec<f64> {
    let mut s = seed;
    (0..m * n).map(|_| lcg(&mut s) * 10.0).collect()
}

fn check(a: &[f64], m: usize, n: usize) {
    let r = svd(a, m, n);
    assert_eq!(r.k, m.min(n));
    assert!(r.s.windows(2).all(|w| w[0] >= w[1]), "descending");
    for i in 0..m {
        for j in 0..n {
            let v: f64 = (0..r.k).map(|t| r.u[i * r.k + t] * r.s[t] * r.vt[t * n + j]).sum();
            assert!((v - a[i * n + j]).abs() < 1e-10, "reconstruction {m}x{n} at {i},{j}: {v} vs {}", a[i * n + j]);
        }
    }
    for p in 0..r.k {
        for q in 0..r.k {
            let want = if p == q { 1.0 } else { 0.0 };
            let u: f64 = (0..m).map(|i| r.u[i * r.k + p] * r.u[i * r.k + q]).sum();
            let v: f64 = (0..n).map(|j| r.vt[p * n + j] * r.vt[q * n + j]).sum();
            assert!((u - want).abs() < 1e-10 && (v - want).abs() < 1e-10, "orthonormal {m}x{n} {p},{q}: {u} {v}");
        }
    }
}

#[test]
fn svd_reconstructs() {
    for (m, n) in [(1, 1), (3, 3), (8, 3), (3, 8), (6, 40), (40, 6), (1, 5), (5, 1)] {
        check(&matrix(m, n, (m * 100 + n) as u64), m, n);
    }
}

#[test]
fn svd_rank_deficient() {
    // centred samples x features: rank <= samples - 1, the last singular value is 0 and its vectors are still unit
    let (m, n) = (5, 30);
    let mut a = matrix(m, n, 7);
    for j in 0..n {
        let mean: f64 = (0..m).map(|i| a[i * n + j]).sum::<f64>() / m as f64;
        (0..m).for_each(|i| a[i * n + j] -= mean);
    }
    check(&a, m, n);
    assert!(svd(&a, m, n).s[m - 1] < 1e-12);
    check(&vec![0.0; 12], 3, 4);
}

#[test]
fn eigh_symmetric() {
    for n in [1usize, 2, 3, 5, 9] {
        let b = matrix(n, n, 31 + n as u64);
        let a: Vec<f64> = (0..n * n).map(|x| b[x] + b[(x % n) * n + x / n]).collect();
        let (w, v) = eigh(&a, n);
        assert!(w.windows(2).all(|p| p[0] <= p[1]));
        for j in 0..n {
            for i in 0..n {
                let av: f64 = (0..n).map(|k| a[i * n + k] * v[k * n + j]).sum();
                assert!((av - w[j] * v[i * n + j]).abs() < 1e-10, "A v = w v, n={n}");
            }
        }
    }
}
