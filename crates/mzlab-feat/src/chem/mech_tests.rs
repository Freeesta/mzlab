//! Tests of `mecc/1` (`mech.rs`): the arrows of an α-cleavage and of a McLafferty rearrangement are well formed, each kind of malformed arrow and
//! of lone pairs that do not fit is named, and bad requests are errors.

use super::*;
use crate::chem::cut::tests::molfile;

fn req(mf: &str, arrows: &str, lone: &str) -> String {
    let mut o = Out::default();
    o.str(mf);
    format!(
        "{{\"molfile\":{},\"arrows\":{arrows},\"lone\":{lone}}}",
        o.0
    )
}

/// Acetone radical cation (M+•): C1 methyl, C2 carbonyl C, O3 with + and the radical, C4 methyl.
fn acetone_ion() -> String {
    molfile(
        &[
            ("C", 0.0, 0.0),
            ("C", 1.0, 0.0),
            ("O", 1.0, 1.0),
            ("C", 2.0, 0.0),
        ],
        &[(1, 2, 1), (2, 3, 2), (2, 4, 1)],
        &["M  CHG  1   3   1", "M  RAD  1   3   2"],
    )
}

#[test]
fn alpha_cleavage_is_well_formed() {
    let arrows = r#"[{"kind":"single","from":{"radical":2},"to":{"bond":[1,2]}},
        {"kind":"single","from":{"bond":[1,3]},"to":{"bond":[1,2]}},
        {"kind":"single","from":{"bond":[1,3]},"to":{"atom":3}}]"#;
    let j = mech_json(&req(&acetone_ion(), arrows, "[[2,1]]")).unwrap();
    assert_eq!(
        j,
        "{\"format\":\"mecc/1\",\"arrows\":[{\"kind\":\"single\",\"from\":\"radical\",\"to\":\"bond\",\"electrons\":1,\"ok\":true},\
{\"kind\":\"single\",\"from\":\"bond\",\"to\":\"bond\",\"electrons\":1,\"ok\":true},\
{\"kind\":\"single\",\"from\":\"bond\",\"to\":\"atom\",\"electrons\":1,\"ok\":true}],\
\"lone\":[{\"atom\":2,\"pairs\":1,\"max\":1,\"ok\":true}],\"ok\":true}"
    );
}

#[test]
fn mclafferty_with_an_explicit_gamma_h_is_well_formed() {
    // 2-pentanone M+•: C1 C2(=O3 •+) C4 C5 C6, H7 on the γ carbon C6
    let mf = molfile(
        &[
            ("C", 0.0, 0.0),
            ("C", 1.0, 0.0),
            ("O", 1.0, 1.0),
            ("C", 2.0, 0.0),
            ("C", 3.0, 0.0),
            ("C", 3.0, 1.0),
            ("H", 2.2, 1.2),
        ],
        &[
            (1, 2, 1),
            (2, 3, 2),
            (2, 4, 1),
            (4, 5, 1),
            (5, 6, 1),
            (6, 7, 1),
        ],
        &["M  CHG  1   3   1", "M  RAD  1   3   2"],
    );
    let arrows = r#"[{"kind":"single","from":{"radical":2},"to":{"atom":6}},
        {"kind":"single","from":{"bond":[5,6]},"to":{"atom":6}},
        {"kind":"single","from":{"bond":[5,6]},"to":{"bond":[4,5]}},
        {"kind":"single","from":{"bond":[3,4]},"to":{"bond":[4,5]}},
        {"kind":"single","from":{"bond":[3,4]},"to":{"bond":[1,3]}}]"#;
    let j = mech_json(&req(&mf, arrows, "[]")).unwrap();
    assert!(j.ends_with("\"lone\":[],\"ok\":true}"), "{j}");
    assert!(!j.contains("problem"), "{j}");
}

