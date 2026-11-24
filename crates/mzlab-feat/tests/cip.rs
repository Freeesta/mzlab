//! `cip/1` against the CIP labeler of RDKit (`tests/data/golden_cip.json`, written by `tools/cip_riferimento.py`): every molecule read from
//! its SMILES by `mzlab-mol`, and one in four from the 2D molfile the Draw page would send, must get exactly the R/S, r/s and E/Z of RDKit;
//! piece by piece the labels are those of the whole record, 40 copies take well under a second, and one chain past the work limit or one
//! digraph past its node cap is refused.
use mzlab_feat::chem::cip::{cip_json, cip_labels, drawing_labels, WORK_LIMIT};
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
        assert_eq!(out["schema"], "cip/1");
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
        r#"{"schema":"cip/1","atoms":[],"bonds":[],"skip":"query"}"#
    );
    assert_eq!(cip_json("{}").unwrap_err().key, "error.cip.request");
    assert_eq!(
        cip_json(r#"{"molfile":"x"}"#).unwrap_err().key,
        "error.cip.molfile"
    );
}

/// The drawing of a record as `cip_json` reads it (no query atom, Kekulé form found).
fn drawing(m: Mol) -> mzlab_mol::Drawing {
    mzlab_mol::Drawing {
        mol: m,
        kekule: true,
        aromatic_h: Vec::new(),
        arom_atoms: Vec::new(),
        query_atoms: Vec::new(),
        query_bonds: Vec::new(),
        wedges: Vec::new(),
        sites: Vec::new(),
    }
}

/// Labels as comparable strings, in their order.
fn flat(l: &mzlab_feat::chem::cip::CipLabels) -> (Vec<String>, Vec<String>) {
    (
        l.atoms.iter().map(|(a, s)| format!("{a}{s}")).collect(),
        l.bonds
            .iter()
            .map(|(a, b, s)| format!("{a}-{b}{s}"))
            .collect(),
    )
}

/// Piece by piece, testing only the defined sites, gives what the whole record with all its sites gave (the labels before the split).
fn same_as_whole_record(smiles: &str) {
    let m = Mol::from_smiles(smiles).unwrap_or_else(|e| panic!("{smiles}: {e}"));
    let old = cip_labels(&m, &m.stereo_sites());
    let new = drawing_labels(&drawing(m)).unwrap_or_else(|e| panic!("{smiles}: {}", e.to_json()));
    assert_eq!(flat(&new), flat(&old), "{smiles}");
}

#[test]
fn pieces_give_the_labels_of_the_whole_record_on_the_goldens() {
    for e in golden()["smiles"].as_array().unwrap() {
        same_as_whole_record(e["smiles"].as_str().unwrap());
    }
}

#[test]
fn pieces_give_the_labels_of_the_whole_record_on_random_mixtures() {
    let g = golden();
    let pool: Vec<&str> = g["smiles"]
        .as_array()
        .unwrap()
        .iter()
        .map(|e| e["smiles"].as_str().unwrap().split(' ').next().unwrap())
        .collect();
    let mut state = 0x2545_F491_4F6C_DD1Du64;
    let mut next = |n: usize| {
        state = state
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1_442_695_040_888_963_407);
        (state >> 33) as usize % n
    };
    for _ in 0..150 {
        let k = 2 + next(4);
        let parts: Vec<&str> = (0..k).map(|_| pool[next(pool.len())]).collect();
        same_as_whole_record(&parts.join("."));
    }
}

/// The amide of the report: two cis/trans double bonds on a 1,4-disubstituted cyclohexane (ring cis/trans), defined or open.
const DEFINED: &str = "CC(C)/C=C/C(=O)N[C@H]1CC[C@H](CC1)/C=C/C";
const OPEN: &str = "CC(C)C=CC(=O)NC1CCC(CC1)C=CC";

#[test]
fn forty_copies_are_labelled_copy_by_copy_well_under_a_second() {
    let one = flat(&drawing_labels(&drawing(Mol::from_smiles(DEFINED).unwrap())).unwrap());
    let n = Mol::from_smiles(DEFINED).unwrap().atoms.len();
    let many = drawing(Mol::from_smiles(&[DEFINED; 40].join(".")).unwrap());
    let t = std::time::Instant::now();
    let l = drawing_labels(&many).unwrap();
    let took = t.elapsed();
    assert_eq!((l.atoms.len(), l.bonds.len()), (80, 80));
    for (k, (a, s)) in l.atoms.iter().enumerate() {
        assert_eq!(format!("{}{s}", a - (k / 2 * n) as u32), one.0[k % 2]);
    }
    for (k, (a, b, s)) in l.bonds.iter().enumerate() {
        let off = (k / 2 * n) as u32;
        assert_eq!(format!("{}-{}{s}", a - off, b - off), one.1[k % 2]);
    }
    // about 60 ms native in release; the bound leaves room for a debug build
    let most = if cfg!(debug_assertions) { 10.0 } else { 1.0 };
    assert!(took.as_secs_f64() < most, "40 copies took {took:?}");
}

