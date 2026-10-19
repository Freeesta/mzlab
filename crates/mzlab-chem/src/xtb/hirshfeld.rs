//! Hirshfeld populations of GFN2-xTB densities by numerical integration (Hirshfeld 1977, doi:10.1007/BF00549096): the valence
//! density Σ Pμν φμ φν is built from the Slater orbitals of the basis, the promolecule from the reference occupations, and the
//! integral runs on Becke-partitioned atomic grids (Becke 1988, doi:10.1063/1.454033). Slater functions stand in for the STO-nG
//! expansions, so the integrated electron count is rescaled to the exact one.

use super::basis::dist;
use super::sasa::sphere;
use super::System;
use nalgebra::DMatrix;
use sci_form::xtb::gfn2_params::get_gfn2_params;
use std::f64::consts::PI;

/// Radial and angular points of each atomic grid.
const RADIAL: usize = 30;
const ANGULAR: usize = 110;
/// Slater functions below this exponent·distance product are taken as zero.
const CUT: f64 = 16.0;
/// Orbital values below this are dropped from the density sums.
const TINY: f64 = 1e-5;

impl System {
    /// Electrons of each atom (Hirshfeld partition) for every density matrix in `ps`, summing to the exact electron count.
    pub fn hirshfeld_populations(&self, ps: &[&DMatrix<f64>], nel: &[usize]) -> Vec<Vec<f64>> {
        let n = self.el.len();
        let b = &self.basis;
        let nao = b.ao.len();
        // Every shell in basis order: (atom, norm², exponent, power, reference occupation); the promolecule uses those with occupation.
        let mut pros = Vec::new();
        for (a, &z) in self.el.iter().enumerate() {
            let p = get_gfn2_params(z).expect("supported element");
            for s in 0..p.nshell {
                let (nm, ze, pw) = sto(p.pqn[s], p.slater[s]);
                pros.push((a, nm, ze, pw, p.ref_occ[s]));
            }
        }
        let stos: Vec<_> = b
            .ao_shell
            .iter()
            .map(|&s| (pros[s].1, pros[s].2, pros[s].3))
            .collect();
        let mut pop = vec![vec![0.0; n]; ps.len()];
        let (mut dst, mut pro, mut cell) = (vec![0.0; n], vec![0.0; n], vec![0.0; n]);
        let (mut phi, mut idx) = (vec![0.0; nao], Vec::new());
        for a in 0..n {
            let rm = if self.el[a] == 1 { 1.0 } else { 1.5 };
            for i in 1..=RADIAL {
                let th = i as f64 * PI / (RADIAL + 1) as f64;
                let x = th.cos();
                let r = rm * (1.0 + x) / (1.0 - x);
                let jac = th.sin() * 2.0 * rm / (1.0 - x).powi(2) * r * r;
                let w = jac * 4.0 * PI * PI / ((RADIAL + 1) * ANGULAR) as f64;
                for d in sphere(ANGULAR) {
                    let o = self.pos[a];
                    let pt = [o[0] + r * d[0], o[1] + r * d[1], o[2] + r * d[2]];
                    pro.fill(0.0);
                    for (d, p) in dst.iter_mut().zip(&self.pos) {
                        *d = dist(&pt, p);
                    }
                    for &(c, nm, z, pw, occ) in pros.iter().filter(|p| p.4 > 0.0) {
                        let n2 = nm * nm;
                        if z * dst[c] <= CUT {
                            pro[c] += occ * n2 * dst[c].powi(2 * pw) * (-2.0 * z * dst[c]).exp()
                                / (4.0 * PI);
                        }
                    }
                    let tot: f64 = pro.iter().sum();
                    if tot < 1e-14 {
                        continue;
                    }
                    self.cells(&dst, &mut cell);
                    let wa = w * cell[a] / cell.iter().sum::<f64>();
                    self.orbitals(&pt, &dst, &stos, &mut phi, &mut idx);
                    for (k, p) in ps.iter().enumerate() {
                        let rho = density(p.as_slice(), &phi, &idx);
                        for (o, w) in pop[k].iter_mut().zip(&pro) {
                            *o += wa * w / tot * rho;
                        }
                    }
                }
            }
        }
        for (k, v) in pop.iter_mut().enumerate() {
            let s: f64 = v.iter().sum();
            v.iter_mut().for_each(|x| *x *= nel[k] as f64 / s);
        }
        pop
    }
}

