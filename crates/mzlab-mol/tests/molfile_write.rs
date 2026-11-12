//! The molfile writer read back by the molfile reader: every molfile of `tests/data/golden_molfile.json` (V2000 and V3000, with wedges, type 4,
//! without wedges) gives the same `mol/1` record after `to_molfile` and `from_molfile`; groups and large records go through V3000.

use mzlab_mol::{Mol, Status, Stereo};
use serde_json::Value;

fn molfiles() -> Vec<(String, String)> {
    let g: Value = serde_json::from_str(include_str!("data/golden_molfile.json")).expect("golden");
    let mut out = Vec::new();
    for c in g["molfiles"].as_array().expect("molfiles") {
        for k in ["mol", "nowedge", "type4"] {
            if let Some(t) = c[k].as_str() {
                out.push((format!("{} ({k})", c["input"]), t.to_string()));
            }
        }
    }
    out
}

/// The record with its coordinates at the four decimals of the V2000 columns (V3000 keeps every digit).
fn at4(m: &Mol) -> String {
    let mut m = m.clone();
    if let Some(c) = m.coords.as_mut() {
        for r in &mut c.xy {
            for v in r {
                *v = (*v * 1e4).round() / 1e4;
            }
        }
    }
    m.to_json()
}

fn back(m: &Mol) -> Mol {
    Mol::from_molfile(&m.to_molfile("t")).unwrap_or_else(|e| panic!("{e}\n{}", m.to_molfile("t")))
}

#[test]
fn every_golden_molfile_survives_a_write_and_a_read() {
    let cases = molfiles();
    assert!(cases.len() >= 92, "{}", cases.len());
    let (mut tetra, mut v3000) = (0, 0);
    for (name, text) in cases {
        let m = Mol::from_molfile(&text).unwrap_or_else(|e| panic!("{name}: {e}"));
        let out = m.to_molfile(&name);
        v3000 += usize::from(out.contains("V3000"));
        tetra += m
            .stereo
            .iter()
            .filter(|s| matches!(s, Stereo::Tetra { status: Status::Defined, .. }))
            .count();
        assert_eq!(at4(&back(&m)), at4(&m), "{name}\n{out}");
        if out.contains("V3000") {
            assert_eq!(back(&m).to_json(), m.to_json(), "{name}: V3000 keeps every digit");
        }
    }
    assert!(tetra >= 20, "defined centres: {tetra}");
    assert!(v3000 >= 1, "the groups of the V3000 files go through V3000");
}

#[test]
fn charges_isotopes_radicals_and_odd_hydrogens_round_trip() {
    // [13CH3][N+](C)(C)C, a carbon radical, a carbene, D2O, [NH4+], a borane anion, sulfur at valence 4, a bare [Na+]
    for smi in [
        "[13CH3][N+](C)(C)C",
        "C[CH2]",
        "[CH2]",
        "[2H]O[2H]",
        "[NH4+]",
        "[BH4-]",
        "CS(C)=O",
        "O=S(=O)(O)O",
        "[Na+].[Cl-]",
        "[SH]",
        "C1=CC=CC=C1",
    ] {
        let m = Mol::from_smiles(smi).unwrap();
        assert_eq!(back(&m).to_json(), m.to_json(), "{smi}\n{}", m.to_molfile(smi));
    }
}

#[test]
fn groups_and_large_records_are_written_as_v3000() {
    let g: Value = serde_json::from_str(include_str!("data/golden_molfile.json")).unwrap();
    let with_groups = g["molfiles"]
        .as_array()
        .unwrap()
        .iter()
        .filter_map(|c| c["mol"].as_str())
        .map(|t| Mol::from_molfile(t).unwrap())
        .find(|m| !m.groups.is_empty())
        .expect("a golden molfile with groups");
    assert!(with_groups.to_molfile("g").contains("M  V30 BEGIN COLLECTION"));
    let chain = format!("C{}", "C".repeat(1199));
    let m = Mol::from_smiles(&chain).unwrap();
    let out = m.to_molfile("chain");
    assert!(out.lines().nth(3).unwrap().contains("V3000"));
    assert!(out.lines().all(|l| l.len() <= 80));
    assert_eq!(back(&m).to_json(), m.to_json());
}

#[test]
fn the_pieces_of_a_record_are_records_of_their_own() {
    let m = Mol::from_smiles("CC(=O)[O-].[Na+].c1ccccc1").unwrap();
    let p = m.pieces();
    assert_eq!(p.len(), 3);
    assert_eq!(p[0].atoms.len(), 4);
    assert_eq!(p[1].atoms[0].el, "Na");
    assert_eq!(p[2].atoms.len(), 6);
    for x in &p {
        x.validate().unwrap();
    }
    let one = Mol::from_smiles("CCO").unwrap();
    assert_eq!(one.pieces(), vec![one.clone()]);
}

#[test]
fn the_title_is_one_line_and_the_file_is_deterministic() {
    let m = Mol::from_smiles("CCO").unwrap();
    let a = m.to_molfile("ethanol\nsecond line");
    assert!(a.starts_with("ethanol\n  mzLab"));
    assert_eq!(a, m.to_molfile("ethanol"));
    assert!(a.ends_with("M  END\n"));
}
