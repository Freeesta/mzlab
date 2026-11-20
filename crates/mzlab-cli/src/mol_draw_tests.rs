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

#[test]
fn mol_draw_an_inchi_alone_is_usage_with_a_hint_and_too_many_atoms_is_data() {
    let o = run(&["mol", "draw", "InChI=1S/CH4/h1H4"]);
    assert_eq!(o.code, 2);
    assert!(o.stderr.contains("InChIKey"), "{}", o.stderr);
    let o = run(&["mol", "draw", &format!("smiles:{}", "C".repeat(2001))]);
    assert_eq!(o.code, 1, "{}", o.stderr);
}

#[test]
fn mol_draw_reads_are_capped_and_a_missing_file_is_a_keyed_error() {
    use super::read_capped;
    assert_eq!(read_capped(&b"abc"[..], 3).unwrap(), "abc");
    assert_eq!(
        read_capped(&b"abcd"[..], 3),
        Err(Some("more than 3 bytes".to_string()))
    );
    assert_eq!(read_capped(&[0xff_u8, 0xfe][..], 3), Err(None));
    let o = run(&["mol", "draw", "--json", "/no/such/x.mol"]);
    assert_eq!(o.code, 4);
    assert_eq!(
        o.error.as_ref().map(|e| e.key.as_str()),
        Some("err.cli.file")
    );
}

#[test]
fn mol_draw_edit_applies_a_patch_inline_or_from_a_file() {
    let patch = r#"[{"op":"atom.add","el":"C"},{"op":"bond.add","a":0,"b":6}]"#;
    let inline = run(&["mol", "draw", "c1ccccc1", "--set", &format!("edit={patch}")]);
    assert_eq!(inline.code, 0, "{}", inline.stderr);
    assert!(
        inline.stderr.contains("choice: edit_added = 6"),
        "{}",
        inline.stderr
    );
    assert_eq!(
        text(&inline).lines().nth(3).unwrap().trim_start().get(..4),
        Some("7  7")
    );
    let p = std::env::temp_dir().join(format!("mol_draw_edit_{}.json", std::process::id()));
    std::fs::write(&p, patch).unwrap();
    let from_file = run(&[
        "mol",
        "draw",
        "c1ccccc1",
        "--set",
        &format!("edit=@{}", p.to_str().unwrap()),
    ]);
    std::fs::remove_file(&p).ok();
    assert_eq!(text(&from_file), text(&inline));
    let none = run(&[
        "mol",
        "draw",
        "c1ccccc1",
        "--set",
        "edit=@/no/such/patch.json",
    ]);
    assert_eq!(none.code, 4);
    // a file over the patch limit or one that is not text is a patch that cannot be read (2), keyed, with its reason
    let big = std::env::temp_dir().join(format!("mol_draw_edit_big_{}.json", std::process::id()));
    std::fs::write(&big, " ".repeat(mzlab_feat::chem::draw::MAX_PATCH + 1)).unwrap();
    let bin = std::env::temp_dir().join(format!("mol_draw_edit_bin_{}.json", std::process::id()));
    std::fs::write(&bin, [0xff, 0xfe]).unwrap();
    for f in [&big, &bin] {
        let o = run(&[
            "mol",
            "draw",
            "CCO",
            "--set",
            &format!("edit=@{}", f.to_str().unwrap()),
        ]);
        std::fs::remove_file(f).ok();
        let e = o.error.as_ref().expect("a keyed error");
        assert_eq!((o.code, e.key.as_str()), (2, "err.mol.draw.editJson"));
        assert!(e.params.iter().any(|p| p.0 == "message" && !p.1.is_empty()));
    }
}

#[test]
fn mol_draw_edit_failure_is_a_keyed_usage_error_naming_the_operation() {
    let bad = r#"[{"op":"atom.set","id":0,"chg":0},{"op":"bond.add","a":1,"b":9}]"#;
    let o = run(&[
        "mol",
        "draw",
        "CCO",
        "--json",
        "--set",
        &format!("edit={bad}"),
    ]);
    assert_eq!(o.code, 2);
    assert!(
        o.stdout.is_empty() || !text(&o).contains("draw/1"),
        "nothing is drawn"
    );
    let e = o.error.as_ref().expect("a keyed error");
    assert_eq!(e.key, "err.mol.draw.edit");
    let param = |k: &str| e.params.iter().find(|p| p.0 == k).map(|p| p.1.as_str());
    assert_eq!(
        (param("index"), param("reason")),
        (Some("1"), Some("unknown_atom"))
    );
    assert_eq!(run(&["mol", "draw", "CCO", "--set", "edit=nope"]).code, 2);
}
