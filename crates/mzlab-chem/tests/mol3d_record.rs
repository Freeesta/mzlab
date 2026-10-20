#![cfg(feature = "record")]
//! `Mol3d` from a `mol/1` record against the one from the SMILES graph: same atoms, hydrogens, charges, aromatic flags and bonds (as a set: the record
//! sorts its bonds), over textbook molecules; a Kekulé parent through the record gets the aromatic flags of its lower-case twin.

use mzlab_chem::mol3d::Mol3d;
use mzlab_feat::chem::grafo::Mol;

const SMILES: [&str; 10] = [
    "Oc1ccccc1",
    "CCc1ccc(O)cc1",
    "COc1ccc(Cl)cc1",
    "CN(C)c1ccccc1",
    "CCN(CC)CC",
    "c1ccc2ccccc2c1",
    "CC(=O)Oc1ccccc1C(=O)O",
    "C[N+](C)(C)C",
    "Cn1cnc2c1c(=O)n(C)c(=O)n2C",
    "OCC(O)CO",
];

type Shape = (
    Vec<u8>,
    Vec<Option<usize>>,
    Vec<i8>,
    Vec<bool>,
    Vec<(usize, usize, u8)>,
);

fn shape(m: &Mol3d) -> Shape {
    let mut bonds: Vec<_> = m
        .bonds
        .iter()
        .map(|&(a, b, o)| (a.min(b), a.max(b), o))
        .collect();
    bonds.sort_unstable();
    (
        m.el.clone(),
        m.heavy_index.clone(),
        m.charge.clone(),
        m.arom.clone(),
        bonds,
    )
}

fn via_record(smiles: &str) -> Mol3d {
    Mol3d::from_record(&mzlab_mol::Mol::from_smiles(smiles).unwrap()).unwrap()
}

#[test]
fn record_gives_the_same_molecule_as_the_graph() {
    for s in SMILES {
        let legacy = Mol3d::from_mol(&Mol::parse(s).unwrap()).unwrap();
        assert_eq!(shape(&via_record(s)), shape(&legacy), "{s}");
    }
}

#[test]
fn a_kekule_parent_gets_the_aromatic_flags_of_its_twin() {
    assert_eq!(
        shape(&via_record("OC1=CC=CC=C1")),
        shape(&via_record("Oc1ccccc1"))
    );
}

#[test]
fn an_unsupported_element_is_refused() {
    let s = "C[Se]C";
    assert!(Mol::parse(s).is_err());
    let rec = mzlab_mol::Mol::from_smiles(s).unwrap();
    assert!(Mol3d::from_record(&rec).is_err());
}