#[test]
fn one_chain_past_the_work_limit_is_refused_with_its_evidence() {
    let chain = drawing(Mol::from_smiles(&[DEFINED; 8].join("")).unwrap());
    let e = drawing_labels(&chain).err().unwrap();
    assert_eq!(e.key, "error.cip.tooComplex");
    let p = |k: &str| e.params.iter().find(|x| x.0 == k).unwrap().1.clone();
    // eight copies of two centres and two double bonds: every defined element of the chain, whatever the stage
    assert_eq!(
        (p("stage"), p("atoms"), p("elements")),
        ("sites".into(), "136".into(), "32".into())
    );
    assert_eq!(p("limit"), WORK_LIMIT.to_string());
    assert!(p("work").parse::<u64>().unwrap() > WORK_LIMIT);
    // the same chain four copies long still finishes, with the labels of the whole record
    same_as_whole_record(&[DEFINED; 4].join(""));
}

#[test]
fn one_digraph_past_its_node_cap_is_refused_not_left_unlabelled() {
    // two dodecahedranyl arms told apart only by one carbon-13: the digraph of the centre passes 40,000 nodes long before the work limit
    let cage = "C12C3C4C5C1C6C7C2C8C3C9C4C%10C5C6C%11C7C8C9C%10%11";
    let heavy = cage.replace("C%10%11", "[13CH]%10%11");
    let m = Mol::from_smiles(&format!("F[C@H]({cage}){heavy}")).unwrap();
    // without the refusal the label was simply missing
    assert!(cip_labels(&m, &m.stereo_sites()).atoms.is_empty());
    let e = drawing_labels(&drawing(m.clone())).err().unwrap();
    assert_eq!(e.key, "error.cip.digraphTooBig");
    let p = |k: &str| e.params.iter().find(|x| x.0 == k).unwrap().1.clone();
    let got: Vec<String> = ["atoms", "elements", "nodes", "limit"]
        .iter()
        .map(|k| p(k))
        .collect();
    assert_eq!(got, ["42", "1", "40001", "40000"]);
    // a small piece beside it is not labelled alone: the drawing is refused whole, never in part
    let two = Mol::from_smiles(&format!("F[C@H](Cl)Br.F[C@H]({cage}){heavy}")).unwrap();
    let e = drawing_labels(&drawing(two)).err().unwrap();
    assert_eq!(e.params[0], ("atoms", "42".to_string()));
}

#[test]
fn an_element_off_every_site_grows_no_digraph() {
    // the second centre is drawn between two identical arms of 8 and 12 phenylenes, so it is no site; ranking its arms alone would pass
    // the work limit or the node cap. The first centre is told apart at the first sphere and keeps the label of the short analogue.
    let short = drawing(Mol::from_smiles("F[C@H](Cl)C[C@H](c1ccccc1)c1ccccc1").unwrap());
    let want = drawing_labels(&short).unwrap().atoms;
    assert_eq!(want.len(), 1);
    for n in [8, 12] {
        let arm = "c1ccc(cc1)".repeat(n);
        let m = Mol::from_smiles(&format!("F[C@H](Cl)C[C@H]({arm}){arm}")).unwrap();
        assert_eq!(m.stereo_sites(), [mzlab_mol::Site::Tetra(1)]);
        assert_eq!(
            drawing_labels(&drawing(m)).unwrap().atoms,
            want,
            "{n} rings"
        );
    }
}

#[test]
fn open_sites_cost_no_flip_test_and_big_drawings_skip_before_any() {
    for (copies, join, skip) in [
        (21, "", None),
        (40, ".", Some("tooBig")),
        (40, "", Some("tooBig")),
    ] {
        let mut m = Mol::from_smiles(&vec![OPEN; copies].join(join)).unwrap();
        m.coords = Some(m.layout_2d());
        let req = serde_json::json!({ "molfile": m.to_molfile("t") }).to_string();
        let t = std::time::Instant::now();
        let out: Value = serde_json::from_str(&cip_json(&req).unwrap()).unwrap();
        let took = t.elapsed();
        assert_eq!(out["atoms"], serde_json::json!([]));
        assert_eq!(out["skip"].as_str(), skip, "{copies} copies {join:?}");
        let most = if cfg!(debug_assertions) { 10.0 } else { 1.0 };
        assert!(
            took.as_secs_f64() < most,
            "{copies} copies {join:?} took {took:?}"
        );
    }
}

#[test]
fn the_refusal_fixture_gives_its_key_and_parameters() {
    // the same fixture is checked against the molkit `.wasm`, `mzlab-cli mol cip` (tests/test_molkit_wasm.py) and the app (labels.rs)
    let fx: Value =
        serde_json::from_str(include_str!("../../../tests/dati/cip_complex.json")).unwrap();
    let e = cip_json(&fx["request"].to_string()).unwrap_err();
    assert_eq!(e.key, fx["error_key"].as_str().unwrap());
    let params: serde_json::Map<String, Value> = e
        .params
        .iter()
        .map(|(k, v)| (k.to_string(), Value::String(v.clone())))
        .collect();
    assert_eq!(Value::Object(params), fx["params"]);
}
