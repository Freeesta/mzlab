//! Tests of `sub/1` and `subdb/1` (`substructure.rs`): hits and errors, and the three kinds of structure list (SMILES list, table, SDF).

use super::*;

#[test]
fn hits_and_errors() {
    let r = search_json(
        r#"{"query":"S(=O)(=O)N","targets":["CS(=O)(=O)N","CCO","C1CC","NS(N)(=O)=O"]}"#,
    )
    .unwrap();
    assert_eq!(
        r,
        r#"{"format":"sub/1","atoms":4,"stereo":false,"hits":[{"i":0,"n":1,"maps":[[1,2,3,4]]},{"i":3,"n":2,"maps":[[1,3,4,0],[1,3,4,2]]}],"errors":[{"i":2,"error":"SMILES: ring closure 1 is never closed"}]}"#
    );
    assert_eq!(search_json("{}").unwrap_err().key, "error.sub.request");
    assert_eq!(
        search_json(r#"{"query":"C1C","targets":[]}"#)
            .unwrap_err()
            .key,
        "error.sub.query"
    );
}

#[test]
fn a_structure_list_of_each_kind() {
    let smi =
        search_json(r##"{"query":"O","db":"# list\nOc1ccccc1 phenol\n\nCC\nC1C bad\n"}"##).unwrap();
    assert_eq!(
        smi,
        r#"{"format":"subdb/1","atoms":1,"stereo":false,"source":"smiles","total":3,"skipped":0,"hits":[{"i":0,"name":"phenol","id":"","structure":"Oc1ccccc1","n":1,"maps":[[0]]}],"errors":[{"i":2,"name":"bad","error":"SMILES: ring closure 1 is never closed"}]}"#
    );
    let csv = "id;name;smiles\r\nk:1;\"a;b\";CO\r\nk:2;no structure;\r\n";
    let (kind, list, skipped) = entries(csv);
    assert_eq!((kind, list.len(), skipped), ("table", 1, 1));
    assert_eq!(
        (
            list[0].name.as_str(),
            list[0].id.as_str(),
            list[0].text.as_str()
        ),
        ("a;b", "k:1", "CO")
    );
    let mol = "\n  t\n\n  1  0  0  0  0  0  0  0  0  0999 V2000\n    0.0000    0.0000    0.0000 O   0  0\nM  END\n";
    let sdf = format!("{mol}> <Name>\nwater\n\n$$$$\r\n{mol}$$$$\n");
    let (kind, list, _) = entries(&sdf);
    assert_eq!((kind, list.len()), ("sdf", 2));
    assert_eq!(
        (list[0].name.as_str(), list[1].name.as_str()),
        ("water", "")
    );
    assert!(list[1].text.starts_with("\n  t\n"), "{:?}", list[1].text);
    let req = format!(r#"{{"query":"O","db":{}}}"#, serde_json_str(&sdf));
    assert!(search_json(&req).unwrap().contains(r#""total":2"#));
    assert_eq!(
        search_json(r##"{"query":"O","db":"# only\n"}"##)
            .unwrap_err()
            .key,
        "error.sub.db"
    );
    assert_eq!(
        search_json(r#"{"query":"O","db":3}"#).unwrap_err().key,
        "error.sub.request"
    );
}

fn serde_json_str(s: &str) -> String {
    let mut o = Out::default();
    o.str(s);
    o.0
}
