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
    let (prefisso, vista) = base.trim_end().split_once(",\"vista\":").unwrap();
    assert!(con.starts_with(prefisso), "the graph part keeps its bytes");
    assert!(con[prefisso.len()..].starts_with(",\"strutture\":{"));
    assert!(
        con.trim_end().ends_with(&format!(",\"vista\":{vista}")),
        "the view stays the last key"
    );
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

#[test]
fn every_node_of_level_two_or_more_with_a_cleavage_is_in_strutture_tutti() {
    let t = text(run(&["--smiles", SMILES, "--strutture", "tutti"]));
    let v = mzlab_core::json::parse(t.as_bytes()).unwrap();
    assert!(v.get("strutture").is_none());
    let nodi: Vec<&str> = v
        .get("strutture_tutti")
        .unwrap()
        .arr()
        .iter()
        .filter_map(|s| s.get("nodo")?.str())
        .collect();
    assert!(nodi.contains(&"n3"), "{nodi:?}");
    assert!(!nodi.contains(&"n0"), "the root is level 1");
    // the same cleavages as asking node by node
    let uno = text(run(&["--smiles", SMILES, "--strutture", "n3"]));
    let uno = mzlab_core::json::parse(uno.as_bytes()).unwrap();
    let in_tutti = v
        .get("strutture_tutti")
        .unwrap()
        .arr()
        .iter()
        .find(|s| s.get("nodo").unwrap().str() == Some("n3"))
        .unwrap();
    assert_eq!(in_tutti, uno.get("strutture").unwrap());
    assert_eq!(t, text(run(&["--smiles", SMILES, "--strutture", "tutti"])));
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
    let con = text(run(&[
        "--smiles",
        SMILES,
        "--molfile",
        mol.to_str().unwrap(),
    ]));
    let v = mzlab_core::json::parse(con.as_bytes()).unwrap();
    assert!(v
        .get("opzioni")
        .unwrap()
        .get("molfile")
        .unwrap()
        .str()
        .unwrap()
        .contains("M  END"));
    // an old graph (no view) is completed with the bytes the engine writes
    let vecchio = dir.join("old.json");
    let (testa, _) = con.trim_end().split_once(",\"vista\":").unwrap();
    std::fs::write(&vecchio, format!("{testa}}}")).unwrap();
    let o = execute(&[
        "msn-graph".to_string(),
        "complete".into(),
        vecchio.to_str().unwrap().into(),
    ]);
    assert_eq!(o.code, 0, "{}", o.stderr);
    assert_eq!(String::from_utf8(o.stdout).unwrap(), con);
    // what is not an msngrafo/1 is a keyed error
    std::fs::write(&vecchio, "{\"formato\":\"msnlib/1\"}").unwrap();
    let o = execute(&[
        "msn-graph".to_string(),
        "complete".into(),
        vecchio.to_str().unwrap().into(),
    ]);
    assert_eq!(o.code, 2);
    assert!(
        o.stderr.contains("badGraph") || o.stderr.contains("msngrafo"),
        "{}",
        o.stderr
    );
    let _ = std::fs::remove_dir_all(&dir);
}
