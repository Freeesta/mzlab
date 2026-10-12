//! Matrix exponential of the 4 x 4 rate matrix of the chain A -> I -> B -> sink: Pade (6, 6) with scaling and squaring.
//! Robust for repeated rate constants (the matrix is then defective), like `kinetics.expm_batch`.

type M = [[f64; 4]; 4];
const PADE: [f64; 7] = [
    1.0,
    1.0 / 2.0,
    5.0 / 44.0,
    1.0 / 66.0,
    1.0 / 792.0,
    1.0 / 15840.0,
    1.0 / 665280.0,
];

fn mul(a: &M, b: &M) -> M {
    let mut c = [[0.0; 4]; 4];
    for i in 0..4 {
        for j in 0..4 {
            let mut s = 0.0;
            for k in 0..4 {
                s += a[i][k] * b[k][j];
            }
            c[i][j] = s;
        }
    }
    c
}

/// Solves `d * x = n` by Gaussian elimination with partial pivoting.
fn solve(mut d: M, mut n: M) -> M {
    for c in 0..4 {
        let p = (c..4).fold(c, |p, r| if d[r][c].abs() > d[p][c].abs() { r } else { p });
        d.swap(c, p);
        n.swap(c, p);
        for r in c + 1..4 {
            let f = d[r][c] / d[c][c];
            for k in c..4 {
                d[r][k] -= f * d[c][k];
            }
            for k in 0..4 {
                n[r][k] -= f * n[c][k];
            }
        }
    }
    for k in 0..4 {
        for r in (0..4).rev() {
            let mut s = n[r][k];
            for j in r + 1..4 {
                s -= d[r][j] * n[j][k];
            }
            n[r][k] = s / d[r][r];
        }
    }
    n
}

pub fn expm(a: &M) -> M {
    let nrm = (0..4)
        .map(|j| (0..4).map(|i| a[i][j].abs()).sum::<f64>())
        .fold(0.0, f64::max);
    let s = (libm::ceil(libm::log2(nrm.max(1e-300))) + 1.0).max(0.0) as i32;
    let scale = libm::pow(2.0, -(s as f64));
    let mut x = *a;
    x.iter_mut().flatten().for_each(|v| *v *= scale);
    let id: M = std::array::from_fn(|i| std::array::from_fn(|j| (i == j) as u8 as f64));
    let (mut num, mut den, mut p) = (id, id, id);
    for k in 1..7 {
        p = mul(&p, &x);
        let sign = if k % 2 == 0 { 1.0 } else { -1.0 };
        for i in 0..4 {
            for j in 0..4 {
                num[i][j] += PADE[k] * p[i][j];
                den[i][j] += sign * PADE[k] * p[i][j];
            }
        }
    }
    let mut e = solve(den, num);
    for _ in 0..s {
        e = mul(&e, &e);
    }
    e
}

/// Concentration of the third species of the chain A -> I -> B -> sink at time `t` (the parent starts at 1) with the rates `k`.
pub fn chain3(k: [f64; 3], t: f64) -> f64 {
    let mut a = [[0.0; 4]; 4];
    for i in 0..3 {
        a[i][i] = -k[i] * t;
        a[i + 1][i] = k[i] * t;
    }
    expm(&a)[2][0]
}
