use std::path::PathBuf;

use crate::execute;

fn msn() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../tests/golden/ref/msntree_rotta/msn.mzML")
}

fn dir(tag: &str) -> PathBuf {
    let d = std::env::temp_dir().join(format!("mzlab_msnlib_{tag}_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).unwrap();
    d
}

fn run(a: &[&str]) -> crate::Outcome {
    execute(&a.iter().map(|s| s.to_string()).collect::<Vec<_>>())
}

#[test]
fn add_writes_the_entry_once_and_the_library_is_the_one_of_the_route() {
    let d = dir("add");
    let lib = d.join("lib.json");
    let (l, f) = (lib.to_str().unwrap(), msn());
    let args = [
        "msn-lib",
        "add",
        l,
        f.to_str().unwrap(),
        "--id",
        "STD-1",
        "--formula",
        "C14H15N3O5+",
    ];
    let o = run(&args);
    assert_eq!(o.code, 0, "{}", o.stderr);
    assert!(o.stderr.contains("added"), "{}", o.stderr);
    let first = std::fs::read_to_string(&lib).unwrap();
    assert!(first.starts_with("{\"formato\":\"msnlib/1\""), "{first}");
    // the graph inside is the graph the command prints
    let g = run(&["msn-graph", f.to_str().unwrap(), "--formula", "C14H15N3O5+"]);
    let g = String::from_utf8(g.stdout).unwrap();
    assert!(
        first.contains(g.trim_end()),
        "the entry holds the graph of msn-graph"
    );
    let again = run(&args);
    assert!(again.stderr.contains("already there"), "{}", again.stderr);
    assert_eq!(std::fs::read_to_string(&lib).unwrap(), first);
    let _ = std::fs::remove_dir_all(&d);
}

#[test]
fn build_reads_a_manifest_and_export_gives_msp() {
    let d = dir("build");
    std::fs::copy(msn(), d.join("msn.mzML")).unwrap();
    std::fs::write(
        d.join("manifest.tsv"),
        "id\tformula_neutra\taddotto_atteso\tfile\nSTD-1\tC14H14N3O5\t[M+H]+\tmsn.mzML\n",
    )
    .unwrap();
    let m = d.join("manifest.tsv");
    let o = run(&["msn-lib", "build", m.to_str().unwrap()]);
    assert_eq!(o.code, 0, "{}", o.stderr);
    let lib = d.join("manifest.json");
    // the same entry as `add` with the ion typed by hand
    let other = d.join("other.json");
    let f = msn();
    let a = run(&[
        "msn-lib",
        "add",
        other.to_str().unwrap(),
        f.to_str().unwrap(),
        "--id",
        "STD-1",
        "--formula",
        "C14H14N3O5 [M+H]+",
    ]);
    assert_eq!(a.code, 0, "{}", a.stderr);
    assert_eq!(std::fs::read(&lib).unwrap(), std::fs::read(&other).unwrap());
    let e = run(&["msn-lib", "export", lib.to_str().unwrap()]);
    let msp = String::from_utf8(e.stdout).unwrap();
    assert!(msp.starts_with("Name: STD-1 "), "{msp}");
    assert!(msp.contains("MS2") && !msp.contains("MS3"), "{msp}");
    assert!(
        msp.contains("Formula: C14H15N3O5") && msp.contains("Ion_mode: P"),
        "{msp}"
    );
    let all = run(&["msn-lib", "export", lib.to_str().unwrap(), "--tutti"]);
    assert!(String::from_utf8(all.stdout).unwrap().contains("MS3"));
    let _ = std::fs::remove_dir_all(&d);
}

#[test]
fn usage_errors_are_exit_2_and_help_is_printed() {
    assert_eq!(run(&["msn-lib", "add"]).code, 2);
    assert_eq!(run(&["msn-lib", "nope"]).code, 2);
    let h = run(&["msn-lib", "--help"]);
    assert!(String::from_utf8(h.stdout).unwrap().contains("msnlib/1"));
}
