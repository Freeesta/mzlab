//! 3D embedding of a `Mol3d`: ETKDG geometry (sci-form, MIT) for several seeds, with a light regularisation of the bond lengths (no force field). The conformers are
//! ranked by the UFF single-point energy of the ETKDG geometry (lowest first). Same seed, same geometry.

use crate::lengths::ideal;
use crate::minimize::lbfgs;
use crate::mol3d::Mol3d;
use mzlab_core::error::{Error, Result};
use sci_form::graph::{Atom, Bond, BondOrder, BondStereo, Hybridization, Molecule};

/// Conformers computed by default; the number is a parameter of `embed`.
pub const DEFAULT_CONFORMERS: usize = 4;

/// One conformer: coordinates in the atom order of the `Mol3d` and the UFF single-point energy (kcal/mol) used to rank it.
pub struct Conf {
    pub xyz: Vec<[f64; 3]>,
    pub energy: f64,
}

/// Up to `n` conformers (seeds `seed`, `seed+997`, ...), lowest energy first. Fails with `error.chem.embed` when none comes out.
pub fn embed(m: &Mol3d, seed: u64, n: usize) -> Result<Vec<Conf>> {
    let sf = to_sf(m);
    let mut out = Vec::new();
    let mut last = String::from("no attempt");
    for k in 0..n.max(1) as u64 {
        match sci_form::conformer::generate_3d_conformer(&sf, seed.wrapping_add(k * 997)) {
            Ok(c) => {
                let flat: Vec<f64> = (0..m.len())
                    .flat_map(|i| (0..3).map(move |j| (i, j)))
                    .map(|(i, j)| c[(i, j)] as f64)
                    .collect();
                out.push(conf(&sf, regularize(m, flat)));
            }
            Err(e) => last = e,
        }
    }
    if out.is_empty() {
        return Err(Error::new("error.chem.embed").with("reason", &last));
    }
    out.sort_by(|a, b| a.energy.total_cmp(&b.energy));
    Ok(out)
}

fn conf(sf: &Molecule, flat: Vec<f64>) -> Conf {
    let energy = rank_energy(sf, &flat);
    let xyz = flat.chunks(3).map(|c| [c[0], c[1], c[2]]).collect();
    Conf { xyz, energy }
}

/// Light regularisation: every bond is pulled to its ideal length while each atom stays near its ETKDG position (a weak spring).
/// It is not a force field: no angles, torsions or non-bonded terms.
fn regularize(m: &Mol3d, start: Vec<f64>) -> Vec<f64> {
    let ideal_len: Vec<f64> = m.bonds.iter().map(|&(a, b, o)| ideal(m, a, b, o)).collect();
    let mut x = start.clone();
    let mut f = |c: &[f64], g: &mut [f64]| {
        g.iter_mut().for_each(|v| *v = 0.0);
        let mut e = 0.0;
        for (&(a, b, _), &r0) in m.bonds.iter().zip(&ideal_len) {
            let d: [f64; 3] = std::array::from_fn(|k| c[3 * a + k] - c[3 * b + k]);
            let r = (d[0] * d[0] + d[1] * d[1] + d[2] * d[2]).sqrt().max(1e-9);
            e += BOND_K * (r - r0).powi(2);
            for k in 0..3 {
                let t = 2.0 * BOND_K * (r - r0) * d[k] / r;
                g[3 * a + k] += t;
                g[3 * b + k] -= t;
            }
        }
        for (i, (v, s)) in c.iter().zip(&start).enumerate() {
            e += ANCHOR_K * (v - s).powi(2);
            g[i] += 2.0 * ANCHOR_K * (v - s);
        }
        e
    };
    lbfgs(&mut f, &mut x, 300);
    x
}

/// Spring constants of the regularisation (arbitrary units): the bond term dominates, the anchor keeps the ETKDG shape.
const BOND_K: f64 = 100.0;
const ANCHOR_K: f64 = 1.0;

/// UFF single-point energy of a geometry, used only to rank the conformers (the geometry itself is not refined).
fn rank_energy(sf: &Molecule, flat: &[f64]) -> f64 {
    let ff = sci_form::forcefield::builder::build_uff_force_field(sf);
    let mut grad = vec![0.0; flat.len()];
    ff.compute_system_energy_and_gradients(flat, &mut grad)
}

/// The sci-form molecule of the same atoms and bonds (explicit hydrogens as atoms, aromatic bonds as such).
fn to_sf(m: &Mol3d) -> Molecule {
    let mut sf = Molecule::new("mzlab");
    let ids: Vec<_> = (0..m.len())
        .map(|i| {
            let mut a = Atom::new(m.el[i], 0.0, 0.0, 0.0);
            a.formal_charge = m.charge[i];
            a.charge = m.charge[i] as f32;
            a.hybridization = hybridization(m, i);
            sf.add_atom(a)
        })
        .collect();
    for &(a, b, o) in &m.bonds {
        sf.add_bond(
            ids[a],
            ids[b],
            Bond {
                order: order(o),
                stereo: BondStereo::None,
            },
        );
    }
    sf
}

fn order(o: u8) -> BondOrder {
    match o {
        1 => BondOrder::Single,
        2 => BondOrder::Double,
        3 => BondOrder::Triple,
        4 => BondOrder::Aromatic,
        _ => BondOrder::Unknown,
    }
}

/// The hybridization rules of the sci-form SMILES reader: aromatic or one double bond sp2, triple or two doubles sp,
/// an N or O next to a double or aromatic bond sp2, otherwise sp3 (hydrogens stay unset).
fn hybridization(m: &Mol3d, i: usize) -> Hybridization {
    if m.el[i] == 1 {
        return Hybridization::Unknown;
    }
    let mine: Vec<u8> = incident(m, i).map(|(_, o)| o).collect();
    let doubles = mine.iter().filter(|&&o| o == 2).count();
    let conjugated = (m.el[i] == 7 || m.el[i] == 8)
        && incident(m, i).any(|(nb, _)| incident(m, nb).any(|(_, o)| o == 2 || o == 4));
    if mine.contains(&4) {
        Hybridization::SP2
    } else if mine.contains(&3) || doubles > 1 {
        Hybridization::SP
    } else if doubles == 1 || conjugated {
        Hybridization::SP2
    } else {
        Hybridization::SP3
    }
}

/// (neighbour, bond order) of atom `i`.
fn incident(m: &Mol3d, i: usize) -> impl Iterator<Item = (usize, u8)> + '_ {
    m.bonds.iter().filter_map(move |&(a, b, o)| {
        if a == i {
            Some((b, o))
        } else if b == i {
            Some((a, o))
        } else {
            None
        }
    })
}
