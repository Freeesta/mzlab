//! `cip/1` against the CIP labeler of RDKit (`tests/data/golden_cip.json`, written by `tools/cip_riferimento.py`): every molecule read from
//! its SMILES by `mzlab-mol`, and one in four from the 2D molfile the Draw page would send, must get exactly the R/S, r/s and E/Z of RDKit.
use mzlab_feat::chem::cip::{cip_json, cip_labels};
use mzlab_mol::Mol;
use serde_json::Value;

fn golden() -> Value {
    let text = std::fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/data/golden_cip.json"
    ))
    .unwrap();
    serde_json::from_str(&text).unwrap()
}

/// The labels of a golden entry as `(atoms, bonds)` strings, comparable with ours.
fn want(e: &Value) -> (Vec<String>, Vec<String>) {
    let atoms = e["atoms"]
        .as_array()
        .unwrap()
        .iter()
        .map(|a| format!("{}{}", a[0], a[1].as_str().unwrap()))
        .collect();
    let bonds = e["bonds"]
        .as_array()
        .unwrap()
        .iter()
        .map(|b| format!("{}-{}{}", b[0], b[1], b[2].as_str().unwrap()))
        .collect();
    (atoms, bonds)
}

#[test]
fn smiles_match_rdkit() {
    let g = golden();
    let list = g["smiles"].as_array().unwrap();
    assert!(list.len() >= 30);
    let mut bad = Vec::new();
    for e in list {
        let smi = e["smiles"].as_str().unwrap();
        let m = Mol::from_smiles(smi).unwrap();
        let l = cip_labels(&m, &m.stereo_sites());
        let got = (
            l.atoms
                .iter()
                .map(|(a, s)| format!("{a}{s}"))
                .collect::<Vec<_>>(),
            l.bonds
                .iter()
                .map(|(a, b, s)| format!("{a}-{b}{s}"))
                .collect::<Vec<_>>(),
        );
        if got != want(e) {
            bad.push(format!("{smi}: ours {got:?}, RDKit {:?}", want(e)));
        }
    }
    assert!(
        bad.is_empty(),
        "{} of {} differ:\n{}",
        bad.len(),
        list.len(),
        bad.join("\n")
    );
}

#[test]
fn molfiles_match_rdkit() {
    let g = golden();
    let list = g["molfile"].as_array().unwrap();
    assert!(list.len() >= 30);
    for e in list {
        let req = serde_json::json!({ "molfile": e["molfile"] }).to_string();
        let out: Value = serde_json::from_str(&cip_json(&req).unwrap()).unwrap();
        assert_eq!(out["format"], "cip/1");
        let atoms: Vec<String> = out["atoms"]
            .as_array()
            .unwrap()
            .iter()
            .map(|a| format!("{}{}", a["atom"], a["label"].as_str().unwrap()))
            .collect();
        let bonds: Vec<String> = out["bonds"]
            .as_array()
            .unwrap()
            .iter()
            .map(|b| format!("{}-{}{}", b["a"], b["b"], b["label"].as_str().unwrap()))
            .collect();
        assert_eq!((atoms, bonds), want(e), "{}", e["smiles"]);
    }
}

#[test]
fn drawings_without_labels_say_why() {
    let query = "\n  t\n\n  2  1  0  0  0  0  0  0  0  0999 V2000\n    0.0000    0.0000    0.0000 C   0  0  0  0  0  0  0  0  0  0  0  0\n    1.0000    0.0000    0.0000 A   0  0  0  0  0  0  0  0  0  0  0  0\n  1  2  1  0  0  0  0\nM  END\n";
    let out = cip_json(&serde_json::json!({ "molfile": query }).to_string()).unwrap();
    assert_eq!(
        out,
        r#"{"format":"cip/1","atoms":[],"bonds":[],"skip":"query"}"#
    );
    assert_eq!(cip_json("{}").unwrap_err().key, "error.cip.request");
    assert_eq!(
        cip_json(r#"{"molfile":"x"}"#).unwrap_err().key,
        "error.cip.molfile"
    );
}
