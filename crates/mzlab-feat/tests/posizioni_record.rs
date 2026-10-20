#![cfg(feature = "record")]
//! `posiz/1` from a `mol/1` record against the one from a SMILES: same JSON for the same parent, over textbook molecules and every transformation,
//! with peaks taken from the expected fragments of the first site; a Kekulé parent through the record finds the aromatic sites of its lower-case twin.

use mzlab_feat::chem::grafo::Mol;
use mzlab_feat::chem::posizioni::{
    expected_fragments, positions_json, positions_prior_json, positions_record_json, sites, Pol,
};

const PARENTS: [&str; 8] = [
    "Oc1ccccc1",
    "Cc1ccccc1",
    "COc1ccc(Cl)cc1",
    "CN(C)c1ccccc1",
    "CCN(CC)CC",
    "CSc1ccccc1",
    "Cc1ccc(O)cc1",
    "c1ccc2ccccc2c1",
];
const TRANSFORMS: [&str; 6] = ["+O", "-CH2", "+O-2H", "+Cl-H", "+Br-H", "-2H"];

/// Peaks `[[mz, 100], ..]` of the expected fragments of the first site, or `[[100, 100]]` when there are none.
fn peaks(smiles: &str, tr: &str) -> String {
    let mol = Mol::parse(smiles).unwrap();
    let Ok(all) = sites(&mol, tr) else {
        return "[[100,100]]".into();
    };
    let f = expected_fragments(&mol, &all[0], Pol::Pos);
    let v: Vec<String> = f
        .iter()
        .take(4)
        .map(|x| format!("[{:.4},50]", x.mz))
        .collect();
    if v.is_empty() {
        "[[100,100]]".into()
    } else {
        format!("[{}]", v.join(","))
    }
}

#[test]
fn record_report_equals_smiles_report() {
    let mut n = 0;
    for p in PARENTS {
        let rec = mzlab_mol::Mol::from_smiles(p).unwrap();
        for tr in TRANSFORMS {
            let pk = peaks(p, tr);
            for (tol, ppm) in [(0.5, false), (10.0, true)] {
                let a = positions_json(p, tr, &pk, tol, ppm, true);
                let b = positions_record_json(&rec, p, tr, &pk, tol, ppm, true, "");
                match (a, b) {
                    (Ok(a), Ok(b)) => {
                        assert_eq!(a, b, "{p} {tr}");
                        n += 1;
                    }
                    (Err(a), Err(b)) => assert_eq!(a.key, b.key, "{p} {tr}"),
                    (a, b) => panic!("{p} {tr}: {a:?} vs {b:?}"),
                }
            }
        }
    }
    assert!(n > 40, "{n} reports compared");
}

#[test]
fn prior_error_is_the_same() {
    let p = "Oc1ccccc1";
    let rec = mzlab_mol::Mol::from_smiles(p).unwrap();
    let a = positions_prior_json(p, "+O", "[[77,10]]", 0.5, false, true, "{}").unwrap_err();
    let b = positions_record_json(&rec, p, "+O", "[[77,10]]", 0.5, false, true, "{}").unwrap_err();
    assert_eq!(a.key, b.key);
}

#[test]
fn kekule_parent_finds_the_aromatic_sites() {
    let aromatic = "Oc1ccccc1";
    let kekule = mzlab_mol::Mol::from_smiles("OC1=CC=CC=C1").unwrap();
    let a = positions_json(aromatic, "+O", "[[77.04,50]]", 0.5, false, true).unwrap();
    let b = positions_record_json(
        &kekule,
        aromatic,
        "+O",
        "[[77.04,50]]",
        0.5,
        false,
        true,
        "",
    )
    .unwrap();
    assert_eq!(a, b);
}
