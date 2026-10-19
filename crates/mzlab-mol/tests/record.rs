//! `mol/1` against RDKit golden data (tests/data/golden.json, from tools/struttura_dato/confronto.py --golden) and the traps of
//! docs/studi/STRUTTURA_DATO.md: the record round-trips JSON byte for byte, the core hash equals the one computed in Python, and
//! "unknown", "none", racemic and pure are all different records.

use mzlab_mol::{Atom, Bond, Cfg, Mol, MolError, Parity, Status, Stereo};
use serde_json::Value;

fn golden() -> Value {
    serde_json::from_str(include_str!("data/golden.json")).expect("golden json")
}

#[test]
fn twenty_cases_roundtrip_and_hash() {
    let g = golden();
    let cases = g["cases"].as_array().expect("cases");
    assert_eq!(cases.len(), 20);
    for c in cases {
        let name = c["name"].as_str().unwrap_or_default();
        for order in ["input_order", "rdkit_canonical"] {
            let rec = &c["records"][order];
            let m =
                Mol::from_json(&rec.to_string()).unwrap_or_else(|e| panic!("{name}/{order}: {e}"));
            let back: Value = serde_json::from_str(&m.to_json()).expect("own json");
            assert_eq!(&back, rec, "{name}/{order}: JSON round trip");
            assert_eq!(m.to_json(), m.to_json(), "{name}/{order}: deterministic");
            assert_eq!(
                Some(m.core_hash_hex().as_str()),
                c["core_sha256"][order].as_str(),
                "{name}/{order}: core hash"
            );
        }
    }
}

fn case(name: &str) -> Mol {
    let g = golden();
    let c = g["cases"]
        .as_array()
        .expect("cases")
        .iter()
        .find(|c| c["name"] == name)
        .expect("case");
    Mol::from_json(&c["records"]["input_order"].to_string()).expect("valid")
}

#[test]
fn enantiomers_unknown_and_racemate_have_different_hashes() {
    let (s, r, u, rac) = (
        case("timolol_S"),
        case("timolol_R"),
        case("timolol_unspec"),
        case("timolol_rac"),
    );
    let hashes = [s.core_hash(), r.core_hash(), u.core_hash(), rac.core_hash()];
    for i in 0..4 {
        for j in i + 1..4 {
            assert_ne!(hashes[i], hashes[j], "records {i} and {j} must differ");
        }
    }
    // unknown is a stereo element with no parity, not an absent element
    assert!(matches!(
        u.stereo.as_slice(),
        [Stereo::Tetra {
            status: Status::Unknown,
            par: None,
            ..
        }]
    ));
    let mut none = u.clone();
    none.stereo.clear();
    assert_ne!(
        u.core_hash(),
        none.core_hash(),
        "unknown stereo is not no stereo"
    );
    assert!(none.validate().is_ok());
}

