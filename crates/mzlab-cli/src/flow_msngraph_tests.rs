//! `spectra.msn_graph@2` (and a saved `@1` plan, migrated): the plan gives the bytes of `mzlab-cli msn-graph` on the synthetic infusion of the route `/api/msntree`.

use std::path::{Path, PathBuf};

fn msn() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../tests/golden/ref/msntree_rotta/msn.mzML")
}

fn dir(tag: &str) -> PathBuf {
    let d = std::env::temp_dir().join(format!("mzlab_msngraph_{tag}_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).unwrap();
    std::fs::copy(msn(), d.join("msn.mzML")).unwrap();
    d
}

fn graph(d: &Path, params: &str) -> mzlab_core::Result<String> {
    let plan = format!(
        r#"{{"format":"mzflow/2","inputs":[{{"name":"msn.mzML","type":"mzml"}}],"steps":[
        {{"id":"g","step":"spectra.msn_graph@1","in":{{"source":"file:msn.mzML"}},"params":{params}}}]}}"#
    );
    let (run, prov) = crate::flow::run_plan(plan.as_bytes(), d, &d.join("work"), 1, None)?;
    assert!(
        run.skipped.is_empty() && !prov.contains("\"error\""),
        "{prov}"
    );
    Ok(std::fs::read_to_string(d.join("work/g.json")).unwrap())
}

fn cli(args: &[&str]) -> String {
    let mut a = vec!["msn-graph".to_string(), msn().to_str().unwrap().to_string()];
    a.extend(args.iter().map(|s| s.to_string()));
    let o = crate::execute(&a);
    assert_eq!(o.code, 0, "{}", o.stderr);
    String::from_utf8(o.stdout).unwrap()
}

#[test]
fn the_step_gives_the_bytes_of_the_command() {
    let d = dir("a");
    let got = graph(&d, r#"{"formula":"C14H15N3O5+"}"#).unwrap();
    assert_eq!(got, cli(&["--formula", "C14H15N3O5+"]));
    // masses only
    assert_eq!(graph(&d, "{}").unwrap(), cli(&[]));
    let _ = std::fs::remove_dir_all(&d);
}

#[test]
fn every_option_of_the_command_is_a_parameter() {
    let d = dir("b");
    let got = graph(
        &d,
        r#"{"formula":"C14H15N3O5+","smiles":"CC","ppm_formula":5,"mz_basso":100,"ppm_basso":15,"prove_min":2,"attivazione":"cid","isotopi":true,"scelte":"305.1=C14H15N3O5"}"#,
    )
    .unwrap();
    let want = cli(&[
        "--formula",
        "C14H15N3O5+",
        "--smiles",
        "CC",
        "--ppm-formula",
        "5",
        "--mz-basso",
        "100",
        "--ppm-basso",
        "15",
        "--prove-min",
        "2",
        "--attivazione",
        "cid",
        "--isotopi",
        "--scelta",
        "305.1=C14H15N3O5",
    ]);
    assert_eq!(got, want);
    let _ = std::fs::remove_dir_all(&d);
}

#[test]
fn the_cleavages_of_a_node_are_a_parameter_of_the_step() {
    let d = dir("c");
    let got = graph(
        &d,
        r#"{"formula":"C14H15N3O5+","smiles":"CCOC(=O)c1c[nH]c2ccccc12","strutture_nodo":"n3","ppm_struttura":5}"#,
    )
    .unwrap();
    let want = cli(&[
        "--formula",
        "C14H15N3O5+",
        "--smiles",
        "CCOC(=O)c1c[nH]c2ccccc12",
        "--strutture",
        "n3",
        "--ppm-struttura",
        "5",
    ]);
    assert_eq!(got, want);
    assert!(got.contains("\"strutture\":{"));
    let _ = std::fs::remove_dir_all(&d);
}

#[test]
fn the_interpretation_of_the_losses_is_a_parameter_of_the_step() {
    let d = dir("d");
    let got = graph(
        &d,
        r#"{"formula":"C14H15N3O5+","interpreta":true,"ppm_perdita":5}"#,
    )
    .unwrap();
    let want = cli(&[
        "--formula",
        "C14H15N3O5+",
        "--interpreta",
        "--ppm-perdita",
        "5",
    ]);
    assert_eq!(got, want);
    assert!(got.contains("\"interpretazione\":{\"versione\":1,\"ppm\":5,"));
    let _ = std::fs::remove_dir_all(&d);
}

#[test]
fn the_rules_are_a_parameter_of_the_step_and_without_them_the_bytes_do_not_change() {
    let d = dir("r");
    let got = graph(
        &d,
        r#"{"formula":"C14H15N3O5+","regole":true,"cutoff_q":0.3,"toll_perdita_mda":2}"#,
    )
    .unwrap();
    let want = cli(&[
        "--formula",
        "C14H15N3O5+",
        "--regole",
        "--cutoff-q",
        "0.3",
        "--toll-perdita-mda",
        "2",
    ]);
    assert_eq!(got, want);
    assert!(
        got.contains("\"regole\":{\"versione\":1,\"tabella\":\"fragrules/1\",\"cutoff_q\":0.3,")
    );
    let senza = graph(&d, r#"{"formula":"C14H15N3O5+"}"#).unwrap();
    assert_eq!(senza, cli(&["--formula", "C14H15N3O5+"]));
    assert!(!senza.contains("\"regole\""));
    let _ = std::fs::remove_dir_all(&d);
}

#[test]
fn the_structure_rules_give_the_same_bytes_in_the_step_and_in_the_command() {
    let d = dir("s");
    let got = graph(
        &d,
        r#"{"formula":"C14H15N3O5+","smiles":"NCCCC(=O)O","regole":true}"#,
    )
    .unwrap();
    let want = cli(&[
        "--formula",
        "C14H15N3O5+",
        "--smiles",
        "NCCCC(=O)O",
        "--regole",
    ]);
    assert_eq!(got, want);
    // a SMILES that does not parse leaves the rules of the formula alone: the structure is reported where it is used
    let bad = graph(
        &d,
        r#"{"formula":"C14H15N3O5+","smiles":"C(C","regole":true}"#,
    )
    .unwrap();
    assert!(bad.contains("\"regole\":{"));
    let _ = std::fs::remove_dir_all(&d);
}

/// The plan of `graph` at `@2`.
fn graph_2(d: &Path, params: &str) -> String {
    let plan = format!(
        r#"{{"format":"mzflow/2","inputs":[{{"name":"msn.mzML","type":"mzml"}}],"steps":[
        {{"id":"g","step":"spectra.msn_graph@2","in":{{"source":"file:msn.mzML"}},"params":{params}}}]}}"#
    );
    let (run, prov) = crate::flow::run_plan(plan.as_bytes(), d, &d.join("work2"), 1, None).unwrap();
    assert!(
        run.skipped.is_empty() && !prov.contains("\"error\""),
        "{prov}"
    );
    std::fs::read_to_string(d.join("work2/g.json")).unwrap()
}

