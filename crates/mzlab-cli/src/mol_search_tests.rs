//! Tests of `mzlab-cli mol-search --sub` (`mol_search_cmd.rs`): the same JSON as `search_json`, files and literal SMILES, usage errors exit 2.

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
    ] {
        assert_eq!(run(&a).code, 2, "{a:?}");
    }
    assert_eq!(run(&["mol-search", "--help"]).code, 0);
}
