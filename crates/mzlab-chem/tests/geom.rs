//! Geometry of `embed` for the 20 public molecules of `tests/data/cc_ref.json` (RDKit ETKDGv3 + MMFF94, seed 42).

use mzlab_chem::{embed, Mol3d, DEFAULT_CONFORMERS};
use mzlab_feat::chem::grafo::Mol;
use nalgebra::{Matrix3, Vector3};
use std::time::Instant;

const SEED: u64 = 42;
const CONFORMERS: usize = DEFAULT_CONFORMERS;

fn reference() -> serde_json::Value {
    let text = std::fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/data/cc_ref.json"
    ))
    .unwrap();
    serde_json::from_str(&text).unwrap()
}

fn mol3d(smiles: &str) -> Mol3d {
    Mol3d::from_mol(&Mol::parse(smiles).unwrap()).unwrap()
}

fn ref_xyz(rec: &serde_json::Value) -> Vec<[f64; 3]> {
    rec["xyz"]
        .as_array()
        .unwrap()
        .iter()
        .map(|p| {
            let v: Vec<f64> = p
                .as_array()
                .unwrap()
                .iter()
                .map(|x| x.as_f64().unwrap())
                .collect();
            [v[0], v[1], v[2]]
        })
        .collect()
}

fn dist(a: &[f64; 3], b: &[f64; 3]) -> f64 {
    ((a[0] - b[0]).powi(2) + (a[1] - b[1]).powi(2) + (a[2] - b[2]).powi(2)).sqrt()
}

fn centred(p: &[[f64; 3]]) -> Vec<Vector3<f64>> {
    let c = p
        .iter()
        .fold(Vector3::zeros(), |s, q| s + Vector3::new(q[0], q[1], q[2]));
    let c = c / p.len() as f64;
    p.iter()
        .map(|q| Vector3::new(q[0], q[1], q[2]) - c)
        .collect()
}

/// RMSD after the best rotation (Kabsch); with `mirror` the second set is reflected first.
fn kabsch(a: &[Vector3<f64>], b: &[Vector3<f64>], mirror: bool) -> f64 {
    let b: Vec<Vector3<f64>> = b
        .iter()
        .map(|v| {
            if mirror {
                Vector3::new(-v.x, v.y, v.z)
            } else {
                *v
            }
        })
        .collect();
    let mut h = Matrix3::zeros();
    for (p, q) in a.iter().zip(&b) {
        h += q * p.transpose();
    }
    let svd = h.svd(true, true);
    let (u, vt) = (svd.u.unwrap(), svd.v_t.unwrap());
    let d = (vt.transpose() * u.transpose()).determinant().signum();
    let r = vt.transpose() * Matrix3::from_diagonal(&Vector3::new(1.0, 1.0, d)) * u.transpose();
    let sum: f64 = a
        .iter()
        .zip(&b)
        .map(|(p, q)| (p - r * q).norm_squared())
        .sum();
    (sum / a.len() as f64).sqrt()
}

/// Heavy-atom RMSD to the reference; the mirror image counts too (the SMILES carry no stereocentres, so RDKit may pick either).
fn heavy_rmsd(m: &Mol3d, xyz: &[[f64; 3]], rec: &serde_json::Value) -> f64 {
    let r = ref_xyz(rec);
    let idx: Vec<usize> = (0..m.len())
        .filter(|&i| m.heavy_index[i].is_some())
        .collect();
    let mine: Vec<[f64; 3]> = idx.iter().map(|&i| xyz[i]).collect();
    let theirs: Vec<[f64; 3]> = idx.iter().map(|&i| r[i]).collect();
    let (a, b) = (centred(&mine), centred(&theirs));
    kabsch(&a, &b, false).min(kabsch(&a, &b, true))
}

fn median(mut v: Vec<f64>) -> f64 {
    v.sort_by(|a, b| a.total_cmp(b));
    v[v.len() / 2]
}

#[test]
fn embedding_succeeds_for_the_20_molecules() {
    for (name, rec) in reference()["molecules"].as_object().unwrap() {
        let m = mol3d(rec["smiles"].as_str().unwrap());
        let confs = embed(&m, SEED, CONFORMERS).unwrap_or_else(|e| panic!("{name}: {e:?}"));
        assert!(!confs.is_empty(), "{name}");
        assert_eq!(confs[0].xyz.len(), m.len(), "{name}");
        assert!(
            confs.windows(2).all(|w| w[0].energy <= w[1].energy),
            "{name}: sorted"
        );
        assert!(
            confs[0].xyz.iter().flatten().all(|v| v.is_finite()),
            "{name}"
        );
    }
}

