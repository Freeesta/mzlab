//! `descriptors/1` (`chem::descriptors`, the one function of mzDraw, the molkit `.wasm`, the app and `mzlab-cli mol measure`): the bytes of
//! `tests/dati/descriptors_disegno.json` (V2000 and V3000, read by the desktop and molkit parity tests too), the RDKit values of the panel of
//! `crates/mzlab-mol/tests/descr.rs`, `analysis` equal to `analysis_json` of the same atoms, and keyed errors.

use mzlab_feat::chem::analysis::analysis_json;
use mzlab_feat::chem::descriptors::descriptors_json;
use serde_json::{json, Value};

fn fixture() -> Value {
    serde_json::from_str(include_str!("../../../tests/dati/descriptors_disegno.json")).unwrap()
}

#[test]
fn every_case_gives_the_bytes_of_the_fixture() {
    let fx = fixture();
    for c in fx["cases"].as_array().unwrap() {
        let got = descriptors_json(&c["request"].to_string()).unwrap();
        assert_eq!(got, c["descriptors"].as_str().unwrap(), "{}", c["name"]);
    }
    // a V3000 file and the V2000 one of the same molecule give the same bytes
    let cases = fx["cases"].as_array().unwrap();
    assert_eq!(cases[0]["descriptors"], cases[1]["descriptors"]);
}

#[test]
fn the_values_are_the_rdkit_ones_of_the_panel() {
    let fx = fixture();
    let d = |i: usize| -> Value {
        serde_json::from_str(fx["cases"][i]["descriptors"].as_str().unwrap()).unwrap()
    };
    // (case, logP, MR, TPSA, rotors, MW) from crates/mzlab-mol/tests/descr.rs
    for (i, logp, mr, tpsa, rotors, mw) in [
        (0, -1.0293, 51.196, 61.82, 0, 194.194),
        (2, 3.0732, 61.035, 37.30, 4, 206.285),
        (3, 1.6866, 26.442, 0.0, 0, 78.114),
    ] {
        let v = d(i);
        assert!((v["logp"].as_f64().unwrap() - logp).abs() < 1e-4, "{i}");
        assert!((v["mr"].as_f64().unwrap() - mr).abs() < 1e-3, "{i}");
        assert!((v["tpsa"].as_f64().unwrap() - tpsa).abs() < 1e-2, "{i}");
        assert_eq!(v["rotors"], rotors, "{i}");
        assert!((v["mw"].as_f64().unwrap() - mw).abs() < 1e-3, "{i}");
    }
    let ion = d(4);
    assert_eq!(ion["analysis"]["structure"]["charge"], -1);
    assert_eq!(ion["analysis"]["ions"][0]["ion"], "[M]-");
}

#[test]
fn the_analysis_is_the_one_of_mol_anal() {
    // the 13C acetate anion: two carbons (one 13C), three H on it, the carboxylate
    let fx = fixture();
    let d: Value = serde_json::from_str(fx["cases"][4]["descriptors"].as_str().unwrap()).unwrap();
    let anal = json!({"atoms": [
        {"el": "C", "iso": 13, "h": 3}, {"el": "C"}, {"el": "O"}, {"el": "O", "q": -1}
    ]});
    let want: Value = serde_json::from_str(&analysis_json(&anal.to_string()).unwrap()).unwrap();
    assert_eq!(d["analysis"], want);
}

#[test]
fn bad_requests_keep_their_keys() {
    let key = |r: &str| descriptors_json(r).unwrap_err().key;
    assert_eq!(key("{"), "error.descr.request");
    assert_eq!(key(r#"{"smiles":"CCO"}"#), "error.descr.request");
    assert_eq!(key(r#"{"molfile":"nonsense"}"#), "error.descr.molfile");
    let empty = "\n  t\n\n  0  0  0  0  0  0  0  0  0  0999 V2000\nM  END\n";
    assert_eq!(
        key(&json!({ "molfile": empty }).to_string()),
        "error.anal.empty"
    );
}
