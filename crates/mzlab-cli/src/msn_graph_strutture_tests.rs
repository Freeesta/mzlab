//! `mzlab-cli msn-graph --strutture`: the cleavages of a structure that give the m/z of a node, on the synthetic infusion of `/api/msntree` and a
//! public molecule (ethyl indole-3-carboxylate: its ester bond gives the acid of 162.05495, the node n3).

use std::path::PathBuf;

use crate::execute;

const SMILES: &str = "CCOC(=O)c1c[nH]c2ccccc12";

fn msn() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../tests/golden/ref/msntree_rotta/msn.mzML")
}

fn run(extra: &[&str]) -> crate::Outcome {
    let mut a = vec![
        "msn-graph".to_string(),
        msn().to_str().unwrap().to_string(),
        "--formula".into(),
        "C14H15N3O5+".into(),
    ];
    a.extend(extra.iter().map(|s| s.to_string()));
    execute(&a)
}

fn text(o: crate::Outcome) -> String {
    assert_eq!(o.code, 0, "{}", o.stderr);
    String::from_utf8(o.stdout).unwrap()
}

#[test]
fn the_cleavage_that_gives_the_node_is_found_with_its_ppm_and_atoms() {
    let t = text(run(&["--smiles", SMILES, "--strutture", "n3"]));
    let v = mzlab_core::json::parse(t.as_bytes()).unwrap();
    let s = v.get("strutture").unwrap();
    assert_eq!(s.get("nodo").unwrap().str(), Some("n3"));
    let c = &s.get("candidati").unwrap().arr()[0];
    assert_eq!(c.get("formula").unwrap().str(), Some("C9H8NO2"));
    assert!(c.get("ppm").unwrap().f64().unwrap().abs() < 1.0);
    let t0 = &c.get("tagli").unwrap().arr()[0];
    let l = t0.get("legame").unwrap();
    assert_eq!(l.get("etichetta").unwrap().str(), Some("C2-O3"));
    let atomi: Vec<f64> = l
        .get("atomi")
        .unwrap()
        .arr()
        .iter()
        .filter_map(|x| x.f64())
        .collect();
    assert_eq!(atomi, [1.0, 2.0]);
    // the charge goes with the indole side, the ethyl is the neutral rest
    assert_eq!(t0.get("lato").unwrap().f64(), Some(2.0));
    assert_eq!(t0.get("resto").unwrap().arr().len(), 2);
    assert_eq!(t0.get("pezzo").unwrap().arr().len(), 12);
    assert_eq!(t0.get("formula").unwrap().str(), Some("C9H7NO2"));
}

#[test]
fn the_graph_without_the_key_does_not_change_and_the_answer_is_deterministic() {
    let base = text(run(&["--smiles", SMILES]));
    let con = text(run(&["--smiles", SMILES, "--strutture", "n3"]));
    let prefisso = base.trim_end().strip_suffix('}').unwrap();
    assert!(con.starts_with(prefisso), "the graph part keeps its bytes");
    assert!(con[prefisso.len()..].starts_with(",\"strutture\":{"));
    assert_eq!(con, text(run(&["--smiles", SMILES, "--strutture", "3"])));
}

#[test]
fn a_node_that_no_cleavage_gives_has_no_candidates() {
    let t = text(run(&["--smiles", SMILES, "--strutture", "n2"]));
    let v = mzlab_core::json::parse(t.as_bytes()).unwrap();
    let c = v.get("strutture").unwrap().get("candidati").unwrap();
    assert!(c.arr().is_empty());
    // a window of 0 ppm finds nothing even for n3 (the match is 0.33 ppm away)
    let t = text(run(&[
        "--smiles",
        SMILES,
        "--strutture",
        "n3",
        "--ppm-struttura",
        "0.1",
    ]));
    let v = mzlab_core::json::parse(t.as_bytes()).unwrap();
    assert!(v
        .get("strutture")
        .unwrap()
        .get("candidati")
        .unwrap()
        .arr()
        .is_empty());
}

#[test]
fn a_missing_structure_node_or_a_bad_smiles_is_a_keyed_error() {
    for (extra, key) in [
        (vec!["--strutture", "n3"], "err.msngrafo.noSmiles"),
        (
            vec!["--smiles", SMILES, "--strutture", "n99"],
            "err.msngrafo.badNode",
        ),
        (
            vec!["--smiles", SMILES, "--strutture", "x"],
            "err.msngrafo.badNode",
        ),
        (
            vec!["--smiles", "C(C", "--strutture", "n3"],
            "err.msngrafo.badSmiles",
        ),
    ] {
        let o = run(&extra);
        assert_ne!(o.code, 0, "{extra:?}");
        assert!(o.stderr.contains(key), "{extra:?}: {}", o.stderr);
    }
}
