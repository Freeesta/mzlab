//! `mol find`: the answer is the route `api/molecules/search` of the built-in catalogue (one list for the site, the app and the CLI), the exit codes are pinned.

use crate::execute;
use serde_json::Value;

fn run(a: &[&str]) -> crate::Outcome {
    let mut v = vec!["mol".to_string(), "find".to_string()];
    v.extend(a.iter().map(|s| s.to_string()));
    execute(&v)
}

fn names(o: &crate::Outcome) -> Vec<String> {
    let j: Value = serde_json::from_slice(&o.stdout).unwrap();
    j.as_array().unwrap().iter().map(|m| m["name"].as_str().unwrap().to_string()).collect()
}

#[test]
fn a_name_a_synonym_a_formula_and_a_mass_find_dimethomorph() {
    for q in ["dimethomorph", "dimeth", "C21H22ClNO4", "387.1237"] {
        let o = run(&[q]);
        assert_eq!(o.code, 0, "{q}");
        assert!(names(&o).iter().any(|n| n == "dimethomorph"), "{q}: {:?}", names(&o));
    }
    assert_eq!(names(&run(&["paracetamol"]))[0], "acetaminophen");
}

#[test]
fn the_answer_keys_are_stable_and_the_limit_counts() {
    let o = run(&["di", "--set", "limit=3"]);
    let j: Value = serde_json::from_slice(&o.stdout).unwrap();
    let rows = j.as_array().unwrap();
    assert_eq!(rows.len(), 3);
    let mut keys: Vec<&str> = rows[0].as_object().unwrap().keys().map(String::as_str).collect();
    keys.sort_unstable();
    assert_eq!(keys, ["adducts", "charge", "formula", "id", "mass", "name", "score", "smiles"]);
    assert!(o.stdout.ends_with(b"\n"));
}

#[test]
fn the_removed_codes_find_nothing_and_that_is_exit_0() {
    let o = run(&["SDB-006"]);
    assert_eq!(o.code, 0);
    assert_eq!(names(&o), Vec::<String>::new());
}

#[test]
fn the_exit_codes_are_pinned() {
    assert_eq!(run(&["--help"]).code, 0);
    assert_eq!(run(&[]).code, 2, "no query");
    assert_eq!(run(&["a", "b"]).code, 2, "two queries");
    assert_eq!(run(&["x", "--nope"]).code, 2);
    assert_eq!(run(&["xx", "--set", "limit=0"]).code, 2);
    assert_eq!(run(&["xx", "--set", "ppm=-1"]).code, 2);
    assert_eq!(run(&["xx", "--set", "what=1"]).code, 2);
    assert_eq!(run(&["xx", "--db", "/no/such/list.csv"]).code, 4);
    let p = std::env::temp_dir().join(format!("mol_find_{}.csv", std::process::id()));
    std::fs::write(&p, "name;molecular_formula\nOk;CH4\nBad;Xx9\n").unwrap();
    let o = run(&["xx", "--db", p.to_str().unwrap()]);
    assert_eq!(o.code, 1, "{}", o.stderr);
    std::fs::write(&p, "name;molecular_formula\nEsempina;C10H15NO\n").unwrap();
    let o = run(&["esemp", "--db", p.to_str().unwrap()]);
    assert_eq!(names(&o), ["Esempina"]);
    std::fs::remove_file(&p).ok();
}
