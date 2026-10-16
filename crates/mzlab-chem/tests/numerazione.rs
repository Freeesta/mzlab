//! Atom numbering of `Mol3d::from_mol`: heavy atoms in the order of `grafo::Mol`, then the hydrogens of each heavy atom in turn;
//! same elements and bonds as the RDKit reference (`tests/data/cc_ref.json`) for the 20 molecules.

use mzlab_chem::mol3d::{atomic_number, Mol3d};
use mzlab_feat::chem::grafo::Mol;

fn reference() -> serde_json::Value {
    let text = std::fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/data/cc_ref.json"
    ))
    .unwrap();
    serde_json::from_str(&text).unwrap()
}

#[test]
fn heavy_atoms_keep_the_grafo_order() {
    for smi in ["CC(=O)Nc1ccc(O)cc1", "NC(=O)N1c2ccccc2C=Cc2ccccc21"] {
        let g = Mol::parse(smi).unwrap();
        let m = Mol3d::from_mol(&g).unwrap();
        for (i, a) in g.atoms.iter().enumerate() {
            assert_eq!(m.heavy_index[i], Some(i));
            assert_eq!(Some(m.el[i]), atomic_number(&a.el));
        }
        let hs = m.heavy_index.iter().filter(|h| h.is_none()).count();
        assert_eq!(hs as u32, g.atoms.iter().map(|a| a.h).sum::<u32>());
        assert!(m.heavy_index[g.atoms.len()..].iter().all(|h| h.is_none()));
    }
}

#[test]
fn same_atoms_and_bonds_as_rdkit() {
    let r = reference();
    for (name, rec) in r["molecules"].as_object().unwrap() {
        let g =
            Mol::parse(rec["smiles"].as_str().unwrap()).unwrap_or_else(|e| panic!("{name}: {e:?}"));
        let m = Mol3d::from_mol(&g).unwrap();
        let z: Vec<u8> = rec["z"]
            .as_array()
            .unwrap()
            .iter()
            .map(|v| v.as_u64().unwrap() as u8)
            .collect();
        assert_eq!(m.el, z, "{name}: elements in order");
        let mut mine: Vec<(usize, usize, u8)> = m
            .bonds
            .iter()
            .map(|&(a, b, o)| (a.min(b), a.max(b), o))
            .collect();
        let mut theirs: Vec<(usize, usize, u8)> = rec["bonds"]
            .as_array()
            .unwrap()
            .iter()
            .map(|b| {
                let (a, c) = (
                    b[0].as_u64().unwrap() as usize,
                    b[1].as_u64().unwrap() as usize,
                );
                let o = b[2].as_f64().unwrap();
                (a.min(c), a.max(c), if o == 1.5 { 4 } else { o as u8 })
            })
            .collect();
        mine.sort();
        theirs.sort();
        // `grafo` calls a bond aromatic when both atoms are written aromatic; RDKit perceives rings (the six-membered ring of caffeine is
        // not aromatic for it): the bonds must join the same atoms, the orders must agree where neither side says aromatic.
        assert_eq!(mine.len(), theirs.len(), "{name}: bond count");
        for (a, b) in mine.iter().zip(&theirs) {
            assert_eq!((a.0, a.1), (b.0, b.1), "{name}: bond ends");
            assert!(
                a.2 == b.2 || a.2 == 4 || b.2 == 4,
                "{name}: order of bond {a:?} vs {b:?}"
            );
        }
    }
}
