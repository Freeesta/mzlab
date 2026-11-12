//! `molid/1` against the catalogue of the site (`mzlab/web/molecole/catalogo.csv`): synthetic writings of catalogue structures (other atom
//! order, aromatic or Kekulé, explicit H, a V2000 molfile) are exact matches; enantiomers, missing stereo and labelled atoms are near matches
//! that say what differs; a protonated ion is neither. Every catalogue structure written in reverse atom order finds itself.

use mzlab_feat::chem::identity::identity_json;
use mzlab_mol::Mol;

fn catalogue() -> String {
    std::fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../mzlab/web/molecole/catalogo.csv"
    ))
    .unwrap()
}

fn ask(cat: &str, key: &str, text: &str) -> serde_json::Value {
    let req = serde_json::json!({ key: text, "catalogo": cat }).to_string();
    serde_json::from_str(&identity_json(&req).unwrap()).unwrap()
}

fn names(v: &serde_json::Value, k: &str) -> Vec<String> {
    v[k].as_array()
        .unwrap()
        .iter()
        .map(|e| e["nome"].as_str().unwrap().to_lowercase())
        .collect()
}

/// (writing, exact name, near name with its `differs`)
const CASES: &[(&str, &str, &str, &str)] = &[
    ("Oc1ccc(NC(C)=O)cc1", "acetaminophen", "", ""),
    ("CC(=O)NC1=CC=C(O)C=C1", "acetaminophen", "", ""),
    ("[H]Oc1ccc(NC(C)=O)cc1", "acetaminophen", "", ""),
    ("OC(=O)c1ccccc1OC(C)=O", "acetylsalicylic acid", "", ""),
    ("O=c1n(C)c(=O)c2c(ncn2C)n1C", "caffeine", "", ""),
    ("COc1ccc2cc([C@H](C)C(O)=O)ccc2c1", "naproxen", "", ""),
    (
        "COc1ccc2cc([C@@H](C)C(O)=O)ccc2c1",
        "",
        "naproxen",
        "stereo",
    ),
    ("COc1ccc2cc(C(C)C(O)=O)ccc2c1", "", "naproxen", "stereo"),
    ("c1ncccc1[C@@H]1CCCN1C", "nicotine", "", ""),
    ("c1ncccc1[C@H]1CCCN1C", "", "nicotine", "stereo"),
    ("Cc1ncc([N+]([O-])=O)n1CCO", "metronidazole", "", ""),
    ("OCCn1c(C)ncc1[N+](=O)[O-]", "metronidazole", "", ""),
    (
        "[O-][N+](=O)c1ccc2NC(=O)CN=C(c3ccccc3Cl)c2c1",
        "clonazepam",
        "",
        "",
    ),
    (
        "[2H]C([2H])([2H])C(=O)Nc1ccc(O)cc1",
        "",
        "acetaminophen",
        "isotopes",
    ),
    (
        "CC(=O)N[13c]1[13cH][13cH][13c](O)[13cH][13cH]1",
        "",
        "acetaminophen",
        "isotopes",
    ),
    (
        "[2H]C([2H])([2H])Oc1ccc2cc([C@@H](C)C(O)=O)ccc2c1",
        "",
        "naproxen",
        "stereo,isotopes",
    ),
    ("C[NH+]1CCC[C@H]1c1cccnc1", "", "", ""),
    ("CCCCCCCCCCCCCCCCCCCCCCCCCCCCCC", "", "", ""),
];

#[test]
fn synthetic_writings() {
    let cat = catalogue();
    for (smi, exact, near, differs) in CASES {
        let v = ask(&cat, "smiles", smi);
        assert_eq!(v["format"], "molid/1");
        let (e, n) = (names(&v, "exact"), names(&v, "near"));
        assert_eq!(
            e.contains(&exact.to_string()),
            !exact.is_empty(),
            "{smi}: exact {e:?}"
        );
        if exact.is_empty() {
            assert!(e.is_empty(), "{smi}: exact {e:?}");
        }
        if near.is_empty() {
            assert!(n.iter().all(|x| x != exact), "{smi}: near {n:?}");
        } else {
            let i = n
                .iter()
                .position(|x| x == near)
                .unwrap_or_else(|| panic!("{smi}: near {n:?}"));
            let d: Vec<&str> = v["near"][i]["differs"]
                .as_array()
                .unwrap()
                .iter()
                .map(|x| x.as_str().unwrap())
                .collect();
            assert_eq!(d.join(","), *differs, "{smi}");
        }
    }
}

/// A V2000 molfile of a SMILES with no stereo (coordinates on a line: the reader needs none without wedges).
fn molfile(smi: &str) -> String {
    let m = Mol::from_smiles(smi).unwrap();
    let mut t = format!(
        "\n  synthetic\n\n{:3}{:3}  0  0  0  0  0  0  0  0999 V2000\n",
        m.atoms.len(),
        m.bonds.len()
    );
    for (i, a) in m.atoms.iter().enumerate() {
        let chg = match a.chg {
            1 => 3,
            -1 => 5,
            _ => 0,
        };
        t += &format!(
            "{:10.4}{:10.4}{:10.4} {:<3} 0{:3}  0  0  0  0  0  0  0  0  0  0\n",
            i as f64 * 1.5,
            0.0,
            0.0,
            a.el,
            chg
        );
    }
    for b in &m.bonds {
        t += &format!("{:3}{:3}{:3}  0\n", b.a + 1, b.b + 1, b.o);
    }
    t + "M  END\n"
}

#[test]
fn molfile_of_the_canvas() {
    let cat = catalogue();
    for (smi, name) in [
        ("Cc1ncc([N+]([O-])=O)n1CCO", "metronidazole"),
        ("Oc1ccc(NC(C)=O)cc1", "acetaminophen"),
        ("OC(=O)c1ccccc1OC(C)=O", "acetylsalicylic acid"),
    ] {
        let v = ask(&cat, "molfile", &molfile(smi));
        assert_eq!(names(&v, "exact"), vec![name.to_string()], "{smi}");
    }
}

#[test]
fn every_entry_finds_itself_in_reverse_order() {
    let cat = catalogue();
    let mut seen = 0;
    for line in cat.lines().skip(1) {
        let f: Vec<&str> = line.split(';').collect();
        let Ok(m) = Mol::from_smiles(f[6]) else {
            continue;
        };
        let n = m.atoms.len() as u32;
        let rev: Vec<u32> = (0..n).map(|i| n - 1 - i).collect();
        let smi = m.renumbered(&rev).unwrap().to_smiles().unwrap();
        let v = ask(&cat, "smiles", &smi);
        let ids: Vec<&str> = v["exact"]
            .as_array()
            .unwrap()
            .iter()
            .map(|e| e["id"].as_str().unwrap())
            .collect();
        assert!(ids.contains(&f[0]), "{} {smi}: {ids:?}", f[1]);
        seen += 1;
    }
    assert!(seen > 1000, "{seen}");
}

#[test]
fn errors_have_keys() {
    let cat = catalogue();
    let req = |k: &str, t: &str| serde_json::json!({ k: t, "catalogo": cat }).to_string();
    let key = |r: String| identity_json(&r).unwrap_err().key;
    assert_eq!(key(req("smiles", "")), "error.molid.empty");
    assert_eq!(key(req("smiles", "C1CC")), "error.molid.read");
    assert_eq!(key("{".into()), "error.molid.request");
}
