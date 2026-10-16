//! GFN2-xTB anisotropic electrostatics (atomic dipoles and quadrupoles).
//! Restructured from sci-form 0.15.2 `src/xtb/gfn2.rs` (AES part of `solve_gfn2`); MIT, see LICENSE-sci-form.

use super::basis::par;
use sci_form::xtb::gfn2_params::{MP_DMP3, MP_DMP5, MP_KEXP_MULTI, MP_RMAX, MP_SHIFT};

const QP_SCALE: [f64; 6] = [1.0, 2.0, 1.0, 2.0, 2.0, 1.0];

impl Aes {
    pub fn new(el: &[u8], pos: &[[f64; 3]], cn: &[f64]) -> Aes {
        let n = el.len();
        let mrad: Vec<f64> = (0..n)
            .map(|i| {
                let p = par(el[i]);
                let r0 = mp_rad(el[i], p.mp_rad);
                let t = (-MP_KEXP_MULTI * (cn[i] - p.mp_vcn - MP_SHIFT)).exp();
                r0 + (MP_RMAX - r0) / (1.0 + t)
            })
            .collect();
        let mut a = Aes {
            n,
            sd: vec![[0.0; 3]; n * n],
            dd: vec![[[0.0; 3]; 3]; n * n],
            sq: vec![[0.0; 6]; n * n],
            dk: el.iter().map(|&z| par(z).dkernel).collect(),
            qk: el.iter().map(|&z| par(z).qkernel).collect(),
        };
        for i in 0..n {
            for j in (0..n).filter(|&j| j != i) {
                let v = [
                    pos[i][0] - pos[j][0],
                    pos[i][1] - pos[j][1],
                    pos[i][2] - pos[j][2],
                ];
                let g1 = 1.0 / (v[0] * v[0] + v[1] * v[1] + v[2] * v[2]).sqrt();
                let g3 = g1 * g1 * g1;
                let g5 = g3 * g1 * g1;
                let rr = 0.5 * (mrad[i] + mrad[j]) * g1;
                let f3 = 1.0 / (1.0 + 6.0 * rr.powf(MP_DMP3));
                let f5 = 1.0 / (1.0 + 6.0 * rr.powf(MP_DMP5));
                let k = i * n + j;
                for x in 0..3 {
                    a.sd[k][x] = v[x] * g3 * f3;
                    for y in 0..3 {
                        let d = if x == y { g3 * f5 } else { 0.0 };
                        a.dd[k][x][y] = d - 3.0 * v[x] * v[y] * g5 * f5;
                    }
                }
                let t = g5 * f5;
                a.sq[k] = [
                    v[0] * v[0] * t,
                    2.0 * v[0] * v[1] * t,
                    v[1] * v[1] * t,
                    2.0 * v[0] * v[2] * t,
                    2.0 * v[1] * v[2] * t,
                    v[2] * v[2] * t,
                ];
            }
        }
        a
    }

    /// Potentials for atomic charges `q`, dipoles `dp` and quadrupoles `qp`, kernels included.
    pub fn potential(&self, q: &[f64], dp: &[[f64; 3]], qp: &[[f64; 6]]) -> AesPot {
        let n = self.n;
        let mut p = AesPot {
            vat: vec![0.0; n],
            vdp: vec![[0.0; 3]; n],
            vqp: vec![[0.0; 6]; n],
        };
        for i in 0..n {
            for j in (0..n).filter(|&j| j != i) {
                let (ij, ji) = (i * n + j, j * n + i);
                for x in 0..3 {
                    p.vdp[i][x] += self.sd[ji][x] * q[j];
                    for y in 0..3 {
                        p.vdp[i][x] += self.dd[ij][x][y] * dp[j][y];
                    }
                    p.vat[i] += self.sd[ij][x] * dp[j][x];
                }
                for c in 0..6 {
                    p.vqp[i][c] += self.sq[ij][c] * q[j];
                    p.vat[i] += self.sq[ij][c] * qp[j][c];
                }
            }
            for x in 0..3 {
                p.vdp[i][x] += 2.0 * self.dk[i] * dp[i][x];
            }
            for c in 0..6 {
                p.vqp[i][c] += 2.0 * self.qk[i] * qp[i][c] * QP_SCALE[c];
            }
        }
        p
    }

    /// Electrostatic multipole energy (Hartree), kernels included.
    pub fn energy(&self, q: &[f64], dp: &[[f64; 3]], qp: &[[f64; 6]]) -> f64 {
        let n = self.n;
        let mut e = 0.0;
        for i in 0..n {
            let mut vd = [0.0; 3];
            let mut vq = [0.0; 6];
            for j in (0..n).filter(|&j| j != i) {
                let (ij, ji) = (i * n + j, j * n + i);
                for x in 0..3 {
                    vd[x] += self.sd[ji][x] * q[j];
                    for y in 0..3 {
                        vd[x] += 0.5 * self.dd[ij][x][y] * dp[j][y];
                    }
                }
                for c in 0..6 {
                    vq[c] += self.sq[ij][c] * q[j];
                }
            }
            for x in 0..3 {
                e += dp[i][x] * vd[x] + self.dk[i] * dp[i][x] * dp[i][x];
            }
            for c in 0..6 {
                e += qp[i][c] * vq[c] + self.qk[i] * qp[i][c] * qp[i][c] * QP_SCALE[c];
            }
        }
        e
    }
}

/// Damped interaction tensors between atom pairs (index `i * n + j`) and the on-site kernels.
pub struct Aes {
    n: usize,
    sd: Vec<[f64; 3]>,
    dd: Vec<[[f64; 3]; 3]>,
    sq: Vec<[f64; 6]>,
    dk: Vec<f64>,
    qk: Vec<f64>,
}

/// Potentials of the multipoles: on the atomic charge, on the dipole, on the quadrupole.
pub struct AesPot {
    pub vat: Vec<f64>,
    pub vdp: Vec<[f64; 3]>,
    pub vqp: Vec<[f64; 6]>,
}

/// Multipole damping radius (bohr): sci-form has 3.9 for Br, the GFN2 parameter file of dxtb 0.4.0 has 4.0 (`tests/parametri_gfn2.rs`).
fn mp_rad(z: u8, sci_form: f64) -> f64 {
    if z == 35 {
        4.0
    } else {
        sci_form
    }
}
