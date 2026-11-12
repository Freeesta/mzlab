//! `sdf/1`: a `.mol` keeps the drawing in one record, an `.sdf` has one record per structure with the mzLab data items; every record reads
//! back through the `mol/1` reader as the structure it came from; bad requests give their error keys.

use mzlab_feat::chem::sdf::sdf_json;
use mzlab_mol::Mol;

fn request(smiles: &str, format: &str, ion: &str) -> String {
    let mf = Mol::from_smiles(smiles).unwrap().to_molfile("");
    format!(
        "{{\"molfile\":{},\"format\":\"{format}\",\"ion\":\"{ion}\",\"name\":\"draw\"}}",
        serde_json::to_string(&mf).unwrap()
    )
}

/// The value of a data item of one SDF record.
fn item<'a>(record: &'a str, key: &str) -> Option<&'a str> {
    let tag = format!("> <{key}>\n");
    record.split(&tag).nth(1).and_then(|r| r.lines().next())
}

#[test]
fn an_sdf_has_one_record_per_structure_with_its_data_items() {
    let out = sdf_json(&request("CC(=O)Nc1ccc(O)cc1.[Na+].[Cl-]", "sdf", "[M+H]+")).unwrap();
    let recs: Vec<&str> = out.split("$$$$\n").filter(|r| !r.is_empty()).collect();
    assert_eq!(recs.len(), 3, "{out}");
    assert!(out.ends_with("$$$$\n"));
    let para = recs[0];
    assert!(para.starts_with("draw\n"));
    assert_eq!(item(para, "MZLAB_FORMULA"), Some("C8H9NO2"));
    assert_eq!(item(para, "MZLAB_EXACT_MASS"), Some("151.06333"));
    assert_eq!(item(para, "MZLAB_ION"), Some("[M+H]+"));
    assert!(
        item(para, "MZLAB_MZ").unwrap().starts_with("152.0706"),
        "{para}"
    );
    assert!(item(para, "MZLAB_HASH").unwrap().starts_with("mol1:"));
    // the drawn ions keep their own m/z (mass - charge x electron), whatever adduct was asked
    assert_eq!(item(recs[1], "MZLAB_FORMULA"), Some("Na+"));
    assert_eq!(item(recs[1], "MZLAB_ION"), Some("drawn"));
    assert_eq!(item(recs[1], "MZLAB_MZ"), Some("22.98922"));
    assert_eq!(item(recs[2], "MZLAB_FORMULA"), Some("Cl-"));
    let whole = Mol::from_smiles("CC(=O)Nc1ccc(O)cc1.[Na+].[Cl-]").unwrap();
    for (r, p) in recs.iter().zip(whole.pieces()) {
        assert_eq!(Mol::from_molfile(r).unwrap().to_json(), p.to_json());
    }
}

#[test]
fn the_hash_is_the_same_for_two_atom_orders_and_no_ion_gives_no_mz() {
    let a = sdf_json(&request("OCC", "sdf", "none")).unwrap();
    let b = sdf_json(&request("CCO", "sdf", "none")).unwrap();
    assert_eq!(item(&a, "MZLAB_HASH"), item(&b, "MZLAB_HASH"));
    assert_eq!(item(&a, "MZLAB_ION"), None);
    assert_eq!(item(&a, "MZLAB_MZ"), None);
    assert_eq!(item(&a, "MZLAB_FORMULA"), Some("C2H6O"));
    let neg = sdf_json(&request("CC(=O)O", "sdf", "[M-H]-")).unwrap();
    assert_eq!(item(&neg, "MZLAB_MZ"), Some("59.01385"));
}

#[test]
fn a_mol_keeps_the_whole_drawing_in_one_record_without_data_items() {
    let out = sdf_json(&request("CCO.[Na+]", "mol", "[M+H]+")).unwrap();
    assert!(out.ends_with("M  END\n") && !out.contains("$$$$") && !out.contains("> <"));
    let back = Mol::from_molfile(&out).unwrap();
    assert_eq!(
        back.to_json(),
        Mol::from_smiles("CCO.[Na+]").unwrap().to_json()
    );
}

#[test]
fn the_stereo_of_a_drawn_molfile_is_kept_in_the_sdf() {
    let g: serde_json::Value = serde_json::from_str(include_str!(
        "../../mzlab-mol/tests/data/golden_molfile.json"
    ))
    .unwrap();
    let mf = g["molfiles"]
        .as_array()
        .unwrap()
        .iter()
        .filter_map(|c| c["mol"].as_str())
        .find(|t| Mol::from_molfile(t).is_ok_and(|m| !m.stereo.is_empty() && m.pieces().len() == 1))
        .unwrap();
    let req = format!(
        "{{\"molfile\":{},\"format\":\"sdf\"}}",
        serde_json::to_string(mf).unwrap()
    );
    let out = sdf_json(&req).unwrap();
    let (a, b) = (
        Mol::from_molfile(&out).unwrap(),
        Mol::from_molfile(mf).unwrap(),
    );
    assert_eq!(a.stereo, b.stereo);
    assert!(!a.stereo.is_empty());
}

#[test]
fn bad_requests_give_their_keys() {
    let key = |r: &str| sdf_json(r).err().map(|e| e.key);
    assert_eq!(key("nope"), Some("error.sdf.request"));
    assert_eq!(key(r#"{"format":"sdf"}"#), Some("error.sdf.request"));
    assert_eq!(key(r#"{"molfile":"x"}"#), Some("error.sdf.molfile"));
    assert_eq!(
        key(&request("CCO", "sdf", "[M+Xe]+")),
        Some("error.sdf.ion")
    );
    assert_eq!(
        key(&request("CCO", "smi", "none")),
        Some("error.sdf.request")
    );
}

#[test]
fn formula_and_mass_are_the_etich_ones_and_the_adducts_the_engine_ones() {
    for smi in [
        "CC(=O)Nc1ccc(O)cc1",
        "[13CH3]C([2H])([2H])O",
        "N",
        "C[NH3+]",
        "OC(=O)CC([O-])=O",
        "[O-]C(=O)CC([O-])=O",
        "ClC(Br)(I)F",
        "[Na+]",
        "OP(=O)(O)OCC(N)=O",
        "c1ccncc1S",
    ] {
        let m = Mol::from_smiles(smi).unwrap();
        let atoms: Vec<String> = m
            .atoms
            .iter()
            .map(|a| {
                format!(
                    "{{\"el\":\"{}\",\"iso\":{},\"h\":{},\"q\":{}}}",
                    a.el, a.iso, a.h, a.chg
                )
            })
            .collect();
        let etich: serde_json::Value = serde_json::from_str(
            &mzlab_feat::chem::labels::labels_json(&format!(
                "{{\"structures\":[[{}]]}}",
                atoms.join(",")
            ))
            .unwrap(),
        )
        .unwrap();
        let e = &etich["structures"][0];
        let (f, q, mass) = mzlab_feat::chem::sdf::formula(&m);
        assert_eq!(f, e["text"].as_str().unwrap(), "{smi}");
        assert_eq!(q, e["charge"].as_i64().unwrap(), "{smi}");
        assert!(
            (mass - e["mass"].as_f64().unwrap()).abs() < 1e-9,
            "{smi}: {mass} {}",
            e["mass"]
        );
    }
    // the nuclide table's Na is 4e-7 Da off the engine's `MASS` table: below the five decimals of MZLAB_MZ
    for (name, shift) in mzlab_core::elements::adducts() {
        let mine = mzlab_feat::chem::sdf::adduct_shift(name).unwrap();
        assert!((mine - shift).abs() < 1e-6, "{name}: {mine} {shift}");
    }
}
