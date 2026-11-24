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
        "{}/../../mzlab/web/molecules/catalogue.csv",
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
    use super::{read_capped, NotRead};
    assert_eq!(read_capped(&b"abc"[..], 3).unwrap(), "abc");
    assert_eq!(read_capped(&b"abcd"[..], 3), Err(NotRead::TooBig(3)));
    assert_eq!(read_capped(&[0xff_u8, 0xfe][..], 3), Err(NotRead::NotText));
    let o = run(&["mol", "draw", "--json", "/no/such/x.mol"]);
    assert_eq!(o.code, 4);
    assert_eq!(
        o.error.as_ref().map(|e| e.key.as_str()),
        Some("err.cli.file")
    );
}

#[test]
fn mol_draw_format_svg_prints_the_picture_and_json_wraps_it() {
    let o = run(&["mol", "draw", "CC(=O)Oc1ccccc1C(=O)O", "--format", "svg"]);
    assert_eq!(o.code, 0, "{}", o.stderr);
    let svg = text(&o);
    assert!(svg.starts_with("<svg ") && svg.ends_with("</svg>\n"));
    assert!(
        o.stderr.contains("choice: spec_kind = smiles"),
        "{}",
        o.stderr
    );
    let j = run(&[
        "mol",
        "draw",
        "CC(=O)Oc1ccccc1C(=O)O",
        "--format",
        "svg",
        "--json",
    ]);
    assert_eq!(j.code, 0, "{}", j.stderr);
    let t = text(&j);
    assert!(
        t.starts_with("{\"schema\":\"draw-image/1\",\"spec_kind\":\"smiles\""),
        "{t}"
    );
    assert!(t.ends_with("}\n") && j.stderr.is_empty());
    // the same SVG text inside the wrapper (JSON-escaped quotes and newlines)
    let v = mzlab_core::json::parse(t.as_bytes()).unwrap();
    assert_eq!(v.get("svg").and_then(|s| s.str()).unwrap(), svg);
    // --set format= is the same as --format
    assert_eq!(
        text(&run(&[
            "mol",
            "draw",
            "CC(=O)Oc1ccccc1C(=O)O",
            "--set",
            "format=svg"
        ])),
        svg
    );
}

