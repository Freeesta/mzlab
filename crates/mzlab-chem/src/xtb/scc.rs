//! Self-consistent charges of GFN2-xTB for any total charge: Fermi smearing per spin channel at 300 K (no spin polarisation, as
//! tblite and xtb), Broyden mixing with restarts and stronger damping when the residual grows, at most `MAX_ITER` iterations.
//! Restructured from the SCC loop of sci-form 0.15.2 `src/xtb/gfn2.rs`; MIT, see LICENSE-sci-form.

use super::{Guess, Scf, System};
use nalgebra::DMatrix;
use sci_form::xtb::broyden::BroydenMixer;

pub const MAX_ITER: usize = 250;
const KT: f64 = 300.0 * 3.166_808_578_545_117e-6;
const ECONV: f64 = 1e-6;
const PCONV: f64 = 2e-5;
const DAMP: f64 = 0.4;

/// Output of one Fock build and diagonalisation for the input moments `x`.
struct Step {
    x: Vec<f64>,
    energy: f64,
    emo: Vec<f64>,
}

/// Fermi occupations (0..1) of `n` electrons in one spin channel and the entropy term (Hartree).
fn fermi(emo: &[f64], n: usize) -> (Vec<f64>, f64) {
    let m = emo.len();
    if n == 0 {
        return (vec![0.0; m], 0.0);
    }
    let mut ef = if n < m {
        0.5 * (emo[n - 1] + emo[n])
    } else {
        emo[m - 1] + 1.0
    };
    let mut f = vec![0.0; m];
    for _ in 0..200 {
        let (mut tot, mut der) = (0.0, 0.0);
        for (k, e) in emo.iter().enumerate() {
            let x = (e - ef) / KT;
            f[k] = if x < 50.0 { 1.0 / (x.exp() + 1.0) } else { 0.0 };
            if x < 50.0 {
                der += x.exp() / (KT * (x.exp() + 1.0).powi(2));
            }
            tot += f[k];
        }
        let miss = n as f64 - tot;
        if miss.abs() <= f64::EPSILON.sqrt() {
            break;
        }
        ef += if der > 1e-150 {
            (miss / der).clamp(-1.0, 1.0)
        } else {
            0.0
        };
    }
    let ts: f64 = f
        .iter()
        .filter(|&&v| v > 1e-15 && 1.0 - v > 1e-15)
        .map(|&v| v * v.ln() + (1.0 - v) * (1.0 - v).ln())
        .sum();
    (f, ts * KT)
}

