//! The chemistry group (`mol *`, `ion *`) meets the agent contract: every JSON answer opens with `"schema"` (never `"format"`) and the tag of its
//! entry, a failure is one `error/1`, and the exit code tells a syntax error (2) from a request chemistry cannot satisfy (1) and an unreadable file (4).

use crate::{execute, Outcome};

const MOL: &str = "\n  t\n\n  2  1  0  0  0  0  0  0  0  0999 V2000\n    0.0000    0.0000    0.0000 C   0  0  0  0  0  0  0  0  0  0  0  0\n    1.0000    0.0000    0.0000 O   0  0  0  0  0  0  0  0  0  0  0  0\n  1  2  1  0\nM  END\n";

fn run(a: &[&str]) -> Outcome {
    execute(&a.iter().map(|s| s.to_string()).collect::<Vec<_>>())
}

fn text(o: &Outcome) -> String {
    String::from_utf8(o.stdout.clone()).unwrap()
}

fn tmp(name: &str, body: &str) -> String {
    let p = std::env::temp_dir().join(format!("chem_contract_{}_{name}", std::process::id()));
    std::fs::write(&p, body).unwrap();
    p.to_str().unwrap().to_string()
}

#[test]
fn every_answer_opens_with_its_schema_and_none_has_a_format_key() {
    let label = tmp(
        "label.json",
        r#"{"structures":[[{"el":"C","h":3},{"el":"O","h":1}]],"arrows":[]}"#,
    );
    let anal = tmp("anal.json", r#"{"atoms":[{"el":"C","h":3},{"el":"Br"}]}"#);
    let mf = tmp("mf.json", &format!("{{\"molfile\":{:?}}}", MOL));
    let mol = tmp("x.mol", MOL);
    let link = tmp("link.json", r#"{"smiles":"CCO","part":false}"#);
    let db = tmp("db.smi", "CCO eth\n");
    let tables = tmp(
        "t.json",
        r#"{"losses":[{"id":"H2O","f":"H2O"}],"adducts":[],"contam":[]}"#,
    );
    let cases: Vec<(Vec<&str>, &str)> = vec![
        (
            vec!["ion", "mz", "C9H8O4", "--adduct", "[M+H]+"],
            "ion_mz/1",
        ),
        (vec!["ion", "mz", "250.8-251.5"], "ion_mz/1"),
        (vec!["ion", "mz", "288.2+16"], "ion_mz/1"),
        (vec!["ion", "pattern", "C9H8O4", "[M+H]+"], "ion_pattern/1"),
        (
            vec![
                "ion", "match", "18.01", "--tables", &tables, "--set", "hr=1",
            ],
            "mz-match/1",
        ),
        (vec!["ion", "elements", "--json"], "elements/1"),
        (vec!["mol", "label", &label], "label/1"),
        (vec!["mol", "anal", &anal], "analysis/1"),
        (vec!["mol", "check", &mf], "check/1"),
        (vec!["mol", "cip", &mf], "cip/1"),
        (vec!["mol", "hilite", &mf], "highlight/1"),
        (vec!["mol", "sel", &mf], "selection/1"),
        (vec!["mol", "link", &link], "link/1"),
        (vec!["mol", "cut", &mol, "--set", "bonds=0-1"], "cut/1"),
        (vec!["mol", "mech", &mf], "mechanism/1"),
        (
            vec!["mol", "search", "--sub", "CO", "CCO"],
            "substructure/1",
        ),
        (
            vec!["mol", "search", "--sub", "CO", "--db", &db],
            "substructure_db/1",
        ),
    ];
    for (a, schema) in cases {
        let o = run(&a);
        assert_eq!(o.code, 0, "{a:?}: {}", o.stderr);
        let t = text(&o);
        assert!(
            t.starts_with(&format!("{{\"schema\":\"{schema}\"")),
            "{a:?}: {t}"
        );
        let v: serde_json::Value = serde_json::from_str(&t).unwrap();
        assert!(v.get("format").is_none(), "{a:?}: a format key is left");
        let e = crate::cmd::find(&a.iter().map(|s| s.to_string()).collect::<Vec<_>>())
            .unwrap()
            .0;
        assert!(
            e.schema.is_empty() || e.schema == schema,
            "{a:?}: entry {} says {}",
            e.name,
            e.schema
        );
    }
}

#[test]
fn every_chemistry_entry_with_one_schema_names_it() {
    for e in
        crate::cmd::entries().filter(|e| e.name.starts_with("mol ") || e.name.starts_with("ion "))
    {
        assert!(!e.exits.is_empty(), "{}: no exit list", e.name);
        // mol export is text and mol search has two documents: the others carry one tag
        if !matches!(e.name, "mol export" | "mol search") {
            assert!(!e.schema.is_empty(), "{}: no schema", e.name);
        }
    }
}

#[test]
fn exit_codes_split_syntax_impossible_and_unreadable() {
    // syntax of a formula, adduct or SMILES in an argument: 2
    for a in [
        vec!["ion", "mz", "C9Xx"],
        vec!["ion", "mz", "C(H"],
        vec!["ion", "pattern", "C9H8O4", "[M+Z]+"],
        vec!["mol", "search", "--sub", "C(", "CCO"],
    ] {
        let o = run(&a);
        assert_eq!(o.code, 2, "{a:?}: {}", o.stderr);
    }
    // a valid request chemistry cannot satisfy: 1, one error/1 under --json
    let o = run(&["ion", "pattern", "H", "[M-H-H2O]-", "--json"]);
    assert_eq!(o.code, 1);
    assert!(text(&o).starts_with("{\"schema\":\"error/1\",\"code\":1"));
    let mol = tmp("arom.mol", MOL);
    assert_eq!(
        run(&[
            "mol",
            "cut",
            &mol,
            "--set",
            "bonds=0-1",
            "--set",
            "mode=heterolytic",
            "--set",
            "cation=1"
        ])
        .code,
        0
    );
    let o = run(&["mol", "cut", &mol, "--set", "bonds=0-5"]);
    assert_eq!(o.code, 1, "{}", o.stderr);
    // a file that is missing or is no request: 4
    assert_eq!(run(&["mol", "label", "/no/such/r.json"]).code, 4);
    let junk = tmp("junk.json", "not json");
    for c in [
        "label", "anal", "check", "cip", "hilite", "sel", "link", "mech",
    ] {
        let o = run(&["mol", c, &junk]);
        assert_eq!(o.code, 4, "mol {c}: {}", o.stderr);
        let o = run(&["mol", c, &junk, "--json"]);
        assert!(
            text(&o).starts_with("{\"schema\":\"error/1\",\"code\":4"),
            "mol {c}"
        );
    }
    // a bad value of an option is usage, not an unreadable file
    assert_eq!(run(&["mol", "export", &mol, "--set", "format=pdf"]).code, 2);
    assert_eq!(run(&["mol", "export", &mol, "--set", "ion=nope"]).code, 2);
}
