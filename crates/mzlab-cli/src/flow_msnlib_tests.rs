//! `msn.library_export@1` and `msn.table_export@1` after `spectra.msn_graph@1` on the synthetic infusion of the route `/api/msntree`: the library has the bytes of
//! `msn-lib add` + `msn-lib export --tutti --annota`, and the tables read like the graph.

use std::path::{Path, PathBuf};

const ION: &str = "C14H15N3O5+";

fn msn() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../tests/golden/ref/msntree_rotta/msn.mzML")
}

fn dir(tag: &str) -> PathBuf {
    let d = std::env::temp_dir().join(format!("mzlab_msnlib_{tag}_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).unwrap();
    std::fs::copy(msn(), d.join("msn.mzML")).unwrap();
    d
}

/// Runs graph → `step` with `params`; returns the text of the file the second step wrote (`ext`).
fn run(d: &Path, step: &str, params: &str, ext: &str) -> String {
    let plan = format!(
        r#"{{"format":"mzflow/2","inputs":[{{"name":"msn.mzML","type":"mzml"}}],"steps":[
        {{"id":"g","step":"spectra.msn_graph@1","in":{{"source":"file:msn.mzML"}},"params":{{"formula":"{ION}"}}}},
        {{"id":"x","step":"{step}@1","in":{{"graph":"g.graph"}},"params":{params}}}]}}"#
    );
    let _ = std::fs::remove_dir_all(d.join("work"));
    let (run, prov) = crate::flow::run_plan(plan.as_bytes(), d, &d.join("work"), 1, None).unwrap();
    assert!(
        run.skipped.is_empty() && !prov.contains("\"error\""),
        "{prov}"
    );
    std::fs::read_to_string(d.join(format!("work/x.{ext}"))).unwrap()
}

fn cli(args: &[&str]) -> String {
    let a: Vec<String> = args.iter().map(|s| s.to_string()).collect();
    let o = crate::execute(&a);
    assert_eq!(o.code, 0, "{}", o.stderr);
    String::from_utf8(o.stdout).unwrap()
}

/// The text `msn-lib export` writes for the same infusion, entry `id`.
fn command(d: &Path, flags: &[&str]) -> String {
    let (lib, file) = (d.join("lib.json"), msn());
    let (lib, file) = (lib.to_str().unwrap(), file.to_str().unwrap());
    let added = cli(&["msn-lib", "add", lib, file, "--id", "STD", "--formula", ION]);
    assert!(added.is_empty());
    let mut a = vec!["msn-lib", "export", lib];
    a.extend(flags);
    cli(&a)
}

#[test]
fn the_library_has_the_bytes_of_the_command() {
    let d = dir("a");
    let got = run(&d, "msn.library_export", r#"{"id":"STD"}"#, "msp");
    assert_eq!(got, command(&d, &["--tutti", "--annota"]));
    // every MS level, the sub-formula and the loss of the explained peaks
    assert!(got.contains("MS2") && got.contains("MS3"));
    assert!(got.contains("\"C12H15N3O; loss C2O4 (87.98002)\""));
    let mb = run(
        &d,
        "msn.library_export",
        r#"{"id":"STD","format":"massbank"}"#,
        "txt",
    );
    assert_eq!(
        mb,
        command(&d, &["--tutti", "--annota", "--formato", "massbank"])
    );
    assert!(mb.contains("PK$ANNOTATION") && mb.contains("neutral loss C2O4 87.98002"));
    let _ = std::fs::remove_dir_all(&d);
}

#[test]
fn the_options_pick_levels_peaks_and_notes() {
    let d = dir("b");
    let all = run(&d, "msn.library_export", r#"{"id":"STD"}"#, "msp");
    let ms2 = run(
        &d,
        "msn.library_export",
        r#"{"id":"STD","levels":"ms2"}"#,
        "msp",
    );
    assert_eq!(ms2, command(&d, &["--annota"]));
    assert!(!ms2.contains("MS3") && ms2.len() < all.len());
    let plain = run(
        &d,
        "msn.library_export",
        r#"{"id":"STD","annotate":false}"#,
        "msp",
    );
    assert_eq!(plain, command(&d, &["--tutti"]));
    assert!(!plain.contains('"'));
    let clean = run(
        &d,
        "msn.library_export",
        r#"{"id":"STD","clean":true}"#,
        "msp",
    );
    assert_eq!(clean, command(&d, &["--tutti", "--annota", "--pulito"]));
    assert!(clean.len() < all.len());
    // without an id the records are named after the ion of the root
    assert!(run(&d, "msn.library_export", "{}", "msp").starts_with("Name: C14H15N3O5 "));
    let _ = std::fs::remove_dir_all(&d);
}

#[test]
fn the_tables_read_like_the_graph() {
    let d = dir("c");
    let graph = cli(&["msn-graph", msn().to_str().unwrap(), "--formula", ION]);
    let g = mzlab_core::json::parse(graph.as_bytes()).unwrap();
    let count = |k: &str| g.get(k).unwrap().arr().len();
    let rows = |t: &str| t.lines().count() - 1;

    let nodes = run(&d, "msn.table_export", r#"{"view":"nodes"}"#, "csv");
    assert_eq!(rows(&nodes), count("nodi"));
    assert!(nodes.contains("n1,2,217.11998,0.00,1,23.40,C12H15N3O,-4.55,unica,1,false,"));

    // the default table: one row per measure of every arc, with the loss and its formula
    let mods = run(&d, "msn.table_export", "{}", "csv");
    let measures: usize = g
        .get("archi")
        .unwrap()
        .arr()
        .iter()
        .map(|a| a.get("misure").unwrap().arr().len().max(1))
        .sum();
    assert_eq!(rows(&mods), measures);
    assert!(mods.contains("n0,305.10000,C14H15N3O5,n1,217.11998,C12H15N3O,87.98002,C2O4,1.18,false,305@cid35,2,cid,35.0,14,23.40"));

    let peaks = run(&d, "msn.table_export", r#"{"view":"peaks"}"#, "csv");
    let all: usize = g
        .get("spettri")
        .unwrap()
        .arr()
        .iter()
        .map(|s| s.get("picchi").unwrap().arr().len())
        .sum();
    assert_eq!(rows(&peaks), all);
    let clean = run(
        &d,
        "msn.table_export",
        r#"{"view":"peaks","clean":true}"#,
        "csv",
    );
    assert!(rows(&clean) < all && !clean.contains(",,\n"));
    let _ = std::fs::remove_dir_all(&d);
}

#[test]
fn a_file_that_is_not_a_graph_stops_the_step() {
    let d = dir("d");
    std::fs::write(d.join("x.json"), r#"{"formato":"other"}"#).unwrap();
    let plan = r#"{"format":"mzflow/2","inputs":[{"name":"x.json","type":"report"}],"steps":[
        {"id":"x","step":"msn.table_export@1","in":{"graph":"file:x.json"}}]}"#;
    let (_, prov) = crate::flow::run_plan(plan.as_bytes(), &d, &d.join("work"), 1, None).unwrap();
    assert!(prov.contains("err.msnlib.bad"), "{prov}");
    let _ = std::fs::remove_dir_all(&d);
}
