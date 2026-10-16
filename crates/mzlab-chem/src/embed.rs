//! 3D embedding of a `Mol3d`: ETKDG (sci-form, MIT) for several seeds, each result relaxed with MMFF94 (UFF when MMFF94 cannot
//! be built for the molecule, with a note); the lowest-energy conformer comes first. Same seed, same geometry.

use crate::minimize::lbfgs;
use crate::mol3d::Mol3d;
use mzlab_core::error::{Error, Result};
use sci_form::forcefield::mmff94::Mmff94Builder;
use sci_form::graph::{Atom, Bond, BondOrder, BondStereo, Hybridization, Molecule};

/// One conformer: coordinates in the atom order of the `Mol3d`, force-field energy (kcal/mol) and the force field used.
pub struct Conf {
    pub xyz: Vec<[f64; 3]>,
    pub energy: f64,
    pub force_field: &'static str,
    pub note: Option<&'static str>,
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
                out.push(relax(m, &sf, flat));
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

/// MMFF94 minimisation; UFF with a note when the MMFF94 terms cannot be built or the energy is not finite.
fn relax(m: &Mol3d, sf: &Molecule, flat: Vec<f64>) -> Conf {
    let bonds: Vec<(usize, usize, u8)> = m
        .bonds
        .iter()
        .map(|&(a, b, o)| (a, b, if o == 4 { 2 } else { o }))
        .collect();
    let built = std::panic::catch_unwind(|| Mmff94Builder::build(&m.el, &bonds));
    if let Ok(terms) = built {
        let mut x = flat.clone();
        let mut f = |c: &[f64], g: &mut [f64]| {
            let (e, grad) = Mmff94Builder::total_energy(&terms, c);
            g.copy_from_slice(&grad);
            e
        };
        let e = lbfgs(&mut f, &mut x, 500);
        if e.is_finite() {
            return conf(x, e, "mmff94", None);
        }
    }
    let ff = sci_form::forcefield::builder::build_uff_force_field(sf);
    let mut x = flat;
    let mut f = |c: &[f64], g: &mut [f64]| {
        g.iter_mut().for_each(|v| *v = 0.0);
        ff.compute_system_energy_and_gradients(c, g)
    };
    let e = lbfgs(&mut f, &mut x, 500);
    conf(x, e, "uff", Some("mmff94 unavailable, uff used"))
}

fn conf(x: Vec<f64>, energy: f64, force_field: &'static str, note: Option<&'static str>) -> Conf {
    let xyz = x.chunks(3).map(|c| [c[0], c[1], c[2]]).collect();
    Conf {
        xyz,
        energy,
        force_field,
        note,
    }
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
    if mine.contains(&4) {
        Hybridization::SP2
    } else if mine.contains(&3) || doubles > 1 {
        Hybridization::SP
    } else if doubles == 1 {
        Hybridization::SP2
    } else if (m.el[i] == 7 || m.el[i] == 8)
        && incident(m, i).any(|(nb, _)| incident(m, nb).any(|(_, o)| o == 2 || o == 4))
    {
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