#[test]
fn each_malformed_arrow_is_named() {
    let mf = acetone_ion();
    let one = |a: &str| mech_json(&req(&mf, &format!("[{a}]"), "[]")).unwrap();
    for (a, p) in [
        (
            r#"{"kind":"pair","from":{"radical":2},"to":{"bond":[1,2]}}"#,
            "pairFromRadical",
        ),
        (
            r#"{"kind":"single","from":{"radical":0},"to":{"atom":1}}"#,
            "noRadical",
        ),
        (
            r#"{"kind":"pair","from":{"lone":0},"to":{"atom":1}}"#,
            "noLonePair",
        ),
        (
            r#"{"kind":"pair","from":{"bond":[0,3]},"to":{"atom":1}}"#,
            "noBond",
        ),
        (
            r#"{"kind":"pair","from":{"bond":[0,1]},"to":{"bond":[0,2]}}"#,
            "noBond",
        ),
        (
            r#"{"kind":"pair","from":{"lone":2},"to":{"atom":2}}"#,
            "same",
        ),
        (
            r#"{"kind":"pair","from":{"bond":[0,1]},"to":{"bond":[1,0]}}"#,
            "same",
        ),
    ] {
        let j = one(a);
        assert!(
            j.contains(&format!("\"ok\":false,\"problem\":\"{p}\""))
                && j.ends_with("\"ok\":false}"),
            "{a}: {j}"
        );
    }
    // a pair from the lone pair of the carbonyl O to the C=O bond, and from a bond to an atom of it, are well formed
    assert!(
        one(r#"{"kind":"pair","from":{"lone":2},"to":{"bond":[1,2]}}"#).ends_with("\"ok\":true}")
    );
    assert!(
        one(r#"{"kind":"pair","from":{"bond":[1,2]},"to":{"atom":2}}"#).ends_with("\"ok\":true}")
    );
}

#[test]
fn lone_pairs_must_fit_the_atom() {
    let mf = acetone_ion();
    let j = mech_json(&req(&mf, "[]", "[[2,2],[0,1]]")).unwrap();
    assert_eq!(
        j,
        "{\"format\":\"mecc/1\",\"arrows\":[],\"lone\":[{\"atom\":2,\"pairs\":2,\"max\":1,\"ok\":false},{\"atom\":0,\"pairs\":1,\"max\":0,\"ok\":false}],\"ok\":false}"
    );
    // neutral water: two lone pairs on O
    let w = molfile(&[("O", 0.0, 0.0)], &[], &[]);
    assert!(mech_json(&req(&w, "[]", "[[0,2]]"))
        .unwrap()
        .contains("\"pairs\":2,\"max\":2,\"ok\":true"));
    // an explicit H holds no lone pair
    let oh = molfile(&[("O", 0.0, 0.0), ("H", 1.0, 0.0)], &[(1, 2, 1)], &[]);
    assert!(mech_json(&req(&oh, "[]", "[[0,2],[1,1]]"))
        .unwrap()
        .ends_with("{\"atom\":1,\"pairs\":1,\"max\":0,\"ok\":false}],\"ok\":false}"));
}

#[test]
fn bad_requests_are_errors() {
    let mf = acetone_ion();
    let key = |t: &str| mech_json(t).unwrap_err().key.to_string();
    assert_eq!(key("{"), "error.grafo.mech.request");
    for a in [
        r#"[{"kind":"both","from":{"lone":2},"to":{"atom":1}}]"#,
        r#"[{"kind":"pair","from":{"atom":2},"to":{"atom":1}}]"#,
        r#"[{"kind":"pair","from":{"lone":2},"to":{"lone":1}}]"#,
        r#"[{"kind":"pair","from":{"lone":2,"radical":2},"to":{"atom":1}}]"#,
        r#"[{"kind":"pair","from":{"lone":-1},"to":{"atom":1}}]"#,
    ] {
        assert_eq!(key(&req(&mf, a, "[]")), "error.grafo.mech.request", "{a}");
    }
    assert_eq!(
        key(&req(
            &mf,
            r#"[{"kind":"pair","from":{"lone":2},"to":{"atom":9}}]"#,
            "[]"
        )),
        "error.grafo.mech.atom"
    );
    assert_eq!(key(&req(&mf, "[]", "[[7,1]]")), "error.grafo.mech.atom");
}
