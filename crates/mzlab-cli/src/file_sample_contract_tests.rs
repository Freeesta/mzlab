//! The agent contract of the file and sample commands, one error path each: exit code plus one `error/1` (key and code), a single JSON document
//! for several files, `items --no-points`, `convert --json` and `--dry-run`, the distinct key of an mzML given to `convert` or `sample list`.

use std::path::Path;

use mzlab_core::json::{parse, Value};

use crate::convert_fixtures::{folder, makers};
use crate::{execute, Outcome};

fn run(args: &[&str]) -> Outcome {
    execute(&args.iter().map(|s| s.to_string()).collect::<Vec<_>>())
}

fn text(o: &Outcome) -> String {
    String::from_utf8(o.stdout.clone()).unwrap()
}

/// `o` is exit `code` with one `error/1` of `key` on stdout (and nothing else) and the `error:` line on stderr.
fn is_error(o: &Outcome, code: u8, key: &str) -> Value {
    assert_eq!(o.code, code, "{}", o.stderr);
    let t = text(o);
    assert_eq!(t.lines().count(), 1, "{t}");
    assert!(t.starts_with("{\"schema\":\"error/1\""), "{t}");
    let v = parse(t.as_bytes()).unwrap();
    assert_eq!(v.get("code").and_then(Value::f64), Some(f64::from(code)));
    assert_eq!(v.get("error_key").and_then(Value::str), Some(key), "{t}");
    assert!(o.stderr.contains("error: "), "{}", o.stderr);
    v
}

fn mzml() -> String {
    format!(
        "{}/../../tests/data/gruppi/N2_t10_r1.mzML",
        env!("CARGO_MANIFEST_DIR")
    )
}

#[test]
fn items_no_points_drops_only_the_ms2_point_list() {
    let f = format!(
        "{}/../../tests/golden/ref/msntree_rotta/msn.mzML",
        env!("CARGO_MANIFEST_DIR")
    );
    let full = text(&run(&["file", "items", &f]));
    let slim = text(&run(&["file", "items", "--no-points", &f]));
    assert!(full.contains("\"ms2_events\":["), "{full}");
    assert!(!slim.contains("ms2_events"), "{slim}");
    assert!(slim.starts_with("{\"schema\":\"items/1\""), "{slim}");
    let keys = |t: &str| -> Vec<String> {
        let v = parse(t.as_bytes()).unwrap();
        let info = v.get("items").unwrap().at(0).unwrap().get("info").unwrap();
        info.entries().iter().map(|(k, _)| k.clone()).collect()
    };
    let (a, b) = (keys(&full), keys(&slim));
    assert_eq!(
        a.into_iter()
            .filter(|k| k != "ms2_events")
            .collect::<Vec<_>>(),
        b
    );
}

#[test]
fn validate_gives_one_document_and_a_missing_file_is_exit_4_with_error_1() {
    let o = run(&["file", "validate", "--json", &mzml()]);
    assert_eq!(o.code, 1, "{}", o.stderr);
    assert!(text(&o).starts_with("{\"schema\":\"mzml-validation/1\",\"files\":["));
    assert!(o.stderr.contains("INVALID"), "{}", o.stderr);
    let o = run(&["file", "validate", "--json", &mzml(), "/no/such/x.mzML"]);
    let v = is_error(&o, 4, "err.cli.file");
    // the file that could be read rides along as the `report`
    let files = v.get("report").and_then(|r| r.get("files")).unwrap();
    assert_eq!(files.arr().len(), 1);
}