#[test]
fn a_saved_plan_at_1_gives_the_bytes_of_the_plan_at_2_with_the_new_names() {
    let d = dir("v2");
    let pairs = [
        (
            r#"{"formula":"C14H15N3O5+","ppm_formula":5,"mz_basso":100,"ppm_basso":15,"prove_min":2,"attivazione":"cid","isotopi":true,"scelte":"305.1=C14H15N3O5"}"#,
            r#"{"formula":"C14H15N3O5+","ppm_formula":5,"low_mz":100,"wide_ppm":15,"min_evidence":2,"activation":"cid","isotopes":true,"pick":"305.1=C14H15N3O5"}"#,
        ),
        (
            r#"{"formula":"C14H15N3O5+","interpreta":true,"ppm_perdita":5,"regole":true,"toll_perdita_mda":2}"#,
            r#"{"formula":"C14H15N3O5+","interpret":true,"loss_ppm":5,"rules":true,"loss_tol_mda":2}"#,
        ),
        (
            r#"{"formula":"C14H15N3O5+","smiles":"CCOC(=O)c1c[nH]c2ccccc12","strutture_nodo":"tutti","ppm_struttura":5}"#,
            r#"{"formula":"C14H15N3O5+","smiles":"CCOC(=O)c1c[nH]c2ccccc12","structures":"all","struct_ppm":5}"#,
        ),
        (
            r#"{"formula":"C14H15N3O5+","smiles":"CCOC(=O)c1c[nH]c2ccccc12","rotture":true,"max_legami":1}"#,
            r#"{"formula":"C14H15N3O5+","smiles":"CCOC(=O)c1c[nH]c2ccccc12","breaks":true,"max_bonds":1}"#,
        ),
    ];
    for (old, new) in pairs {
        let a = graph(&d, old).unwrap();
        assert_eq!(a, graph_2(&d, new), "{old}");
        assert!(a.starts_with("{\"formato\":\"msngrafo/2\""));
    }
    let _ = std::fs::remove_dir_all(&d);
}

#[test]
fn the_plan_written_after_a_run_names_2_and_an_old_set_name_is_read_as_the_new_one() {
    let d = dir("plan");
    let plan = d.join("plan.json");
    std::fs::write(
        &plan,
        r#"{"format":"mzflow/2","inputs":[{"name":"msn.mzML","type":"mzml"}],"steps":[
        {"id":"g","step":"spectra.msn_graph@1","in":{"source":"file:msn.mzML"},"params":{"formula":"C14H15N3O5+","prove_min":2}}]}"#,
    )
    .unwrap();
    let out = d.join("out");
    let a: Vec<String> = [
        "flow",
        "run",
        plan.to_str().unwrap(),
        "--in",
        d.to_str().unwrap(),
        "--out",
        out.to_str().unwrap(),
        "--set",
        "g.mz_basso=100",
    ]
    .iter()
    .map(|s| s.to_string())
    .collect();
    let o = crate::execute(&a);
    assert_eq!(o.code, 0, "{}", o.stderr);
    let written = std::fs::read_to_string(out.join("plan.json")).unwrap();
    assert!(written.contains("\"spectra.msn_graph@2\""), "{written}");
    assert!(
        written.contains("\"min_evidence\":2") && written.contains("\"low_mz\":100"),
        "{written}"
    );
    for old in ["@1", "prove_min", "mz_basso"] {
        assert!(!written.contains(old), "{old} in {written}");
    }
    let g = std::fs::read_to_string(out.join("g.json")).unwrap();
    assert_eq!(
        g,
        cli(&[
            "--formula",
            "C14H15N3O5+",
            "--min-evidence",
            "2",
            "--low-mz",
            "100"
        ])
    );
    let _ = std::fs::remove_dir_all(&d);
}
