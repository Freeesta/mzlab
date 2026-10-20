//! The molfile reader against RDKit: every molfile in `tests/data/golden_molfile.json` (V2000 and V3000, with wedges, with the aromatic
//! bond type 4, without wedges) is compared with the record RDKit gives for the same molecule in `golden.json`; no RDKit is needed here.

mod common;

use common::{defined, golden, groups_json, same_bonds};
use mzlab_mol::{Mol, Status, Stereo};
use serde_json::Value;

fn molfiles() -> Vec<Value> {
    let g: Value = serde_json::from_str(include_str!("data/golden_molfile.json")).expect("golden");
    g["molfiles"].as_array().expect("molfiles").clone()
}

/// RDKit's record and aromatic pairs for the molecule written as `input`.
fn truth(input: &str) -> (Value, Value) {
    let g = golden();
    for c in g["cases"].as_array().expect("cases") {
        if c["input"] == input {
            return (c["records"]["input_order"].clone(), c["aromatic"].clone());
        }
    }
    for c in g["smiles_cases"].as_array().expect("smiles_cases") {
        if c["input"] == input {
            return (c["record"].clone(), c["aromatic"].clone());
        }
    }
    panic!("no golden record for {input}");
}

fn read(text: &str, input: &str) -> Mol {
    Mol::from_molfile(text).unwrap_or_else(|e| panic!("{input}: {e}"))
}

/// What the reader must give for a molfile that carries everything: atoms, bonds, the defined stereo and the groups of V3000.
fn same_as_rdkit(input: &str, text: &str, v3000_groups: bool) {
    let (rec, aromatic) = truth(input);
    let m = read(text, input);
    let mine: Value = serde_json::to_value(&m).expect("json");
    if input.contains("[nH]") && text.contains(" 4  0") {
        // aromatic bonds without an H count do not say which ring N carries the H (tautomers): the reader must still place the same number
        let hydrogens = |v: &Value| -> u64 {
            v["atoms"]
                .as_array()
                .expect("atoms")
                .iter()
                .map(|a| a["h"].as_u64().unwrap_or(0))
                .sum()
        };
        assert_eq!(hydrogens(&mine), hydrogens(&rec), "{input}: hydrogens");
        return;
    }
    assert_eq!(mine["atoms"], rec["atoms"], "{input}: atoms");
    same_bonds(&mine, &rec, &aromatic, input);
    assert_eq!(defined(&mine), defined(&rec), "{input}: defined stereo");
    if v3000_groups {
        let mut want: Vec<Value> = rec["groups"].as_array().expect("groups").clone();
        want.sort_by_key(|v| v["atoms"].to_string());
        assert_eq!(groups_json(&m), want, "{input}: groups");
    }
}

#[test]
fn molfiles_with_wedges_read_like_rdkit() {
    let all = molfiles();
    assert!(all.len() > 80);
    for f in &all {
        let text = f["mol"].as_str().expect("mol");
        same_as_rdkit(
            f["input"].as_str().expect("input"),
            text,
            text.contains("V3000"),
        );
    }
}

#[test]
fn aromatic_bond_type_four_is_kekulised() {
    let all: Vec<Value> = molfiles()
        .into_iter()
        .filter(|f| f.get("type4").is_some())
        .collect();
    assert!(all.len() > 20);
    for f in &all {
        let text = f["type4"].as_str().expect("type4");
        same_as_rdkit(
            f["input"].as_str().expect("input"),
            text,
            text.contains("V3000"),
        );
    }
}

#[test]
fn a_molfile_without_wedges_has_no_tetrahedral_element() {
    let all: Vec<Value> = molfiles()
        .into_iter()
        .filter(|f| f.get("nowedge").is_some())
        .collect();
    assert!(all.len() > 15);
    for f in &all {
        let input = f["input"].as_str().expect("input");
        let m = read(f["nowedge"].as_str().expect("nowedge"), input);
        let (rec, aromatic) = truth(input);
        let mine: Value = serde_json::to_value(&m).expect("json");
        assert_eq!(mine["atoms"], rec["atoms"], "{input}: atoms");
        same_bonds(&mine, &rec, &aromatic, input);
        let tetra = m.stereo.iter().any(|s| {
            matches!(
                s,
                Stereo::Tetra {
                    status: Status::Defined,
                    ..
                }
            )
        });
        assert!(!tetra, "{input}: stereo invented without wedges");
        assert!(
            m.groups.is_empty(),
            "{input}: group without a defined centre"
        );
    }
}

