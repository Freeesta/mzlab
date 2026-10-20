//! The graph built from a `mol/1` record against the one the SMILES and molfile readers of `grafo` build: same atoms, hydrogens, charges, aromatic
//! flags and bonds (as a set: the record sorts them by atom pair), over the study cases, the extra cases and the 625 compounds of composti.csv.

use mzlab_feat::chem::grafo::Mol;
use serde_json::Value;

type Key = (String, bool, u32, i32);

fn shape(m: &Mol) -> (Vec<Key>, Vec<(usize, usize, u32)>) {
    let atoms = m
        .atoms
        .iter()
        .map(|a| (a.el.clone(), a.arom, a.h, a.charge))
        .collect();
    let mut bonds: Vec<_> = m
        .bonds
        .iter()
        .map(|b| (b.a.min(b.b), b.a.max(b.b), (b.order * 2.0) as u32))
        .collect();
    bonds.sort_unstable();
    (atoms, bonds)
}

/// The graph of `smiles` through the record, with aromaticity perceived exactly when the legacy reader found any.
fn same(smiles: &str) {
    let legacy = Mol::parse(smiles).unwrap_or_else(|e| panic!("{smiles}: {e:?}"));
    let rec = mzlab_mol::Mol::from_smiles(smiles).unwrap_or_else(|e| panic!("{smiles}: {e}"));
    let aromatic = legacy.atoms.iter().any(|a| a.arom);
    let view = Mol::from_record(&rec, aromatic).unwrap_or_else(|e| panic!("{smiles}: {e:?}"));
    assert_eq!(shape(&view), shape(&legacy), "{smiles}");
    assert_eq!(view.formula(), legacy.formula(), "{smiles}: formula");
}

fn golden() -> Value {
    serde_json::from_str(include_str!("../../mzlab-mol/tests/data/golden.json")).expect("golden")
}

#[test]
fn study_and_extra_cases_give_the_same_graph() {
    let g = golden();
    let mut n = 0;
    for key in ["cases", "smiles_cases"] {
        for c in g[key].as_array().expect("cases") {
            let s = c["input"].as_str().unwrap_or_default();
            // salts, isotope-only and exotic elements are refused by the legacy reader
            if Mol::parse(s).is_ok() {
                same(s);
                n += 1;
            }
        }
    }
    assert!(n > 60, "{n} cases compared");
}

#[test]
fn composti_give_the_same_graph() {
    let text = std::fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../TP_Mine/data/composti.csv"
    ))
    .expect("composti.csv");
    let mut n = 0;
    for line in text.lines().skip(1).filter(|l| !l.is_empty()) {
        // the SMILES is the second from the right (the names on the left may hold commas)
        let smiles = line.rsplit(',').nth(1).unwrap_or_default();
        if Mol::parse(smiles).is_ok() {
            same(smiles);
            n += 1;
        }
    }
    assert!(n > 500, "{n} compounds compared");
}

#[test]
fn a_molfile_gives_the_same_graph() {
    let g: Value = serde_json::from_str(include_str!(
        "../../mzlab-mol/tests/data/golden_molfile.json"
    ))
    .expect("golden");
    let (mut n, mut moved) = (0, 0);
    for c in g["molfiles"].as_array().expect("molfiles") {
        let mf = c["mol"].as_str().unwrap_or_default();
        let (Ok(legacy), Ok(rec)) = (Mol::from_molfile(mf), mzlab_mol::Mol::from_molfile(mf))
        else {
            continue;
        };
        // the legacy reader takes V2000 only
        if legacy.atoms.is_empty() {
            continue;
        }
        let aromatic = legacy.atoms.iter().any(|a| a.arom);
        let view = Mol::from_record(&rec, aromatic).expect("view");
        let (mine, theirs) = (shape(&view), shape(&legacy));
        // the legacy molfile reader knows no radicals and adds a charge's hydrogen shift on top of the valence it already got from the bond orders:
        // only the hydrogens of charged or radical atoms may differ, and the record is the right one (N+ with a double bond and two single bonds has none)
        let odd: Vec<bool> = (0..rec.atoms.len())
            .map(|i| legacy.atoms[i].charge != 0 || rec.atoms[i].rad != 0)
            .collect();
        if mine != theirs {
            assert_eq!(mine.1, theirs.1, "{mf}: bonds");
            for (i, (x, y)) in mine.0.iter().zip(&theirs.0).enumerate() {
                assert!(
                    x == y || (odd[i] && x.0 == y.0 && x.1 == y.1),
                    "{mf}: atom {i}"
                );
            }
            moved += 1;
        }
        n += 1;
    }
    assert!(n > 40, "{n} molfiles compared");
    assert!(
        moved * 10 < n,
        "{moved} of {n} differ in a charged or radical atom"
    );
}

#[test]
fn a_record_that_the_graph_cannot_hold_is_refused() {
    let rec = mzlab_mol::Mol::from_smiles("[Na+].[Cl-]").expect("record");
    assert!(Mol::from_record(&rec, false).is_err());
    let big = mzlab_mol::Mol::from_smiles(&"C".repeat(201)).expect("record");
    assert!(Mol::from_record(&big, false).is_err());
}
