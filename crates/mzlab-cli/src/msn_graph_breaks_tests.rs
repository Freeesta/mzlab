//! `mzlab-cli msn graph --breaks`: the bond breaks of a structure that give the ions of the graph, on the synthetic infusion of `/api/msntree` and
//! ethyl indole-3-carboxylate (the ester bond gives the acid of 162.05495, the node n3).

use std::path::PathBuf;

use crate::execute;

const SMILES: &str = "CCOC(=O)c1c[nH]c2ccccc12";

fn run(extra: &[&str]) -> String {
    let f = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../tests/golden/ref/msntree_rotta/msn.mzML");
    let mut a = vec![
        "msn".to_string(),
        "graph".to_string(),
        f.to_str().unwrap().to_string(),
        "--formula".into(),
        "C14H15N3O5+".into(),
        "--smiles".into(),
        SMILES.into(),
    ];
    a.extend(extra.iter().map(|s| s.to_string()));
    let o = execute(&a);
    assert_eq!(o.code, 0, "{}", o.stderr);
    String::from_utf8(o.stdout).unwrap()
}

#[test]
fn the_ester_bond_is_a_hypothesis_of_the_node_and_the_key_keeps_the_view_last() {
    let base = run(&[]);
    let t = run(&["--breaks"]);
    let (prefix, view) = base.trim_end().split_once(",\"view\":").unwrap();
    assert!(t.starts_with(prefix));
    assert!(t.trim_end().ends_with(&format!(",\"view\":{view}")));
    let v = mzlab_core::json::parse(t.as_bytes()).unwrap();
    let r = v.get("breaks").unwrap();
    assert_eq!(r.get("version").unwrap().f64(), Some(1.0));
    let n3 = r
        .get("nodes")
        .unwrap()
        .arr()
        .iter()
        .find(|n| n.get("node").unwrap().str() == Some("n3"))
        .expect("n3 has a hypothesis");
    let h = &n3.get("hypotheses").unwrap().arr()[0];
    assert_eq!(h.get("formula").unwrap().str(), Some("C9H7NO2"));
    assert_eq!(h.get("h").unwrap().f64(), Some(0.0));
    assert_eq!(h.get("bonds").unwrap().arr().len(), 1);
    assert!(h.get("ppm").unwrap().f64().unwrap().abs() < 1.0);
    let c = r.get("counts").unwrap();
    assert!(
        c.get("edges").unwrap().f64().unwrap() >= c.get("explained_edges").unwrap().f64().unwrap()
    );
}

#[test]
fn the_calculation_is_deterministic_and_needs_the_structure() {
    assert_eq!(run(&["--breaks"]), run(&["--breaks"]));
    // one bond only finds no more than two
    let one = run(&["--breaks", "--max-bonds", "1"]);
    assert!(one.contains("\"max_bonds\":1"));
    let f = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../tests/golden/ref/msntree_rotta/msn.mzML");
    let o = execute(&[
        "msn".to_string(),
        "graph".to_string(),
        f.to_str().unwrap().to_string(),
        "--formula".into(),
        "C14H15N3O5+".into(),
        "--breaks".into(),
    ]);
    assert_ne!(o.code, 0);
    assert!(o.stderr.contains("err.msngrafo.noSmiles"), "{}", o.stderr);
}
