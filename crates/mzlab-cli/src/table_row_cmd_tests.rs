//! `mzlab-cli table row` on a table of two synthetic mzML files (indexes on disk): `row/2` and `row_traces/2` are the bytes of the engine function the app and the site call,
//! the same row with the samples proposed or given, and the exit codes (2, 4, 5) are pinned. Synthetic data only.

use std::path::PathBuf;

use mzlab_core::mzidx::build_mzml_to_vec;
use mzlab_core::upload::prepare;
use mzlab_feat::store::Slice;
use mzlab_ops::run::table::core;

use crate::execute;
use crate::tests::{mzml, page_table};

fn run(args: &[String]) -> (u8, String, String) {
    let o = execute(args);
    (o.code, String::from_utf8(o.stdout).unwrap(), o.stderr)
}

fn a(args: &[&str]) -> Vec<String> {
    ["table", "row"]
        .iter()
        .chain(args)
        .map(|s| s.to_string())
        .collect()
}

struct Case {
    dir: PathBuf,
    table: Vec<u8>,
    samples: String,
}

fn case(tag: &str) -> Case {
    let dir = std::env::temp_dir().join(format!("mzlab_cli_row_{tag}_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(dir.join("idx")).unwrap();
    let files = vec![
        ("s_t0.mzML".to_string(), mzml(0.0)),
        ("s_t10.mzML".to_string(), mzml(10.0)),
    ];
    for (n, b) in &files {
        let mut src = b.clone();
        let p = prepare(&mut src, b.len() as u64, n).unwrap();
        let idx = build_mzml_to_vec(&p.table, b, 64 << 20).unwrap().0;
        std::fs::write(dir.join("idx").join(n.replace(".mzML", ".mzidx")), idx).unwrap();
    }
    let (table, inputs) = page_table(&files, "");
    std::fs::write(dir.join("t.mzfeat"), &table).unwrap();
    let list = inputs
        .iter()
        .map(|(n, s)| format!("{{\"name\":\"{n}\",\"sha256\":\"{s}\"}}"))
        .collect::<Vec<_>>()
        .join(",");
    let samples = core::samples_json(&format!("{{\"files\":[{list}]}}")).unwrap();
    std::fs::write(dir.join("samples.json"), &samples).unwrap();
    Case {
        dir,
        table,
        samples,
    }
}

fn p(c: &Case, f: &str) -> String {
    c.dir.join(f).to_string_lossy().into_owned()
}

fn first_id(c: &Case) -> u32 {
    let v = mzlab_core::json::parse(
        core::view_json(&mut Slice(&c.table), c.table.len() as u64, &c.samples, "")
            .unwrap()
            .as_bytes(),
    )
    .unwrap();
    v.get("rows").unwrap().arr()[0].f64().unwrap() as u32
}

#[test]
fn the_row_and_its_traces_are_the_bytes_of_the_engine() {
    let c = case("bytes");
    let id = first_id(&c);
    let size = c.table.len() as u64;
    let row = core::row_json(&mut Slice(&c.table), size, &c.samples, id).unwrap();
    assert!(row.starts_with("{\"schema\":\"row/2\""));
    let (code, out, err) = run(&a(&[
        &p(&c, "t.mzfeat"),
        "--id",
        &id.to_string(),
        "--samples",
        &p(&c, "samples.json"),
    ]));
    assert_eq!((code, out), (0, format!("{row}\n")), "{err}");
    assert_eq!(err, "");
    let idx = |k: usize| {
        let f = std::fs::read(
            c.dir
                .join("idx")
                .join(["s_t0", "s_t10"][k].to_string() + ".mzidx"),
        )
        .unwrap();
        mzlab_core::idxread::Index::open(f.clone(), f.len() as u64)
    };
    let traces =
        core::row_traces_json(&mut Slice(&c.table), size, &c.samples, id, 5.0, idx).unwrap();
    assert!(traces.starts_with("{\"schema\":\"row_traces/2\""));
    let args = a(&[
        &p(&c, "t.mzfeat"),
        "--id",
        &id.to_string(),
        "--samples",
        &p(&c, "samples.json"),
        "--traces",
        "--index",
        &p(&c, "idx"),
    ]);
    let (code, out, err) = run(&args);
    assert_eq!((code, out), (0, format!("{row}\n{traces}\n")), "{err}");
    let _ = std::fs::remove_dir_all(&c.dir);
}

#[test]
fn without_samples_they_are_proposed_and_noted_on_stderr() {
    let c = case("prop");
    let id = first_id(&c).to_string();
    let (code, out, err) = run(&a(&[&p(&c, "t.mzfeat"), "--id", &id]));
    assert_eq!(code, 0, "{err}");
    assert!(err.starts_with("note.table.row.samples:"), "{err}");
    let (_, given, _) = run(&a(&[
        &p(&c, "t.mzfeat"),
        "--id",
        &id,
        "--samples",
        &p(&c, "samples.json"),
    ]));
    assert_eq!(out, given);
    let _ = std::fs::remove_dir_all(&c.dir);
}

#[test]
fn the_exit_codes_are_pinned() {
    let c = case("exit");
    let t = p(&c, "t.mzfeat");
    // an id the table does not have: 5 (a selector that names nothing)
    let (code, out, err) = run(&a(&[&t, "--id", "4000000"]));
    assert_eq!(code, 5, "{err}");
    // a JSON command answers a failure with `error/1` on stdout (the contract of the JSON commands)
    assert!(out.contains("\"error_key\":\"error.rowNotFound\""), "{out}");
    assert!(err.contains("4000000"), "{err}");
    // usage: no --id, --traces without --index, --index without --traces
    for args in [
        vec![t.as_str()],
        vec![&t, "--id", "1", "--traces"],
        vec![&t, "--id", "1", "--index", "x"],
        vec![&t, "--id", "x"],
    ] {
        assert_eq!(run(&a(&args)).0, 2, "{args:?}");
    }
    // a table that is not there, an index folder with no index
    assert_eq!(run(&a(&[&p(&c, "none.mzfeat"), "--id", "1"])).0, 4);
    let id = first_id(&c).to_string();
    let empty = c.dir.join("empty");
    std::fs::create_dir_all(&empty).unwrap();
    let args = a(&[
        &t,
        "--id",
        &id,
        "--traces",
        "--index",
        &empty.to_string_lossy(),
    ]);
    assert_eq!(run(&args).0, 4);
    // --out in a folder that is not there
    assert_eq!(
        run(&a(&[&t, "--id", &id, "--out", &p(&c, "no/such/out.json")])).0,
        6
    );
    let _ = std::fs::remove_dir_all(&c.dir);
}

#[test]
fn the_keys_are_those_of_the_schemas() {
    use mzlab_core::json::{parse, Value};
    let keys = |v: &Value| v.entries().iter().map(|e| e.0.clone()).collect::<Vec<_>>();
    let props = |s: &str| keys(parse(s.as_bytes()).unwrap().get("properties").unwrap());
    let c = case("keys");
    let id = first_id(&c).to_string();
    let args = a(&[
        &p(&c, "t.mzfeat"),
        "--id",
        &id,
        "--samples",
        &p(&c, "samples.json"),
        "--traces",
        "--index",
        &p(&c, "idx"),
    ]);
    let (code, out, err) = run(&args);
    assert_eq!(code, 0, "{err}");
    let (row, traces) = out.trim_end().split_once('\n').unwrap();
    let (row, traces) = (
        parse(row.as_bytes()).unwrap(),
        parse(traces.as_bytes()).unwrap(),
    );
    assert_eq!(
        keys(&row),
        props(include_str!("../../../docs/schemi/row-2.json"))
    );
    assert_eq!(
        keys(&traces),
        props(include_str!("../../../docs/schemi/row_traces-2.json"))
    );
    let _ = std::fs::remove_dir_all(&c.dir);
}

#[test]
fn the_task_words_find_the_command() {
    for q in [
        "row of the table",
        "one feature row of the table",
        "xic of every file of one feature",
    ] {
        let (code, out, _) = run(&[
            "commands".into(),
            "--find".into(),
            q.into(),
            "--json".into(),
        ]);
        assert_eq!(code, 0);
        let v = mzlab_core::json::parse(out.as_bytes()).unwrap();
        let top: Vec<&str> = v
            .get("matches")
            .unwrap()
            .arr()
            .iter()
            .take(3)
            .filter_map(|m| m.get("name").and_then(|n| n.str()))
            .collect();
        assert!(top.contains(&"table row"), "{q:?}: {top:?}");
    }
}