/// 2-butene drawn as a zigzag; `flag` is the bond stereo field of the double bond (3 is the "either" mark).
fn butene(flag: u8) -> String {
    format!(
        "
  mzlab

  4  3  0  0  0  0  0  0  0  0999 V2000
    0.0000    0.0000    0.0000 C   0  0  0  0  0  0  0  0  0  0  0  0
    1.2000    0.7000    0.0000 C   0  0  0  0  0  0  0  0  0  0  0  0
    2.4000    0.0000    0.0000 C   0  0  0  0  0  0  0  0  0  0  0  0
    3.6000    0.7000    0.0000 C   0  0  0  0  0  0  0  0  0  0  0  0
  1  2  1  0
  2  3  2{flag:3}
  3  4  1  0
M  END
"
    )
}

#[test]
fn drawn_double_bond_gives_z_or_e_and_the_either_flag_gives_unknown() {
    let drawn = butene(0);
    let m = read(&drawn, "butene");
    let db: Vec<_> = m
        .stereo
        .iter()
        .filter(|s| matches!(s, Stereo::Db { .. }))
        .collect();
    assert_eq!(db.len(), 1);
    assert!(matches!(
        db[0],
        Stereo::Db {
            status: Status::Defined,
            ..
        }
    ));
    let either = read(&butene(3), "butene either");
    assert!(either.stereo.iter().any(|s| matches!(
        s,
        Stereo::Db {
            status: Status::Unknown,
            refs: None,
            cfg: None,
            ..
        }
    )));
}

#[test]
fn opechemlib_reorders_atoms_when_a_hydrogen_is_explicit() {
    let g: Value = serde_json::from_str(include_str!("data/golden_molfile.json")).expect("golden");
    let all = g["openchemlib"].as_array().expect("openchemlib");
    let moved: Vec<_> = all.iter().filter(|c| c["order_kept"] == false).collect();
    assert!(!moved.is_empty(), "the trap needs at least one reordering");
    for c in all {
        let smiles = c["smiles"].as_str().expect("smiles");
        let labels: Vec<String> = c["labels"]
            .as_array()
            .expect("labels")
            .iter()
            .map(|l| l.as_str().unwrap_or_default().to_string())
            .collect();
        let m = Mol::from_smiles(smiles).unwrap_or_else(|e| panic!("{smiles}: {e}"));
        // OpenChemLib labels hydrogen isotopes D and T
        let ours: Vec<String> = m
            .atoms
            .iter()
            .map(|a| match (a.el.as_str(), a.iso) {
                ("H", 2) => "D".to_string(),
                ("H", 3) => "T".to_string(),
                (el, _) => el.to_string(),
            })
            .collect();
        // the reader keeps the order it is given; OpenChemLib's labels equal it only when `order_kept`
        assert_eq!(ours == labels, c["order_kept"] == true, "{smiles}");
    }
}

/// `kind+atoms:status` of every stereo element of a record.
fn statuses(rec: &Value) -> Vec<String> {
    let mut out: Vec<String> = rec["stereo"]
        .as_array()
        .expect("stereo")
        .iter()
        .map(|s| {
            let at = if s["t"] == "tetra" {
                format!("t{}", s["c"])
            } else {
                format!("d{}-{}", s["a"], s["b"])
            };
            format!("{at}:{}", s["status"].as_str().unwrap_or_default())
        })
        .collect();
    out.sort();
    out
}

#[test]
fn sites_of_a_molfile_are_defined_or_unknown_like_rdkit() {
    let (mut checked, mut skipped) = (0, 0);
    for f in molfiles() {
        let (input, text) = (
            f["input"].as_str().expect("input"),
            f["mol"].as_str().expect("mol"),
        );
        if input.contains("[nH]") && text.contains(" 4  0") {
            continue;
        }
        let (rec, _) = truth(input);
        let mine = serde_json::to_value(read(text, input).with_stereo_sites()).expect("json");
        // RDKit marks every site it cannot place from the file as unknown too, so statuses must agree site by site
        if statuses(&mine) != statuses(&rec) {
            skipped += 1;
            eprintln!(
                "{input}\n   mine {:?}\n   rdkit {:?}",
                statuses(&mine),
                statuses(&rec)
            );
        }
        checked += 1;
    }
    assert!(
        checked > 80 && skipped == 0,
        "{skipped} of {checked} differ"
    );
}
