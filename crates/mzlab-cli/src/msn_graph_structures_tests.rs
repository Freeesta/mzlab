//! `mzlab-cli msn graph --structures`: the cleavages of a structure that give the m/z of a node, on the synthetic infusion of `/api/msntree` and a
//! public molecule (ethyl indole-3-carboxylate: its ester bond gives the acid of 162.05495, the node n3).

use std::path::PathBuf;

use crate::execute;

const SMILES: &str = "CCOC(=O)c1c[nH]c2ccccc12";

fn msn() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../tests/golden/ref/msntree_rotta/msn.mzML")
}

fn run(extra: &[&str]) -> crate::Outcome {
    let mut a = vec![
        "msn".to_string(),
        "graph".to_string(),
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
    let t = text(run(&["--smiles", SMILES, "--structures", "n3"]));
    let v = mzlab_core::json::parse(t.as_bytes()).unwrap();
    let s = v.get("structures").unwrap();
    assert_eq!(s.get("node").unwrap().str(), Some("n3"));
    let c = &s.get("candidates").unwrap().arr()[0];
    assert_eq!(c.get("formula").unwrap().str(), Some("C9H8NO2"));
    assert!(c.get("ppm").unwrap().f64().unwrap().abs() < 1.0);
    let t0 = &c.get("cuts").unwrap().arr()[0];
    let l = t0.get("bond").unwrap();
    assert_eq!(l.get("label").unwrap().str(), Some("C2-O3"));
    let atoms: Vec<f64> = l
        .get("atoms")
        .unwrap()
        .arr()
        .iter()
        .filter_map(|x| x.f64())
        .collect();
    assert_eq!(atoms, [1.0, 2.0]);
    // the charge goes with the indole side, the ethyl is the neutral rest
    assert_eq!(t0.get("side").unwrap().f64(), Some(2.0));
    assert_eq!(t0.get("rest").unwrap().arr().len(), 2);
    assert_eq!(t0.get("piece").unwrap().arr().len(), 12);
    assert_eq!(t0.get("formula").unwrap().str(), Some("C9H7NO2"));
}

#[test]
fn the_graph_without_the_key_does_not_change_and_the_answer_is_deterministic() {
    let base = text(run(&["--smiles", SMILES]));
    let with_n3 = text(run(&["--smiles", SMILES, "--structures", "n3"]));
    let (prefix, view) = base.trim_end().split_once(",\"view\":").unwrap();
    assert!(
        with_n3.starts_with(prefix),
        "the graph part keeps its bytes"
    );
    assert!(with_n3[prefix.len()..].starts_with(",\"structures\":{"));
    assert!(
        with_n3.trim_end().ends_with(&format!(",\"view\":{view}")),
        "the view stays the last key"
    );
    assert_eq!(
        with_n3,
        text(run(&["--smiles", SMILES, "--structures", "3"]))
    );
}

#[test]
fn a_node_that_no_cleavage_gives_has_no_candidates() {
    let t = text(run(&["--smiles", SMILES, "--structures", "n2"]));
    let v = mzlab_core::json::parse(t.as_bytes()).unwrap();
    let c = v.get("structures").unwrap().get("candidates").unwrap();
    assert!(c.arr().is_empty());
    // a window of 0 ppm finds nothing even for n3 (the match is 0.33 ppm away)
    let t = text(run(&[
        "--smiles",
        SMILES,
        "--structures",
        "n3",
        "--struct-ppm",
        "0.1",
    ]));
    let v = mzlab_core::json::parse(t.as_bytes()).unwrap();
    assert!(v
        .get("structures")
        .unwrap()
        .get("candidates")
        .unwrap()
        .arr()
        .is_empty());
}

#[test]
fn a_missing_structure_node_or_a_bad_smiles_is_a_keyed_error() {
    for (extra, key) in [
        (vec!["--structures", "n3"], "err.msngrafo.noSmiles"),
        (
            vec!["--smiles", SMILES, "--structures", "n99"],
            "err.msngrafo.badNode",
        ),
        (
            vec!["--smiles", SMILES, "--structures", "x"],
            "err.msngrafo.badNode",
        ),
        (
            vec!["--smiles", "C(C", "--structures", "n3"],
            "err.msngrafo.badSmiles",
        ),
    ] {
        let o = run(&extra);
        assert_ne!(o.code, 0, "{extra:?}");
        assert!(o.stderr.contains(key), "{extra:?}: {}", o.stderr);
    }
}

#[test]
fn every_node_of_level_two_or_more_with_a_cleavage_is_in_strutture_tutti() {
    let t = text(run(&["--smiles", SMILES, "--structures", "all"]));
    let v = mzlab_core::json::parse(t.as_bytes()).unwrap();
    assert!(v.get("structures").is_none());
    let nodes: Vec<&str> = v
        .get("structures_all")
        .unwrap()
        .arr()
        .iter()
        .filter_map(|s| s.get("node")?.str())
        .collect();
    assert!(nodes.contains(&"n3"), "{nodes:?}");
    assert!(!nodes.contains(&"n0"), "the root is level 1");
    // the same cleavages as asking node by node
    let one = text(run(&["--smiles", SMILES, "--structures", "n3"]));
    let uno = mzlab_core::json::parse(one.as_bytes()).unwrap();
    let in_all = v
        .get("structures_all")
        .unwrap()
        .arr()
        .iter()
        .find(|s| s.get("node").unwrap().str() == Some("n3"))
        .unwrap();
    assert_eq!(in_all, uno.get("structures").unwrap());
    assert_eq!(t, text(run(&["--smiles", SMILES, "--structures", "all"])));
}

#[test]
fn the_molfile_is_written_back_and_complete_adds_the_view_to_an_old_graph() {
    let dir = std::env::temp_dir().join(format!("mzlab_cli_molfile_{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let mol = dir.join("x.mol");
    std::fs::write(
        &mol,
        "\n  mzlab\n\n  1  0  0  0  0  0  0  0  0  0999 V2000\nM  END\n",
    )
    .unwrap();
    let with_n3 = text(run(&[
        "--smiles",
        SMILES,
        "--molfile",
        mol.to_str().unwrap(),
    ]));
    let v = mzlab_core::json::parse(with_n3.as_bytes()).unwrap();
    assert!(v
        .get("options")
        .unwrap()
        .get("molfile")
        .unwrap()
        .str()
        .unwrap()
        .contains("M  END"));
    // an old graph (no view) is completed with the bytes the engine writes
    let old = dir.join("old.json");
    let (head, _) = with_n3.trim_end().split_once(",\"view\":").unwrap();
    std::fs::write(&old, format!("{head}}}")).unwrap();
    let o = execute(&[
        "msn".to_string(),
        "graph".to_string(),
        "complete".into(),
        old.to_str().unwrap().into(),
    ]);
    assert_eq!(o.code, 0, "{}", o.stderr);
    assert_eq!(String::from_utf8(o.stdout).unwrap(), with_n3);
    // what is not an msn-graph/1 is a keyed error
    std::fs::write(&old, "{\"schema\":\"msn-lib/1\"}").unwrap();
    let o = execute(&[
        "msn".to_string(),
        "graph".to_string(),
        "complete".into(),
        old.to_str().unwrap().into(),
    ]);
    assert_eq!(o.code, 2);
    assert!(
        o.stderr.contains("badGraph") || o.stderr.contains("msngrafo"),
        "{}",
        o.stderr
    );
    let _ = std::fs::remove_dir_all(&dir);
}