#[test]
fn unknown_elements_serialise_without_fields() {
    let u = case("timolol_unspec");
    assert!(u
        .core_json()
        .contains(r#"{"t":"tetra","c":6,"status":"unknown"}"#));
    let b = case("butene_unspec");
    assert!(b
        .core_json()
        .contains(r#"{"t":"db","a":1,"b":2,"status":"unknown"}"#));
}

#[test]
fn layers_outside_the_core_do_not_change_the_hash() {
    let mut m = case("ephedrine");
    let h = m.core_hash();
    let n = m.atoms.len();
    m.coords = Some(mzlab_mol::Coords {
        dim: 2,
        xy: vec![vec![0.0, 1.5]; n],
        from: "test".into(),
    });
    m.id = Some(mzlab_mol::Ids {
        can: Some("x".into()),
        ..Default::default()
    });
    m.props.push(mzlab_mol::Prop {
        name: "q".into(),
        from: "test".into(),
        can: None,
        v: vec![0.25; n],
    });
    m.regions.push(mzlab_mol::Region {
        atoms: vec![0, 1],
        delta: "+O".into(),
        kind: mzlab_mol::RegionKind::Zone,
        support: Some(0.5),
    });
    assert!(m.validate().is_ok());
    assert_eq!(m.core_hash(), h);
    let back = Mol::from_json(&m.to_json()).expect("with layers");
    assert_eq!(back, m);
    assert_eq!(back.to_json(), m.to_json());
}

fn minimal() -> Mol {
    let c = |el: &str, h| Atom {
        el: el.into(),
        iso: 0,
        chg: 0,
        rad: 0,
        h,
    };
    Mol::new(
        vec![c("C", 3), c("C", 1), c("C", 1), c("C", 3)],
        vec![
            Bond { a: 0, b: 1, o: 1 },
            Bond { a: 1, b: 2, o: 2 },
            Bond { a: 2, b: 3, o: 1 },
        ],
        vec![],
        vec![],
    )
}

fn invalid(m: &Mol, what: &str) {
    assert!(
        matches!(m.validate(), Err(MolError::Invalid(_))),
        "{what} must be rejected"
    );
}

#[test]
fn rejects_broken_records() {
    assert!(minimal().validate().is_ok());
    let mut m = minimal();
    m.atoms[0].el = "Xx".into();
    invalid(&m, "unknown element");
    let mut m = minimal();
    m.bonds[0].o = 4;
    invalid(&m, "bond order 4 (aromatic orders are not stored)");
    let mut m = minimal();
    m.bonds.push(Bond { a: 3, b: 1, o: 1 });
    invalid(&m, "bond with a > b");
    let mut m = minimal();
    m.bonds.push(Bond { a: 0, b: 1, o: 1 });
    invalid(&m, "duplicate bond");
    let mut m = minimal();
    m.stereo.push(Stereo::Db {
        a: 1,
        b: 2,
        refs: Some([0, 3]),
        cfg: None,
        status: Status::Defined,
    });
    invalid(&m, "defined double bond without cfg");
    let mut m = minimal();
    m.stereo.push(Stereo::Db {
        a: 0,
        b: 1,
        refs: None,
        cfg: None,
        status: Status::Unknown,
    });
    invalid(&m, "stereo on a single bond");
    let mut m = minimal();
    m.stereo.push(Stereo::Db {
        a: 1,
        b: 2,
        refs: Some([3, 0]),
        cfg: Some(Cfg::Cis),
        status: Status::Defined,
    });
    invalid(&m, "refs on the wrong ends");
    let mut m = minimal();
    m.stereo.push(Stereo::Tetra {
        c: 1,
        refs: Some(vec![0, 2]),
        implicit_h: Some(true),
        par: Some(Parity::Cw),
        status: Status::Defined,
    });
    invalid(&m, "two refs on a tetrahedral centre");
    let mut m = minimal();
    m.stereo.push(Stereo::Tetra {
        c: 1,
        refs: Some(vec![0]),
        implicit_h: None,
        par: None,
        status: Status::Unknown,
    });
    invalid(&m, "unknown centre carrying refs");
    let mut m = minimal();
    m.format = "mol/2".into();
    invalid(&m, "another format version");
    let mut m = minimal();
    m.groups.push(mzlab_mol::StereoGroup {
        kind: mzlab_mol::GroupKind::And,
        atoms: vec![1],
    });
    invalid(&m, "group on a non-stereo atom");
    assert!(matches!(Mol::from_json("{"), Err(MolError::Json(_))));
}

/// Moving the atoms of the written-order record by RDKit's canonical permutation gives RDKit's own canonical-order record: refs, parities,
/// cis/trans, groups and the hash all follow. Bond orders inside aromatic rings are left out (RDKit may Kekulise the two orders differently).
#[test]
fn renumbering_matches_rdkit_canonical_order() {
    let g = golden();
    for c in g["cases"].as_array().expect("cases") {
        let name = c["name"].as_str().unwrap_or_default();
        let perm: Vec<u32> = c["canonical_perm"]
            .as_array()
            .expect("perm")
            .iter()
            .map(|v| v.as_u64().unwrap_or(0) as u32)
            .collect();
        let written = Mol::from_json(&c["records"]["input_order"].to_string()).expect("written");
        let moved: Value =
            serde_json::to_value(written.renumbered(&perm).expect("permutation")).expect("json");
        let truth = &c["records"]["rdkit_canonical"];
        for key in ["atoms", "stereo", "groups"] {
            assert_eq!(moved[key], truth[key], "{name}: {key}");
        }
        let pairs = |v: &Value| -> Vec<(u64, u64)> {
            v["bonds"]
                .as_array()
                .expect("bonds")
                .iter()
                .map(|b| (b["a"].as_u64().unwrap_or(0), b["b"].as_u64().unwrap_or(0)))
                .collect()
        };
        assert_eq!(pairs(&moved), pairs(truth), "{name}: bond pairs");
        if c["aromatic"].as_array().is_some_and(|a| a.is_empty()) {
            assert_eq!(moved["bonds"], truth["bonds"], "{name}: bond orders");
            let m = written.renumbered(&perm).expect("permutation");
            assert_eq!(
                Some(m.core_hash_hex().as_str()),
                c["core_sha256"]["rdkit_canonical"].as_str(),
                "{name}: hash"
            );
        }
    }
}

#[test]
fn renumbering_rejects_a_non_permutation() {
    let m = minimal();
    assert!(m.renumbered(&[0]).is_err());
    assert!(m.renumbered(&vec![0; m.atoms.len()]).is_err());
}
