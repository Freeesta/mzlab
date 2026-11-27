//! `mzlab-cli table slots` on a synthetic table: the bytes are those of the engine function the app and the site call, samples proposed or given, the exit codes (2, 4, 5) are
//! pinned, the keys are those of the schema, and every step and command a slot names exists in the catalogs. Synthetic data only.

use std::path::PathBuf;

use mzlab_feat::store::Slice;
use mzlab_ops::run::table::core;

use crate::execute;
use crate::tests::{mzml, page_table};

fn run(args: &[&str]) -> (u8, String, String) {
    let all: Vec<String> = ["table", "slots"]
        .iter()
        .chain(args)
        .map(|s| s.to_string())
        .collect();
    let o = execute(&all);
    (o.code, String::from_utf8(o.stdout).unwrap(), o.stderr)
}

fn call(args: &[&str]) -> (u8, String) {
    let all: Vec<String> = args.iter().map(|s| s.to_string()).collect();
    let o = execute(&all);
    (o.code, String::from_utf8(o.stdout).unwrap())
}

struct Case {
    dir: PathBuf,
    table: Vec<u8>,
    samples: String,
}

fn case(tag: &str) -> Case {
    let dir = std::env::temp_dir().join(format!("mzlab_cli_slots_{tag}_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let files = vec![
        ("s_t0.mzML".to_string(), mzml(0.0)),
        ("s_t10.mzML".to_string(), mzml(10.0)),
    ];
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
fn the_slots_are_the_bytes_of_the_engine_and_name_the_table() {
    let c = case("bytes");
    let id = first_id(&c);
    let want = core::slots_json(
        &mut Slice(&c.table),
        c.table.len() as u64,
        &c.samples,
        id,
        "t.mzfeat",
    )
    .unwrap();
    assert!(want.starts_with("{\"schema\":\"feature-slots/1\""));
    let (code, out, err) = run(&[
        &p(&c, "t.mzfeat"),
        "--id",
        &id.to_string(),
        "--samples",
        &p(&c, "samples.json"),
    ]);
    assert_eq!((code, out), (0, format!("{want}\n")), "{err}");
    assert_eq!(err, "");
    let _ = std::fs::remove_dir_all(&c.dir);
}

#[test]
fn without_samples_they_are_proposed_and_noted_on_stderr() {
    let c = case("prop");
    let id = first_id(&c).to_string();
    let (code, out, err) = run(&[&p(&c, "t.mzfeat"), "--id", &id]);
    assert_eq!(code, 0, "{err}");
    assert!(err.starts_with("note.table.slots.samples:"), "{err}");
    let (_, given, _) = run(&[
        &p(&c, "t.mzfeat"),
        "--id",
        &id,
        "--samples",
        &p(&c, "samples.json"),
    ]);
    assert_eq!(out, given);
    let _ = std::fs::remove_dir_all(&c.dir);
}

#[test]
fn the_exit_codes_are_pinned() {
    let c = case("exit");
    let t = p(&c, "t.mzfeat");
    let (code, out, err) = run(&[&t, "--id", "4000000"]);
    assert_eq!(code, 5, "{err}");
    assert!(out.contains("\"error_key\":\"error.rowNotFound\""), "{out}");
    for args in [vec![t.as_str()], vec![&t, "--id", "x"], vec!["--id", "1"]] {
        assert_eq!(run(&args).0, 2, "{args:?}");
    }
    assert_eq!(run(&[&p(&c, "none.mzfeat"), "--id", "1"]).0, 4);
    let id = first_id(&c).to_string();
    assert_eq!(
        run(&[&t, "--id", &id, "--out", &p(&c, "no/such/out.json")]).0,
        6
    );
    let _ = std::fs::remove_dir_all(&c.dir);
}

#[test]
fn the_keys_are_those_of_the_schema() {
    use mzlab_core::json::{parse, Value};
    let c = case("keys");
    let id = first_id(&c).to_string();
    let (code, out, err) = run(&[
        &p(&c, "t.mzfeat"),
        "--id",
        &id,
        "--samples",
        &p(&c, "samples.json"),
    ]);
    assert_eq!(code, 0, "{err}");
    let keys = |v: &Value| v.entries().iter().map(|e| e.0.clone()).collect::<Vec<_>>();
    let doc = parse(out.as_bytes()).unwrap();
    let schema =
        parse(include_str!("../../../docs/schemi/feature_slots-1.json").as_bytes()).unwrap();
    assert_eq!(keys(&doc), keys(schema.get("properties").unwrap()));
    let item = schema
        .get("properties")
        .unwrap()
        .get("slots")
        .unwrap()
        .get("items")
        .unwrap();
    for slot in doc.get("slots").unwrap().arr() {
        assert_eq!(keys(slot), keys(item.get("properties").unwrap()));
    }
    let _ = std::fs::remove_dir_all(&c.dir);
}

#[test]
fn every_step_and_command_a_slot_names_exists() {
    let steps = call(&["flow", "steps", "--compact"]).1;
    let cmds = call(&["commands", "--json"]).1;
    for (slot, step, command) in mzlab_feat::slots::SLOTS {
        assert!(
            step.is_some() || command.is_some(),
            "{slot} names no way to fill it"
        );
        if let Some(s) = step {
            assert!(
                steps.contains(&format!("\"id\":\"{s}\"")),
                "{slot}: no Flow step {s}"
            );
        }
        if let Some(c) = command {
            assert!(
                cmds.contains(&format!("\"path\":\"{c}\"")),
                "{slot}: no command {c}"
            );
        }
    }
}

#[test]
fn the_task_words_find_the_command() {
    for q in [
        "what fills the slots of a feature",
        "feature view slots of one row",
        "which slot of a feature is empty",
    ] {
        let (code, out) = call(&["commands", "--find", q, "--json"]);
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
        assert!(top.contains(&"table slots"), "{q:?}: {top:?}");
    }
}
