//! `mzlab-cli mol draw`: the command prints the bytes of `chem::draw`, names and keys need `--set db`, and each failure has its exit code.

use crate::execute;

fn run(a: &[&str]) -> crate::Outcome {
    execute(&a.iter().map(|s| s.to_string()).collect::<Vec<_>>())
}

fn text(o: &crate::Outcome) -> String {
    String::from_utf8(o.stdout.clone()).unwrap()
}

fn catalog() -> String {
    format!(
        "{}/../../mzlab/web/molecole/catalogo.csv",
        env!("CARGO_MANIFEST_DIR")
    )
}

#[test]
fn mol_draw_prints_a_molfile_and_with_json_the_scene() {
    let o = run(&["mol", "draw", "smiles:C/C=C/C(=O)O"]);
    assert_eq!(o.code, 0, "{}", o.stderr);
    assert!(
        text(&o).contains("M  END") && o.stderr.contains("choice: spec_kind = smiles"),
        "{}",
        o.stderr
    );
    let j = run(&["mol", "draw", "C/C=C/C(=O)O", "--json"]);
    assert_eq!(j.code, 0);
    let t = text(&j);
    assert!(
        t.starts_with("{\"schema\":\"draw/1\",\"spec_kind\":\"smiles\"") && t.ends_with("}\n"),
        "{t}"
    );
    assert!(j.stderr.is_empty());
}

#[test]
fn mol_draw_a_name_needs_the_database_and_hints_at_it() {
    let o = run(&["mol", "draw", "name:caffeine"]);
    assert_eq!(o.code, 2);
    assert!(o.stderr.contains("--set db"), "{}", o.stderr);
    let o = run(&[
        "mol",
        "draw",
        "name:caffeine",
        "--set",
        &format!("db={}", catalog()),
    ]);
    assert_eq!(o.code, 0, "{}", o.stderr);
    assert!(text(&o).starts_with("caffeine\n") && text(&o).contains("M  END"));
    assert!(o.stderr.contains("name_hit = caffeine"), "{}", o.stderr);
}

#[test]
fn mol_draw_not_found_is_5_with_the_nearest_names() {
    let db = format!("db={}", catalog());
    let o = run(&["mol", "draw", "name:caffeinee", "--set", &db]);
    assert_eq!(o.code, 5);
    assert!(o.stderr.contains("nearest names: caffeine"), "{}", o.stderr);
    let j = run(&["mol", "draw", "name:zzzzzzzzzz", "--set", &db, "--json"]);
    assert_eq!(j.code, 5);
    assert!(
        text(&j).starts_with("{\"schema\":\"error/1\",\"code\":5"),
        "{}",
        text(&j)
    );
}

#[test]
fn mol_draw_missing_files_and_unwritable_outputs() {
    assert_eq!(run(&["mol", "draw", "/no/such/x.mol"]).code, 4);
    assert_eq!(
        run(&[
            "mol",
            "draw",
            "name:caffeine",
            "--set",
            "db=/no/such/db.csv"
        ])
        .code,
        4
    );
    assert_eq!(
        run(&["mol", "draw", "smiles:CCO", "--out", "/no/such/dir/o.mol"]).code,
        6
    );
    assert_eq!(run(&["mol", "draw", "smiles:C(("]).code, 2);
    assert_eq!(
        run(&["mol", "draw", "smiles:CCO", "--set", "layout=sideways"]).code,
        2
    );
    assert_eq!(
        run(&["mol", "draw", "smiles:CCO", "--set", "colour=red"]).code,
        2
    );
    assert_eq!(
        run(&[
            "mol",
            "draw",
            "smiles:CCO",
            "--json",
            "--set",
            "format=molfile"
        ])
        .code,
        2
    );
    assert_eq!(run(&["mol", "draw"]).code, 2);
}

#[test]
fn mol_draw_reads_a_molfile_path_and_keeps_its_coordinates() {
    let first = run(&["mol", "draw", "smiles:CCO", "--set", "name=ethanol"]);
    let p = std::env::temp_dir().join(format!("mol_draw_{}.mol", std::process::id()));
    std::fs::write(&p, text(&first)).unwrap();
    let path = p.to_str().unwrap();
    let again = run(&["mol", "draw", path]);
    let relaid = run(&["mol", "draw", path, "--set", "layout=new"]);
    std::fs::remove_file(&p).ok();
    assert_eq!(text(&again), text(&first));
    assert!(again.stderr.contains("layout = kept"), "{}", again.stderr);
    assert_eq!(text(&relaid), text(&first));
    assert!(relaid.stderr.contains("layout = new"), "{}", relaid.stderr);
}

#[test]
fn mol_draw_writes_to_out() {
    let p = std::env::temp_dir().join(format!("mol_draw_out_{}.mol", std::process::id()));
    let o = run(&["mol", "draw", "smiles:CCO", "--out", p.to_str().unwrap()]);
    let written = std::fs::read_to_string(&p).unwrap();
    std::fs::remove_file(&p).ok();
    assert_eq!(o.code, 0);
    assert!(o.stdout.is_empty() && written.contains("M  END"));
}
