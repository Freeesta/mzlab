//! `mzlab-cli sample import`: the bytes are the engine's `import`, findings are exit 1 with their lines on stderr, `--dry-run` writes nothing, the exit codes are distinct.

use mzlab_core::json::{parse, Value};

use crate::execute;

/// The call of the contract test of missing inputs: a table that is not there, then files that are not there.
pub(crate) const MISSING: &[&[&str]] = &[&[
    "sample",
    "import",
    "/nonexistent/m.csv",
    "/nonexistent/x.mzML",
]];

fn run(args: &[&str]) -> (u8, String, String) {
    let o = execute(&args.iter().map(|s| s.to_string()).collect::<Vec<_>>());
    (o.code, String::from_utf8(o.stdout).unwrap(), o.stderr)
}

/// A folder of placeholder data files (synthetic, only names and bytes matter) and a metadata table.
fn set(tag: &str, csv: &str) -> (std::path::PathBuf, String) {
    let d = std::env::temp_dir().join(format!("mzlab-cli-{}-import-{tag}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).unwrap();
    for n in ["run_a.mzML", "run_b.mzML", "run_c.mzML"] {
        std::fs::write(d.join(n), format!("synthetic {n}")).unwrap();
    }
    let meta = d.join("meta.csv");
    std::fs::write(&meta, csv).unwrap();
    (d.clone(), meta.to_string_lossy().into_owned())
}

#[test]
fn a_complete_table_prints_the_samples_and_exits_0() {
    let (d, meta) = set(
        "ok",
        "file,sample,role,group\nrun_a.mzML,S1,dark,G\nrun_b.mzML,S2,wash,G\nrun_c.mzML,S3,QC,G\n",
    );
    let (code, out, err) = run(&["sample", "import", &meta, d.to_str().unwrap()]);
    assert_eq!(code, 0, "{err}");
    assert!(out.starts_with("{\"schema\":\"samples/1\""), "{out}");
    let v = parse(out.as_bytes()).unwrap();
    let kinds: Vec<&str> = v
        .get("samples")
        .unwrap()
        .arr()
        .iter()
        .map(|s| s.get("kind").and_then(Value::str).unwrap())
        .collect();
    assert_eq!(kinds, ["dark", "blank", "qc"]);
    // every automatic choice is on stderr: the role the name proposed and the one the table gave
    assert!(
        err.contains("auto: role row 2 run_a.mzML \"sample\" -> \"dark\""),
        "{err}"
    );
    // the files were hashed: the SHA-256 is the key the roles are matched by
    assert!(v.get("samples").unwrap().arr()[0]
        .get("sha256")
        .and_then(Value::str)
        .is_some_and(|s| s.len() == 64));
}

#[test]
fn findings_are_exit_1_with_evidence_and_the_document_is_still_written() {
    let (d, meta) = set(
        "find",
        "file,role\nrun_a.mzML,blank\nrun_a.mzML,dark\nghost.mzML,std\nrun_b.mzML,perhaps\n",
    );
    let out_file = d.join("samples.json");
    let (code, out, err) = run(&[
        "sample",
        "import",
        &meta,
        d.to_str().unwrap(),
        "--out",
        out_file.to_str().unwrap(),
        "--json",
    ]);
    assert_eq!(code, 1, "{err}");
    let v = parse(out.as_bytes()).unwrap();
    let r = v.get("report").unwrap();
    assert_eq!(r.get("status").and_then(Value::str), Some("partial"));
    assert!(
        err.contains("unmatched row 4: \"ghost.mzML\" (no file)"),
        "{err}"
    );
    assert!(err.contains("unmatched file: run_c.mzML (no row)"), "{err}");
    assert!(
        err.contains("duplicate: run_a.mzML in rows 2, 3 (values differ; row 2 kept)"),
        "{err}"
    );
    assert!(
        err.contains("unknown role: row 5 \"perhaps\" for run_b.mzML"),
        "{err}"
    );
    // --out holds the bare samples/1, the same as the `samples` of the document
    let written = std::fs::read_to_string(&out_file).unwrap();
    assert!(written.starts_with("{\"schema\":\"samples/1\""));
    assert!(out.contains(written.trim_end()));
}

#[test]
fn the_bytes_are_the_engine_answer_and_a_dry_run_writes_nothing() {
    let (d, meta) = set("dry", "file,role\nrun_a.mzML,std\n");
    let (code, dry, _) = run(&["sample", "import", &meta, d.to_str().unwrap(), "--dry-run"]);
    assert_eq!(code, 1);
    let (_, json, _) = run(&["sample", "import", &meta, d.to_str().unwrap(), "--json"]);
    assert_eq!(dry, json);
    assert!(dry.starts_with("{\"report\":{\"choices\":"));
    let body = serde_json::json!({"text": "file,role\nrun_a.mzML,std\n", "files": [{"name": "run_a.mzML", "sha256": "x"}]}).to_string();
    assert!(mzlab_core::samples::route_all("import", &body).is_ok());
    assert!(!d.join("samples.json").exists());
    let (code, _, err) = run(&[
        "sample",
        "import",
        &meta,
        d.to_str().unwrap(),
        "--dry-run",
        "--out",
        d.join("x.json").to_str().unwrap(),
    ]);
    assert_eq!(code, 2, "{err}");
    assert!(!d.join("x.json").exists());
}

#[test]
fn the_exit_codes_are_distinct() {
    let (d, meta) = set("codes", "file,role\nrun_a.mzML,std\n");
    let dir = d.to_str().unwrap();
    assert_eq!(run(&["sample", "import"]).0, 2);
    assert_eq!(run(&["sample", "import", &meta]).0, 2);
    assert_eq!(run(&["sample", "import", &meta, dir, "--bogus"]).0, 2);
    assert_eq!(run(&["sample", "import", "/nonexistent/m.csv", dir]).0, 4);
    assert_eq!(
        run(&["sample", "import", &meta, "/nonexistent/x.mzML"]).0,
        4
    );
    std::fs::write(&meta, "role\nstd\n").unwrap();
    let (code, _, err) = run(&["sample", "import", &meta, dir]);
    assert_eq!(code, 3, "{err}");
    std::fs::write(&meta, "file,role\nrun_a.mzML,std\n").unwrap();
    assert_eq!(
        run(&[
            "sample",
            "import",
            &meta,
            dir,
            "--out",
            "/nonexistent/dir/s.json"
        ])
        .0,
        6
    );
}

#[test]
fn a_samples_file_is_the_base_and_help_lists_the_exit_codes() {
    let (d, meta) = set("base", "file,role\nrun_b.mzML,qc\n");
    let (_, table, _) = run(&[
        "sample",
        "table",
        d.join("run_a.mzML").to_str().unwrap(),
        d.join("run_b.mzML").to_str().unwrap(),
    ]);
    let s = d.join("base.json");
    std::fs::write(&s, &table).unwrap();
    let (code, out, err) = run(&[
        "sample",
        "import",
        &meta,
        "--samples",
        s.to_str().unwrap(),
        "--json",
    ]);
    assert_eq!(code, 1, "{err}");
    assert!(
        out.contains("\"unmatched_files\":[\"run_a.mzML\"]"),
        "{out}"
    );
    let (code, help, _) = run(&["sample", "import", "--help"]);
    assert_eq!(code, 0);
    assert!(help.contains("Exit codes: 0") && help.contains("--dry-run"));
}
