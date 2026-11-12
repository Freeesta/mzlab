//! Tests of `mzlab-cli mol-search --sub` (`mol_search_cmd.rs`): the same JSON as `search_json`, files and literal SMILES, `--db` lists giving the
//! answers of `tests/dati/sub_db.json` (shared with the molkit `.wasm` and the app), usage errors exit 2.

use crate::execute;

fn run(a: &[&str]) -> crate::Outcome {
    execute(&a.iter().map(|s| s.to_string()).collect::<Vec<_>>())
}

#[test]
fn the_command_prints_the_function_json() {
    let p = std::env::temp_dir().join(format!("mol_search_{}.smi", std::process::id()));
    std::fs::write(&p, "Oc1ccc(Cl)cc1Cl").unwrap();
    let f = p.to_str().unwrap();
    let o = run(&[
        "mol-search",
        "--sub",
        "Oc1ccccc1Cl",
        f,
        "CCO",
        "--set",
        "stereo=1",
        "--set",
        "max=5",
    ]);
    std::fs::remove_file(&p).ok();
    assert_eq!(o.code, 0);
    let req =
        r#"{"query":"Oc1ccccc1Cl","targets":["Oc1ccc(Cl)cc1Cl","CCO"],"stereo":true,"max":5}"#;
    let want = mzlab_feat::chem::substructure::search_json(req).unwrap();
    let got = String::from_utf8(o.stdout).unwrap();
    assert_eq!(got, format!("{want}\n"));
    assert!(got.contains(r#""hits":[{"i":0,"n":1,"#), "{got}");
}

#[test]
fn usage_errors_exit_2() {
    for a in [
        vec!["mol-search"],
        vec!["mol-search", "C", "CC"],
        vec!["mol-search", "--sub", "C"],
        vec!["mol-search", "--sub", "--nope", "C", "CC"],
        vec!["mol-search", "--sub", "C", "CC", "--set", "colour=red"],
        vec!["mol-search", "--sub", "C", "CC", "--set", "max=0"],
        vec!["mol-search", "--sub", "C", "CC", "--set", "stereo=yes"],
        vec!["mol-search", "--sub", "C1C", "CC"],
        vec!["mol-search", "--sub", "C", "CC", "--db", "-"],
        vec!["mol-search", "--sub", "C", "--db"],
        vec!["mol-search", "--sub", "C", "--db", "/no/such/list.sdf"],
    ] {
        assert_eq!(run(&a).code, 2, "{a:?}");
    }
    assert_eq!(run(&["mol-search", "--help"]).code, 0);
}

#[test]
fn a_structure_list_gives_the_shared_fixture() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let fx: serde_json::Value = serde_json::from_str(
        &std::fs::read_to_string(root.join("tests/dati/sub_db.json")).unwrap(),
    )
    .unwrap();
    for (k, c) in fx["cases"].as_array().unwrap().iter().enumerate() {
        let r = &c["request"];
        let p = std::env::temp_dir().join(format!("mol_search_db_{}_{k}.txt", std::process::id()));
        std::fs::write(&p, r["db"].as_str().unwrap()).unwrap();
        let mut a = vec![
            "mol-search",
            "--sub",
            r["query"].as_str().unwrap(),
            "--db",
            p.to_str().unwrap(),
        ];
        if r.get("max").is_some() {
            a.extend(["--set", "max=1"]);
        }
        if r["stereo"].as_bool() == Some(true) {
            a.extend(["--set", "stereo=1"]);
        }
        let o = run(&a);
        std::fs::remove_file(&p).ok();
        assert_eq!(o.code, 0, "{}", c["name"]);
        let got = String::from_utf8(o.stdout).unwrap();
        assert_eq!(
            got.trim_end(),
            c["answer"].as_str().unwrap(),
            "{}",
            c["name"]
        );
        assert_eq!(
            mzlab_feat::chem::substructure::search_json(&r.to_string()).unwrap(),
            got.trim_end()
        );
    }
}