impl System {
    /// Becke cell weights of every atom at a point with distances `dst` to the atoms.
    fn cells(&self, dst: &[f64], cell: &mut [f64]) {
        let n = dst.len();
        cell.fill(1.0);
        for c in 0..n {
            for e in (c + 1)..n {
                let rab = dist(&self.pos[c], &self.pos[e]).max(1e-9);
                let s = becke((dst[c] - dst[e]) / rab);
                cell[c] *= s;
                cell[e] *= 1.0 - s;
            }
        }
    }

    /// Orbital values `phi` at `pt` and the indices of those above `TINY`.
    fn orbitals(
        &self,
        pt: &[f64; 3],
        dst: &[f64],
        stos: &[(f64, f64, i32)],
        phi: &mut [f64],
        idx: &mut Vec<usize>,
    ) {
        idx.clear();
        for (m, &(c, _, l, mm)) in self.basis.ao.iter().enumerate() {
            let (nm, z, pw) = stos[m];
            let rc = dst[c];
            phi[m] = 0.0;
            if z * rc <= CUT && (rc > 1e-12 || l == 0) {
                let (o, q) = (self.pos[c], rc.max(1e-12));
                let u = [(pt[0] - o[0]) / q, (pt[1] - o[1]) / q, (pt[2] - o[2]) / q];
                phi[m] = nm * rc.powi(pw) * (-z * rc).exp() * harmonic(l, mm, &u);
                if phi[m].abs() > TINY {
                    idx.push(m);
                }
            }
        }
    }
}

/// Normalisation, exponent and n − 1 of a radial Slater function.
fn sto(n: u8, zeta: f64) -> (f64, f64, i32) {
    let k = 2 * n as i32;
    let mut f = 1.0;
    for i in 1..=k {
        f *= i as f64;
    }
    (((2.0 * zeta).powi(k + 1) / f).sqrt(), zeta, n as i32 - 1)
}

/// Real spherical harmonic (m in the order of the basis: p = x, y, z; d = xy, xz, yz, x²−y², z²) at the unit vector `u`.
fn harmonic(l: u8, m: u8, u: &[f64; 3]) -> f64 {
    let [x, y, z] = *u;
    let d = (3.75 / PI).sqrt();
    match (l, m) {
        (0, _) => (0.25 / PI).sqrt(),
        (1, k) => (0.75 / PI).sqrt() * u[k as usize],
        (_, 0) => d * x * y,
        (_, 1) => d * x * z,
        (_, 2) => d * y * z,
        (_, 3) => 0.5 * d * (x * x - y * y),
        _ => 0.25 * (5.0 / PI).sqrt() * (3.0 * z * z - 1.0),
    }
}

/// Becke step function s(μ) after three iterations of p(μ) = 1.5μ − 0.5μ³.
fn becke(mu: f64) -> f64 {
    let f = (0..3).fold(mu, |f, _| 1.5 * f - 0.5 * f * f * f);
    0.5 * (1.0 - f)
}

/// Electron density Σ Pμν φμ φν over the orbitals `idx` (column-major symmetric `p`, `phi` the orbital values).
fn density(p: &[f64], phi: &[f64], idx: &[usize]) -> f64 {
    let nao = phi.len();
    let mut rho = 0.0;
    for (j, &m) in idx.iter().enumerate() {
        let mut s = 0.5 * p[m * nao + m] * phi[m];
        for &q in &idx[..j] {
            s += p[m * nao + q] * phi[q];
        }
        rho += 2.0 * phi[m] * s;
    }
    rho
}