#[test]
fn no_non_bonded_pair_closer_than_0_9_angstrom() {
    for (name, rec) in reference()["molecules"].as_object().unwrap() {
        let m = mol3d(rec["smiles"].as_str().unwrap());
        let c = &embed(&m, SEED, CONFORMERS).unwrap()[0];
        for i in 0..m.len() {
            for j in i + 1..m.len() {
                let bonded = m
                    .bonds
                    .iter()
                    .any(|&(a, b, _)| (a, b) == (i, j) || (b, a) == (i, j));
                if !bonded {
                    assert!(
                        dist(&c.xyz[i], &c.xyz[j]) >= 0.9,
                        "{name}: atoms {i}-{j}: {:.3}",
                        dist(&c.xyz[i], &c.xyz[j])
                    );
                }
            }
        }
    }
}

#[test]
fn bond_lengths_within_0_1_angstrom_of_rdkit() {
    for (name, rec) in reference()["molecules"].as_object().unwrap() {
        let m = mol3d(rec["smiles"].as_str().unwrap());
        let c = &embed(&m, SEED, CONFORMERS).unwrap()[0];
        let r = ref_xyz(rec);
        let mut worst = 0.0f64;
        for &(a, b, _) in &m.bonds {
            worst = worst.max((dist(&c.xyz[a], &c.xyz[b]) - dist(&r[a], &r[b])).abs());
        }
        println!("bond {name} worst deviation {worst:.3}");
        assert!(worst <= 0.1, "{name}: worst bond deviation {worst:.3} A");
    }
}

/// Molecules without rotatable bonds between heavy atoms: the conformer is fixed by the ring system.
const RIGID: [&str; 8] = [
    "phenol",
    "aniline",
    "toluene",
    "chlorobenzene",
    "naphthalene",
    "nitrobenzene",
    "caffeine",
    "benzoic_acid",
];

#[test]
fn heavy_atom_rmsd_of_the_best_conformer() {
    let (mut rigid, mut all) = (Vec::new(), Vec::new());
    for (name, rec) in reference()["molecules"].as_object().unwrap() {
        let m = mol3d(rec["smiles"].as_str().unwrap());
        let confs = embed(&m, SEED, CONFORMERS).unwrap();
        let best = confs
            .iter()
            .map(|c| heavy_rmsd(&m, &c.xyz, rec))
            .fold(f64::INFINITY, f64::min);
        let first = heavy_rmsd(&m, &confs[0].xyz, rec);
        println!("rmsd {name} best {best:.3} lowest-energy {first:.3}");
        all.push(best);
        if RIGID.contains(&name.as_str()) {
            rigid.push(best);
        }
    }
    let (mr, ma) = (median(rigid), median(all));
    println!("rmsd median rigid {mr:.3} all {ma:.3}");
    assert!(mr <= 0.5, "median RMSD of the rigid molecules {mr:.3} A");
}

#[test]
fn same_seed_same_geometry() {
    let m = mol3d("CC(=O)Nc1ccc(O)cc1");
    let a = embed(&m, SEED, CONFORMERS).unwrap();
    let b = embed(&m, SEED, CONFORMERS).unwrap();
    assert_eq!(a.len(), b.len());
    for (x, y) in a.iter().zip(&b) {
        assert_eq!(x.xyz, y.xyz);
        assert_eq!(x.energy, y.energy);
    }
}

#[test]
fn unsupported_element_is_a_json_error() {
    // `grafo` itself refuses the elements the geometry does not cover; an atom changed by hand reaches the adapter
    let mut g = Mol::parse("CC").unwrap();
    g.atoms[0].el = "Se".into();
    let e = Mol3d::from_mol(&g).err().unwrap();
    assert_eq!(e.key, "error.chem.element");
}

#[test]
fn time_per_molecule() {
    for (name, smiles) in [
        ("paracetamol", "CC(=O)Nc1ccc(O)cc1"),
        ("diclofenac", "OC(=O)Cc1ccccc1Nc1c(Cl)cccc1Cl"),
    ] {
        let m = mol3d(smiles);
        for n in [DEFAULT_CONFORMERS, 8] {
            let t = Instant::now();
            embed(&m, SEED, n).unwrap();
            println!(
                "time {name} ({} atoms, {n} conformers): {:?}",
                m.len(),
                t.elapsed()
            );
        }
    }
}
