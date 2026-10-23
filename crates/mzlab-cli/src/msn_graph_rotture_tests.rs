//! `mzlab-cli msn-graph --rotture`: the bond breaks of a structure that give the ions of the graph, on the synthetic infusion of `/api/msntree` and
//! ethyl indole-3-carboxylate (the ester bond gives the acid of 162.05495, the node n3).

use std::path::PathBuf;

use crate::execute;

const SMILES: &str = "CCOC(=O)c1c[nH]c2ccccc12";

fn run(extra: &[&str]) -> String {
    let f = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../tests/golden/ref/msntree_rotta/msn.mzML");
    let mut a = vec![
        "msn-graph".to_string(),
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
    let t = run(&["--rotture"]);
    let (prefisso, vista) = base.trim_end().split_once(",\"vista\":").unwrap();
    assert!(t.starts_with(prefisso));
    assert!(t.trim_end().ends_with(&format!(",\"vista\":{vista}")));
    let v = mzlab_core::json::parse(t.as_bytes()).unwrap();
    let r = v.get("rotture").unwrap();
    assert_eq!(r.get("versione").unwrap().f64(), Some(1.0));
    let n3 = r
        .get("nodi")
        .unwrap()
        .arr()
        .iter()
        .find(|n| n.get("nodo").unwrap().str() == Some("n3"))
        .expect("n3 has a hypothesis");
    let h = &n3.get("ipotesi").unwrap().arr()[0];
    assert_eq!(h.get("formula").unwrap().str(), Some("C9H7NO2"));
    assert_eq!(h.get("h").unwrap().f64(), Some(0.0));
    assert_eq!(h.get("legami").unwrap().arr().len(), 1);
    assert!(h.get("ppm").unwrap().f64().unwrap().abs() < 1.0);
    let c = r.get("conteggi").unwrap();
    assert!(
        c.get("archi").unwrap().f64().unwrap() >= c.get("archi_spiegati").unwrap().f64().unwrap()
    );
}

#[test]
fn the_calculation_is_deterministic_and_needs_the_structure() {
    assert_eq!(run(&["--rotture"]), run(&["--rotture"]));
    // one bond only finds no more than two
    let one = run(&["--rotture", "--max-legami", "1"]);
    assert!(one.contains("\"max_legami\":1"));
    let f = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../tests/golden/ref/msntree_rotta/msn.mzML");
    let o = execute(&[
        "msn-graph".to_string(),
        f.to_str().unwrap().to_string(),
        "--formula".into(),
        "C14H15N3O5+".into(),
        "--rotture".into(),
    ]);
    assert_ne!(o.code, 0);
    assert!(o.stderr.contains("err.msngrafo.noSmiles"), "{}", o.stderr);
}