#[test]
fn mol_draw_png_is_the_file_on_stdout_or_in_out() {
    let o = run(&["mol", "draw", "CCO", "--format", "png"]);
    assert_eq!(o.code, 0, "{}", o.stderr);
    assert_eq!(&o.stdout[..8], &[0x89, b'P', b'N', b'G', 13, 10, 26, 10]);
    let dir = std::env::temp_dir().join(format!("mzlab_draw_png_{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let f = dir.join("a.png");
    let w = run(&[
        "mol",
        "draw",
        "CCO",
        "--format",
        "png",
        "--out",
        f.to_str().unwrap(),
    ]);
    assert_eq!(w.code, 0, "{}", w.stderr);
    assert_eq!(std::fs::read(&f).unwrap(), o.stdout);
    let j = run(&["mol", "draw", "CCO", "--format", "png", "--json"]);
    assert!(
        text(&j).contains("\"png_base64\":\"iVBORw0KGgo"),
        "{}",
        text(&j)
    );
    std::fs::remove_dir_all(dir).unwrap();
}

#[test]
fn mol_draw_picture_settings_and_their_errors() {
    let svg = |extra: &[&str]| {
        let mut a = vec!["mol", "draw", "N[C@@H](C)C(=O)O", "--format", "svg"];
        a.extend_from_slice(extra);
        run(&a)
    };
    let plain = text(&svg(&[]));
    assert!(text(&svg(&["--set", "scale=40"])).len() != plain.len());
    assert!(text(&svg(&["--set", "highlight=0,1,0-1@blue"])).contains("#a9c4f5"));
    assert!(plain.contains(">(</text>") || plain.contains("(S)") || plain.contains(">S<"));
    assert!(text(&svg(&["--set", "cip=off"])).len() < plain.len());
    for bad in ["scale=3", "cip=maybe", "highlight=0@mauve", "highlight=99"] {
        let o = svg(&["--set", bad]);
        assert_eq!(o.code, 2, "{bad}: {}", o.stderr);
        assert!(o.stderr.contains("err.mol.draw.set"), "{}", o.stderr);
    }
    assert_eq!(run(&["mol", "draw", "CCO", "--format", "gif"]).code, 2);
}

#[test]
fn mol_draw_writes_ket_and_cdxml_as_text_or_in_the_scene() {
    let spec = "C[C@H](N)C(=O)O";
    let ket = run(&["mol", "draw", spec, "--format", "ket"]);
    assert_eq!(ket.code, 0, "{}", ket.stderr);
    assert!(
        text(&ket).starts_with("{\"root\":{\"nodes\":[{\"$ref\":\"mol0\"}]}"),
        "{}",
        text(&ket)
    );
    assert!(
        ket.stderr.contains("choice: file_format = ket"),
        "{}",
        ket.stderr
    );
    let cdxml = run(&["mol", "draw", spec, "--set", "format=cdxml"]);
    assert_eq!(cdxml.code, 0, "{}", cdxml.stderr);
    assert!(text(&cdxml).starts_with("<?xml") && text(&cdxml).ends_with("</CDXML>\n"));
    for (kind, plain) in [("ket", &ket), ("cdxml", &cdxml)] {
        let j = run(&["mol", "draw", spec, "--format", kind, "--json"]);
        assert_eq!(j.code, 0, "{}", j.stderr);
        let t = text(&j);
        assert!(t.starts_with("{\"schema\":\"draw/1\""), "{t}");
        let v = mzlab_core::json::parse(t.as_bytes()).unwrap();
        assert_eq!(v.get("file_format").and_then(|x| x.str()), Some(kind));
        assert_eq!(
            v.get("file").and_then(|x| x.str()),
            Some(text(plain).as_str())
        );
        assert!(j.stderr.is_empty());
    }
}

#[test]
fn mol_draw_format_errors_are_usage_errors() {
    assert_eq!(run(&["mol", "draw", "CCO", "--format", "gif"]).code, 2);
    assert_eq!(run(&["mol", "draw", "CCO", "--format"]).code, 2);
    assert_eq!(
        run(&["mol", "draw", "CCO", "--format", "molfile", "--json"]).code,
        2
    );
}

#[test]
fn mol_draw_a_molfile_that_is_not_text_is_a_molfile_error_not_a_file_error() {
    let dir = std::env::temp_dir().join(format!("mzlab_draw_bin_{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let f = dir.join("bad.mol");
    std::fs::write(&f, b"\xff\xfe\x00 M  END\n").unwrap();
    let o = run(&["mol", "draw", f.to_str().unwrap()]);
    assert_eq!(o.code, 4);
    assert!(
        o.stderr.contains("err.mol.draw.molfile") && !o.stderr.contains("err.cli.file"),
        "{}",
        o.stderr
    );
    std::fs::remove_dir_all(dir).unwrap();
}

#[test]
fn a_non_element_atom_is_a_data_error_not_a_usage_error() {
    assert_eq!(crate::exit::code_of_key("err.mol.draw.element"), 1);
    assert_eq!(crate::exit::code_of_key("err.mol.draw.set"), 2);
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

#[test]
fn mol_draw_edit_stereo_group_gives_the_engine_bytes_and_its_inverse_undoes_it() {
    let spec = "smiles:C[C@H](O)C[C@@H](N)C[C@H](F)Cl |&1:1,o1:4,&2:7|";
    let set = r#"[{"op":"stereo.group","atoms":[1,7],"g":"or1"}]"#;
    let o = run(&[
        "mol",
        "draw",
        spec,
        "--json",
        "--set",
        &format!("edit={set}"),
    ]);
    assert_eq!(o.code, 0, "{}", o.stderr);
    let mut req = mzlab_core::json::Out::default();
    req.raw("{\"spec\":")
        .str(spec)
        .raw(",\"format\":\"scene\",\"edit\":")
        .str(set)
        .raw("}");
    let engine = mzlab_feat::chem::draw::draw_json(&req.0).unwrap();
    assert_eq!(text(&o).trim_end(), engine);
    assert!(
        engine.contains(r#""i":1,"el":"C","chg":0,"iso":0,"h":1,"stg":"or1""#),
        "{engine}"
    );
    // the inverse (each atom back to its group) on the molfile the patch made gives the starting groups again
    let edited = text(&run(&[
        "mol",
        "draw",
        spec,
        "--set",
        &format!("edit={set}"),
    ]));
    let undo = r#"[{"op":"stereo.group","atoms":[1],"g":"and1"},{"op":"stereo.group","atoms":[7],"g":"and2"}]"#;
    let p = std::env::temp_dir().join(format!("mol_draw_stg_{}.mol", std::process::id()));
    std::fs::write(&p, &edited).unwrap();
    let back = run(&[
        "mol",
        "draw",
        p.to_str().unwrap(),
        "--json",
        "--set",
        &format!("edit={undo}"),
    ]);
    std::fs::remove_file(&p).ok();
    let stg = |o: &crate::Outcome| {
        let v = mzlab_core::json::parse(text(o).as_bytes()).unwrap();
        let atoms = v.get("atoms").unwrap().arr().to_vec();
        atoms
            .iter()
            .map(|a| a.get("stg").and_then(|s| s.str()).unwrap_or("").to_string())
            .collect::<Vec<_>>()
    };
    assert_eq!(stg(&back), stg(&run(&["mol", "draw", spec, "--json"])));
    let bad = run(&[
        "mol",
        "draw",
        spec,
        "--set",
        r#"edit=[{"op":"stereo.group","atoms":[0],"g":"abs"}]"#,
    ]);
    let e = bad.error.as_ref().expect("a keyed error");
    let param = |k: &str| e.params.iter().find(|p| p.0 == k).map(|p| p.1.as_str());
    assert_eq!(
        (bad.code, param("reason"), param("index")),
        (2, Some("not_stereocentre"), Some("0"))
    );
}