impl System {
    fn split<'a>(&self, x: &'a [f64]) -> (&'a [f64], Vec<[f64; 3]>, Vec<[f64; 6]>) {
        let (ns, n) = (self.basis.shells.len(), self.el.len());
        let dp = (0..n)
            .map(|i| std::array::from_fn(|k| x[ns + 3 * i + k]))
            .collect();
        let qp = (0..n)
            .map(|i| std::array::from_fn(|c| x[ns + 3 * n + 6 * i + c]))
            .collect();
        (&x[..ns], dp, qp)
    }

    fn atom_charges(&self, sh: &[f64]) -> Vec<f64> {
        let mut q = vec![0.0; self.el.len()];
        for (s, v) in self.basis.shells.iter().zip(sh) {
            q[s.atom] += v;
        }
        q
    }

    /// Atomic potential: multipoles, self-consistent D4 and solvation.
    fn fock(&self, x: &[f64]) -> DMatrix<f64> {
        let (sh, dp, qp) = self.split(x);
        let q = self.atom_charges(sh);
        let mut pot = self.aes.potential(&q, &dp, &qp);
        let d4 = self.d4.get_potential(&self.d4.weight_references(&q));
        let solv = self
            .solv
            .as_ref()
            .map(|a| a * nalgebra::DVector::from_column_slice(&q));
        for i in 0..q.len() {
            pot.vat[i] += d4[i] + solv.as_ref().map_or(0.0, |v| v[i]);
        }
        let vsh: Vec<f64> = (0..sh.len())
            .map(|i| {
                (self.gamma.row(i) * nalgebra::DVector::from_column_slice(sh))[0]
                    + self.basis.shells[i].gam3 * sh[i] * sh[i]
            })
            .collect();
        let n = self.basis.ao.len();
        let ao = &self.basis.ao;
        let v: Vec<f64> = (0..n)
            .map(|i| vsh[self.basis.ao_shell[i]] + pot.vat[ao[i].0])
            .collect();
        let mut f = self.h0.clone();
        for i in 0..n {
            for j in 0..n {
                let (a, b) = (ao[i].0, ao[j].0);
                let mut d = 0.5 * self.ints.s[(i, j)] * (v[i] + v[j]);
                for k in 0..3 {
                    d += 0.5
                        * (self.ints.dp[k][(i, j)] * pot.vdp[a][k]
                            + self.ints.dp[k][(j, i)] * pot.vdp[b][k]);
                }
                for c in 0..6 {
                    d += 0.5
                        * (self.ints.qp[c][(i, j)] * pot.vqp[a][c]
                            + self.ints.qp[c][(j, i)] * pot.vqp[b][c]);
                }
                f[(i, j)] -= d;
            }
        }
        f
    }

    /// Orbital energies (ascending) and coefficients of the Fock matrix `f`.
    fn diag(&self, f: &DMatrix<f64>) -> (Vec<f64>, DMatrix<f64>) {
        let e = (&self.x * f * &self.x).symmetric_eigen();
        let mut idx: Vec<usize> = (0..e.eigenvalues.len()).collect();
        idx.sort_by(|&a, &b| e.eigenvalues[a].total_cmp(&e.eigenvalues[b]));
        let c = &self.x * e.eigenvectors.select_columns(&idx);
        (idx.iter().map(|&k| e.eigenvalues[k]).collect(), c)
    }

    /// Density for `nel` electrons (α = ⌈nel/2⌉, β = ⌊nel/2⌋) and the entropy term.
    fn density(&self, emo: &[f64], c: &DMatrix<f64>, nel: usize) -> (DMatrix<f64>, f64) {
        let (fa, ta) = fermi(emo, nel.div_ceil(2));
        let (fb, tb) = fermi(emo, nel / 2);
        let mut cw = c.clone();
        for k in 0..emo.len() {
            let w = (fa[k] + fb[k]).max(0.0).sqrt();
            cw.column_mut(k).scale_mut(w);
        }
        (&cw * cw.transpose(), ta + tb)
    }

    /// Mulliken shell charges and atomic multipoles of the density `p`, flattened like the mixer input.
    fn moments(&self, p: &DMatrix<f64>) -> Vec<f64> {
        let (ns, n, nao) = (self.basis.shells.len(), self.el.len(), self.basis.ao.len());
        let mut x = vec![0.0; ns + 9 * n];
        for (s, sh) in self.basis.shells.iter().enumerate() {
            x[s] = sh.ref_occ;
        }
        for mu in 0..nao {
            let a = self.basis.ao[mu].0;
            for nu in 0..nao {
                let pv = p[(nu, mu)];
                x[self.basis.ao_shell[mu]] -= pv * self.ints.s[(nu, mu)];
                for k in 0..3 {
                    x[ns + 3 * a + k] -= pv * self.ints.dp[k][(mu, nu)];
                }
                for c in 0..6 {
                    x[ns + 3 * n + 6 * a + c] -= pv * self.ints.qp[c][(mu, nu)];
                }
            }
        }
        x
    }

    /// Electronic energy (Hartree) of the output moments `x` with density `p`.
    fn energy(&self, p: &DMatrix<f64>, x: &[f64], ts: f64) -> f64 {
        let (sh, dp, qp) = self.split(x);
        let q = self.atom_charges(sh);
        let shv = nalgebra::DVector::from_column_slice(sh);
        let band = p.component_mul(&self.h0).sum();
        let e2 = 0.5 * (shv.transpose() * &self.gamma * &shv)[0];
        let e3: f64 = self
            .basis
            .shells
            .iter()
            .zip(sh)
            .map(|(s, v)| s.gam3 * v.powi(3) / 3.0)
            .sum();
        let d4 = self.d4.get_energy(&self.d4.weight_references(&q));
        let qv = nalgebra::DVector::from_column_slice(&q);
        let solv = self
            .solv
            .as_ref()
            .map_or(0.0, |a| 0.5 * (qv.transpose() * a * &qv)[0]);
        band + e2 + e3 + self.aes.energy(&q, &dp, &qp) + ts + d4 + solv
    }

    fn step(&self, x: &[f64], nel: usize) -> Step {
        let (emo, c) = self.diag(&self.fock(x));
        let (p, ts) = self.density(&emo, &c, nel);
        let out = self.moments(&p);
        Step {
            energy: self.energy(&p, &out, ts),
            x: out,
            emo,
        }
    }

    /// SCC for `nel` valence electrons from the starting moments `x0`.
    pub(super) fn scc(&self, nel: usize, x0: Vec<f64>) -> Scf {
        let len = x0.len();
        let mut x = x0;
        let mut damp = DAMP;
        let mut mixer = BroydenMixer::new(len, MAX_ITER, damp);
        let (mut best, mut best_x, mut since) = (f64::INFINITY, x.clone(), 0usize);
        let mut prev = f64::NAN;
        let mut last = None;
        for it in 0..MAX_ITER {
            let s = self.step(&x, nel);
            let rms =
                (s.x.iter()
                    .zip(&x)
                    .map(|(o, i)| (o - i).powi(2))
                    .sum::<f64>()
                    / len as f64)
                    .sqrt();
            let done = (s.energy - prev).abs() < ECONV && rms < PCONV;
            prev = s.energy;
            if done {
                return self.result(s, true, it + 1);
            }
            since += 1;
            if rms < best {
                (best, best_x) = (rms, x.clone());
            } else if since > 8 && rms > 10.0 * best && damp > 0.05 {
                damp *= 0.5;
                mixer = BroydenMixer::new(len, MAX_ITER, damp);
                (x, since, prev) = (best_x.clone(), 0, f64::NAN);
                last = Some(s);
                continue;
            }
            mixer.set(&x);
            mixer.diff(&s.x);
            if mixer.step().is_err() {
                mixer = BroydenMixer::new(len, MAX_ITER, damp);
                x = s.x.clone();
            } else {
                mixer.get(&mut x);
            }
            last = Some(s);
        }
        self.result(last.expect("at least one iteration"), false, MAX_ITER)
    }

    fn result(&self, s: Step, converged: bool, iterations: usize) -> Scf {
        let ns = self.basis.shells.len();
        Scf {
            q: self.atom_charges(&s.x[..ns]),
            emo_ev: s
                .emo
                .iter()
                .map(|e| e * super::basis::EV_PER_HARTREE)
                .collect(),
            energy: s.energy,
            converged,
            iterations,
            guess: Guess { x: s.x },
        }
    }
}