#[test]
fn convert_json_dry_run_and_an_mzml_input() {
    let dir = folder("contract_convert");
    let src = makers(&dir, false);
    let out = dir.join("out");
    let (s, o) = (src.to_str().unwrap(), out.to_str().unwrap());
    let d = run(&["file", "convert", s, "--out", o, "--dry-run", "--json"]);
    assert_eq!(d.code, 0, "{}", d.stderr);
    let v = parse(&d.stdout).unwrap();
    assert!(text(&d).starts_with("{\"schema\":\"convert/1\",\"dry_run\":true"));
    let items = v.get("items").unwrap().arr();
    assert_eq!(items.len(), 2);
    assert!(items
        .iter()
        .all(|i| i.get("reader").and_then(Value::str).is_some()
            && i.get("input_bytes").and_then(Value::f64).unwrap() > 0.0));
    assert!(!out.exists(), "a dry run writes nothing");
    let plain = run(&["file", "convert", s, "--out", o, "--dry-run"]);
    assert_eq!((plain.code, text(&plain).lines().count()), (0, 2));
    let r = run(&["file", "convert", s, "--out", o, "--json"]);
    assert_eq!(r.code, 0, "{}", r.stderr);
    assert!(text(&r).starts_with("{\"schema\":\"convert/1\",\"dry_run\":false"));
    let v = parse(&r.stdout).unwrap();
    assert_eq!(v.get("items").unwrap().arr().len(), 2);
    assert!(v
        .get("items")
        .unwrap()
        .at(0)
        .unwrap()
        .get("sha256")
        .is_some());
    // an mzML has its own key; nothing says it is among the accepted types
    let e = run(&["file", "convert", "--json", &mzml(), "--out", o]);
    is_error(&e, 4, "err.file.alreadyMzml");
    assert!(!text(&e).contains("Accepted"), "{}", text(&e));
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn file_list_and_pair_failures_are_error_1() {
    is_error(
        &run(&["file", "list", "--json", "/no/such/folder"]),
        4,
        "err.session.fileNotFound",
    );
    is_error(
        &run(&["file", "list", "--json", &mzml()]),
        4,
        "err.cli.file",
    );
    is_error(
        &run(&["file", "pair", "--json", "a.txt"]),
        2,
        "err.cli.usage",
    );
    // a missing companion is a verdict: the document stays, the summary is on stderr in both forms
    for extra in [&["--json"][..], &[]] {
        let mut a = vec!["file", "pair", "/no/such/x.wiff"];
        a.extend(extra);
        let o = run(&a);
        assert_eq!(o.code, 1);
        assert!(o.stderr.contains("companion is missing"), "{}", o.stderr);
    }
}

#[test]
fn sample_select_and_dad_name_the_selector_and_the_file() {
    // the fixture is mandatory: `wiff` panics when it cannot be built, so these checks never skip
    let (dir, w) = crate::contract_tests::wiff("file_sample_dad");
    let w = w.as_str();
    is_error(
        &run(&["dad", "delay", w, "--sample", "9", "--json"]),
        5,
        "err.sample.unknown",
    );
    is_error(
        &run(&["dad", "delay", "/no/such/x.wiff", "--json"]),
        4,
        "err.session.fileNotFound",
    );
    is_error(
        &run(&["sample", "select", w, "--sample", "9", "--json"]),
        5,
        "err.sample.unknown",
    );
    let both = run(&["sample", "list", "--json", w, w]);
    assert_eq!(text(&both).lines().count(), 1);
    assert_eq!(
        parse(&both.stdout)
            .unwrap()
            .get("files")
            .unwrap()
            .arr()
            .len(),
        2
    );
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn background_and_dda_window_missing_files_are_error_1() {
    is_error(
        &run(&["trace", "background", "--json", "/no/such/x.mzML"]),
        4,
        "err.session.fileNotFound",
    );
    is_error(
        &run(&[
            "spectrum",
            "dda-window",
            "--json",
            "/no/such/x.mzML",
            "--from",
            "1",
            "--to",
            "2",
        ]),
        4,
        "err.session.fileNotFound",
    );
    let blank = run(&[
        "trace",
        "background",
        "--json",
        &mzml(),
        "--blank",
        "/no/such/b.mzML",
    ]);
    is_error(&blank, 4, "err.session.fileNotFound");
    assert!(Path::new(&mzml()).exists());
}

#[test]
fn browser_wording_has_a_terminal_variant() {
    for k in [
        "error.badIndex",
        "error.generic",
        "err.engine.indexCrash",
        "err.file.nativeStopped",
    ] {
        let e = mzlab_core::Error::new(Box::leak(k.to_string().into_boxed_str()));
        let t = crate::error_text::human_error(&e);
        assert!(
            !t.to_lowercase().contains("browser") && !t.contains("reload"),
            "{k}: {t}"
        );
    }
}

#[test]
fn items_json_flag_is_a_no_op_and_no_points_refuses_a_bad_info() {
    let f = format!(
        "{}/../../tests/golden/ref/msntree_rotta/msn.mzML",
        env!("CARGO_MANIFEST_DIR")
    );
    let plain = text(&run(&["file", "items", &f]));
    assert_eq!(text(&run(&["file", "items", "--json", &f])), plain);
    // an info that is not a JSON object cannot lose its points silently
    let bad = [mzlab_core::lr::OpenItem {
        info: "not json".into(),
        hr: false,
    }];
    let e = crate::items_cmd::document(&bad, true).unwrap_err();
    assert_eq!(e.key, "err.file.index");
    assert!(crate::items_cmd::document(&bad, false).is_ok());
}

/// The explicit `.exits()` list of each probed entry is the set of codes seen.
fn assert_listed(seen: &std::collections::BTreeMap<&str, std::collections::BTreeSet<u8>>) {
    for (name, codes) in seen {
        let words: Vec<String> = name.split(' ').map(String::from).collect();
        let (e, _) = crate::cmd::find(&words).unwrap();
        let listed: std::collections::BTreeSet<u8> = e.exits.iter().copied().collect();
        assert!(!listed.is_empty(), "{name} lists no codes");
        assert_eq!(
            &listed, codes,
            "{name}: listed {listed:?}, produced {codes:?}"
        );
    }
}

/// `sample list` on a pair whose `.scan` is still being written (started right here, so it is still growing when probed) is exit 7.
fn probe_growing(
    wiff: &str,
    dir: &Path,
    seen: &mut std::collections::BTreeMap<&str, std::collections::BTreeSet<u8>>,
) {
    let (live, writer) =
        crate::manual_table_cmd::guard_tests::growing_pair(Path::new(wiff), &dir.join("live"));
    let r = run(&["sample", "list", &live]);
    assert_eq!(r.code, 7, "{}", r.stderr);
    seen.entry("sample list").or_default().insert(r.code);
    writer.join().unwrap();
}

/// Every explicit `.exits()` list of the file and sample entries is exactly the set of codes the command produces: each code is probed, none is left unlisted.
#[test]
fn the_explicit_exit_lists_are_the_codes_the_commands_produce() {
    use crate::convert_fixtures::{folder, makers, mzxml};
    use std::collections::{BTreeMap, BTreeSet};
    let dir = folder("contract_exits");
    let src = makers(&dir, false);
    let (s, out) = (src.to_str().unwrap(), dir.join("out"));
    let cut = dir.join("cut");
    std::fs::create_dir_all(&cut).unwrap();
    let opz = mzxml::Opz {
        zlib: false,
        ..mzxml::FLAT
    };
    let whole = mzxml::mzxml(&mzxml::run(), opz);
    std::fs::write(cut.join("t.mzXML"), &whole[..whole.len() / 2]).unwrap();
    let junk = dir.join("junk.txt");
    std::fs::write(&junk, b"not a data file").unwrap();
    let (_wdir, w) = crate::contract_tests::wiff("file_sample_exits");
    let (o, j, c) = (
        out.to_str().unwrap(),
        junk.to_str().unwrap(),
        cut.to_str().unwrap(),
    );
    let (m, o3) = (mzml(), format!("{o}3"));
    let nodir = "/no/such/folder/x.json";
    // converted first: the valid mzML of the `validate` probe
    let probes: Vec<(&str, Vec<&str>, u8)> = vec![
        ("file convert", vec![s, "--out", o], 0),
        ("file convert", vec![j, "--out", o], 1),
        ("file convert", vec![], 2),
        ("file convert", vec![c, "--out", &o3], 3),
        ("file convert", vec!["/no/such", "--out", o], 4),
        ("file convert", vec![&w, "--sample", "9", "--out", o], 5),
        ("file items", vec![&m], 0),
        ("file items", vec![], 2),
        ("file items", vec!["/no/such"], 4),
        ("file items", vec![&m, "--out", nodir], 6),
        ("file pair", vec![&w], 0),
        ("file pair", vec!["/no/such/x.wiff"], 1),
        ("file pair", vec![], 2),
        ("file validate", vec![&m], 1),
        ("file validate", vec![], 2),
        ("file validate", vec!["/no/such.mzML"], 4),
        ("sample list", vec![&w], 0),
        ("sample list", vec![j], 1),
        ("sample list", vec![], 2),
        ("sample list", vec!["/no/such.wiff"], 4),
        ("sample select", vec![&w, "--sample", "1"], 0),
        ("sample select", vec![], 2),
        ("sample select", vec!["/no/such.wiff", "--sample", "1"], 4),
        ("sample select", vec![&w, "--sample", "9"], 5),
        ("sample select", vec![&w, "--sample", "1", "--ranges"], 0),
        ("sample select", vec![&w, "--ranges"], 2),
        ("sample select", vec![&w, "--sample", "9", "--ranges"], 5),
    ];
    let mut seen: BTreeMap<&str, BTreeSet<u8>> = BTreeMap::new();
    for (name, rest, code) in &probes {
        let mut a: Vec<&str> = name.split(' ').collect();
        a.extend(rest);
        let r = run(&a);
        assert_eq!(r.code, *code, "{a:?}: {}", r.stderr);
        seen.entry(name).or_default().insert(r.code);
    }
    probe_growing(&w, &dir, &mut seen);
    // a valid mzML (the one `convert` wrote) is exit 0 of `validate`
    let written = std::fs::read_dir(&out)
        .unwrap()
        .filter_map(|e| e.ok())
        .find(|e| e.path().extension().is_some_and(|x| x == "mzML"));
    let v = run(&[
        "file",
        "validate",
        written.unwrap().path().to_str().unwrap(),
    ]);
    assert_eq!(v.code, 0, "{}", v.stderr);
    seen.entry("file validate").or_default().insert(0);
    assert_listed(&seen);
    let _ = std::fs::remove_dir_all(&dir);
}
