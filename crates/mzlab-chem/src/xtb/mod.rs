//! GFN2-xTB (Bannwarth, Ehlert, Grimme 2019, doi:10.1021/acs.jctc.8b01176) for any total charge, in the gas phase or in ALPB water.
//! Parameters: `sci_form::xtb::gfn2_params` (sci-form 0.15.2, MIT), checked number by number against the GFN2 parameter file of
//! dxtb 0.4.0 (`param/gfn2/gfn2-xtb.toml`, Apache-2.0) by `tests/parametri_gfn2.rs`. The geometry-dependent matrices are built
//! once per `System` and shared by the neutral molecule, the cation and the anion.

mod aes;
pub mod alpb;
mod basis;
mod hirshfeld;
mod sasa;
mod scc;
mod water;

use mzlab_core::error::{Error, Result};
use nalgebra::DMatrix;
use sci_form::xtb::d4::D4Model;
use sci_form::xtb::gfn2_params;

pub use basis::{ANGSTROM_TO_BOHR, EV_PER_HARTREE};
pub use scc::MAX_ITER;

/// Valence electrons of the neutral atom of atomic number `z`.
pub fn valence(z: u8) -> f64 {
    basis::par(z).ref_occ.iter().sum()
}

/// Geometry-dependent part of a GFN2-xTB calculation.
pub struct System {
    el: Vec<u8>,
    /// Positions in bohr.
    pos: Vec<[f64; 3]>,
    basis: basis::Basis,
    ints: basis::Ints,
    h0: DMatrix<f64>,
    gamma: DMatrix<f64>,
    /// Löwdin S^(-1/2).
    x: DMatrix<f64>,
    aes: aes::Aes,
    d4: D4Model,
    solv: Option<DMatrix<f64>>,
}

/// Converged (or last) shell charges and atomic multipoles, to start another charge state from.
#[derive(Clone)]
pub struct Guess {
    x: Vec<f64>,
}

impl System {
    /// Elements (atomic numbers), positions in Å, optional solvent.
    pub fn new(el: &[u8], xyz: &[[f64; 3]], solvent: Option<&alpb::Solvent>) -> Result<System> {
        if let Some(&z) = el.iter().find(|&&z| !gfn2_params::is_gfn2_supported(z)) {
            return Err(Error::new("error.chem.element").with("element", z.to_string()));
        }
        let pos: Vec<[f64; 3]> = xyz
            .iter()
            .map(|p| p.map(|v| v * ANGSTROM_TO_BOHR))
            .collect();
        let b = basis::basis(el);
        let ints = basis::ints(el, &pos, &b);
        let cn = basis::coordination(el, &pos);
        let h0 = basis::h0(el, &pos, &b, &ints.s, &cn);
        let e = ints.s.clone().symmetric_eigen();
        let inv = e
            .eigenvalues
            .map(|v| if v > 1e-8 { 1.0 / v.sqrt() } else { 0.0 });
        let x = &e.eigenvectors * DMatrix::from_diagonal(&inv) * e.eigenvectors.transpose();
        Ok(System {
            gamma: basis::gamma(&pos, &b),
            aes: aes::Aes::new(el, &pos, &cn),
            d4: D4Model::new(el, &pos),
            solv: solvent.map(|s| alpb::matrix(el, &pos, s)),
            el: el.to_vec(),
            pos,
            basis: b,
            ints,
            h0,
            x,
        })
    }

    /// Valence electrons of the neutral molecule.
    pub fn electrons(&self) -> usize {
        gfn2_params::gfn2_electron_count(&self.el)
    }

    /// SCC for total charge `charge`, from `guess` (for example the neutral result) or from zero charges.
    pub fn solve(&self, charge: i32, guess: Option<&Guess>) -> Result<Scf> {
        let nel = self.electrons() as i64 - charge as i64;
        if nel < 0 || nel as usize > 2 * self.basis.ao.len() {
            return Err(Error::new("error.chem.charge").with("charge", charge.to_string()));
        }
        let len = self.basis.shells.len() + 9 * self.el.len();
        let x0 = guess.map_or_else(|| vec![0.0; len], |g| g.x.clone());
        Ok(self.scc(nel as usize, x0))
    }
}

impl System {
    /// Share of the HOMO and of the LUMO on each atom (Mulliken partition of |ψ|², each list sums to 1) for `nel` electrons.
    pub fn frontier_shares(&self, scf: &Scf, nel: usize) -> (Vec<f64>, Vec<f64>) {
        let sc = &self.ints.s * &scf.c;
        let share = |k: usize| {
            let mut v = vec![0.0; self.el.len()];
            for (m, ao) in self.basis.ao.iter().enumerate() {
                v[ao.0] += scf.c[(m, k)] * sc[(m, k)];
            }
            v
        };
        let h = (nel / 2).max(1) - 1;
        (share(h), share((nel / 2).min(scf.emo_ev.len() - 1)))
    }
}

/// Result of one SCC.
pub struct Scf {
    /// Mulliken atomic charges.
    pub q: Vec<f64>,
    /// Orbital energies (eV), ascending.
    pub emo_ev: Vec<f64>,
    /// Electronic energy (Hartree): band, second and third order, multipoles, self-consistent D4, solvation, entropy.
    pub energy: f64,
    pub converged: bool,
    pub iterations: usize,
    pub guess: Guess,
    /// Density matrix (atomic-orbital basis) and molecular-orbital coefficients (columns, ascending energy).
    pub p: DMatrix<f64>,
    pub c: DMatrix<f64>,
}

impl Scf {
    /// HOMO and LUMO (eV) for `nel` electrons (closed shell: orbitals nel/2 − 1 and nel/2).
    pub fn frontier(&self, nel: usize) -> (f64, f64) {
        let h = (nel / 2).max(1) - 1;
        let l = (nel / 2).min(self.emo_ev.len() - 1);
        (self.emo_ev[h], self.emo_ev[l])
    }
}
