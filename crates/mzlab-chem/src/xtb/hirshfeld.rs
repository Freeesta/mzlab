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
/// Orbital values below this are dropped from the density sums, grid weights below `TINY_W` skip the point.
const TINY: f64 = 1e-5;
const TINY_W: f64 = 1e-10;

/// Normalisation, exponent and n − 1 of a radial Slater function.
fn sto(n: u8, zeta: f64) -> (f64, f64, i32) {
    let k = 2 * n as i32;
    let f: f64 = (1..=k).map(|i| i as f64).product();
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

impl System {
    /// Electrons of each atom (Hirshfeld partition) for every density matrix in `ps`, summing to the exact electron count.
    pub fn hirshfeld_populations(&self, ps: &[&DMatrix<f64>], nel: &[usize]) -> Vec<Vec<f64>> {
        let n = self.el.len();
        let b = &self.basis;
        let nao = b.ao.len();
        let par = |a: usize| get_gfn2_params(self.el[a]).expect("supported element");
        let stos: Vec<_> =
            b.ao.iter()
                .map(|&(a, sh, ..)| sto(par(a).pqn[sh], par(a).slater[sh]))
                .collect();
        // Promolecule: (atom, norm², exponent, power, occupation) of each occupied shell.
        let mut pros = Vec::new();
        for a in 0..n {
            let p = par(a);
            for s in (0..p.nshell).filter(|&s| p.ref_occ[s] > 0.0) {
                let (nm, z, pw) = sto(p.pqn[s], p.slater[s]);
                pros.push((a, nm * nm, z, pw, p.ref_occ[s]));
            }
        }
        let rab = |c: usize, e: usize| dist(&self.pos[c], &self.pos[e]).max(1e-9);
        let mut pop = vec![vec![0.0; n]; ps.len()];
        let (mut dst, mut pro, mut cell) = (vec![0.0; n], vec![0.0; n], vec![0.0; n]);
        let (mut phi, mut idx) = (vec![0.0; nao], Vec::new());
        for a in 0..n {
            let rm = if self.el[a] == 1 { 1.0 } else { 1.5 };
            for i in 1..=RADIAL {
                let th = i as f64 * PI / (RADIAL + 1) as f64;
                let x = th.cos();
                let r = rm * (1.0 + x) / (1.0 - x);
                let w = PI / (RADIAL + 1) as f64 * th.sin() * 2.0 * rm / (1.0 - x).powi(2)
                    * r
                    * r
                    * 4.0
                    * PI
                    / ANGULAR as f64;
                for d in sphere(ANGULAR) {
                    let pt: [f64; 3] = std::array::from_fn(|k| self.pos[a][k] + r * d[k]);
                    pro.fill(0.0);
                    for (d, p) in dst.iter_mut().zip(&self.pos) {
                        *d = dist(&pt, p);
                    }
                    for &(c, n2, z, pw, occ) in &pros {
                        if z * dst[c] <= CUT {
                            pro[c] += occ * n2 * dst[c].powi(2 * pw) * (-2.0 * z * dst[c]).exp()
                                / (4.0 * PI);
                        }
                    }
                    let tot: f64 = pro.iter().sum();
                    if tot < 1e-14 {
                        continue;
                    }
                    cell.fill(1.0);
                    for c in 0..n {
                        for e in (c + 1)..n {
                            let s = becke((dst[c] - dst[e]) / rab(c, e));
                            cell[c] *= s;
                            cell[e] *= 1.0 - s;
                        }
                    }
                    let wa = w * cell[a] / cell.iter().sum::<f64>();
                    if wa < TINY_W {
                        continue;
                    }
                    idx.clear();
                    for (m, &(c, _, l, mm)) in b.ao.iter().enumerate() {
                        let (nm, z, pw) = stos[m];
                        let rc = dst[c];
                        phi[m] = 0.0;
                        if z * rc <= CUT && (rc > 1e-12 || l == 0) {
                            let u: [f64; 3] =
                                std::array::from_fn(|k| (pt[k] - self.pos[c][k]) / rc.max(1e-12));
                            phi[m] = nm * rc.powi(pw) * (-z * rc).exp() * harmonic(l, mm, &u);
                            if phi[m].abs() > TINY {
                                idx.push(m);
                            }
                        }
                    }
                    for (k, p) in ps.iter().enumerate() {
                        let p = p.as_slice();
                        let mut rho = 0.0;
                        for (j, &m) in idx.iter().enumerate() {
                            let mut s = 0.5 * p[m * nao + m] * phi[m];
                            for &q in &idx[..j] {
                                s += p[m * nao + q] * phi[q];
                            }
                            rho += 2.0 * phi[m] * s;
                        }
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
