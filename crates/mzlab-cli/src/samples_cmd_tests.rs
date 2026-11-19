//! `mzlab-cli samples`: the answers are the ones of `samples::route` on the same names.

use mzlab_core::samples::route_all as route;

use crate::execute;

fn run(args: &[&str]) -> (u8, String, String) {
    let o = execute(&args.iter().map(|s| s.to_string()).collect::<Vec<_>>());
    (o.code, String::from_utf8(o.stdout).unwrap(), o.stderr)
}

fn names_file(tag: &str) -> std::path::PathBuf {
    let p = std::env::temp_dir().join(format!("mzlab-cli-{}-nomi-{tag}.txt", std::process::id()));
    std::fs::write(
        &p,
        "MQ_H2O2_T00\r\nMQ_H2O2_T05\n\nTW_H2O2_T00\nTW_H2O2_T05\n",
    )
    .unwrap();
    p
}

const BODY: &str = r#"{"files":[{"name":"MQ_H2O2_T00"},{"name":"MQ_H2O2_T05"},{"name":"TW_H2O2_T00"},{"name":"TW_H2O2_T05"}]"#;

#[test]
fn variables_is_the_route_answer() {
    let f = names_file("a");
    let (code, out, err) = run(&["samples", "vars", "--names", f.to_str().unwrap()]);
    assert_eq!(code, 0, "{err}");
    assert_eq!(
        out,
        route("variables", &format!("{BODY}}}")).unwrap() + "\n"
    );
    assert!(out.contains("\"id\":\"matrix\""));
    let (_, out, _) = run(&[
        "samples",
        "vars",
        "--names",
        f.to_str().unwrap(),
        "--pos",
        "-1",
        "--sep",
        "_",
    ]);
    let want = route(
        "variables",
        &format!("{BODY},\"manual\":{{\"pos\":-1,\"sep\":\"_\"}}}}"),
    )
    .unwrap();
    assert_eq!(out, want + "\n");
    let (_, out, _) = run(&[
        "samples",
        "vars",
        "--names",
        f.to_str().unwrap(),
        "--has",
        "O2, N2",
        "--substring",
    ]);
    let want = route(
        "variables",
        &format!("{BODY},\"manual\":{{\"has\":[\"O2\",\"N2\"],\"substring\":true}}}}"),
    )
    .unwrap();
    assert_eq!(out, want + "\n");
    // the manual variable really is in the answer (a rule the engine ignored would give the same bytes on both sides)
    assert!(out.contains("\"manual\":{"), "{out}");
    std::fs::remove_file(f).unwrap();
}

/// The generic labels of the time points (t0, 3h, 8h, a replicate two ways, names that only look like a time).
const TIME_NAMES: [&str; 9] = [
    "RET t0 (2)",
    "RET 3h_2",
    "RET 8h (2)",
    "PERM t0",
    "PERM 3h",
    "PERM 8h",
    "H20 demi",
    "blk 90-10",
    "S 1 h 30",
];

#[test]
fn table_is_the_route_answer_with_the_time_points() {
    let p = std::env::temp_dir().join(format!("mzlab-cli-{}-names-time.txt", std::process::id()));
    std::fs::write(&p, TIME_NAMES.join("\n")).unwrap();
    let (code, out, err) = run(&["samples", "table", "--names", p.to_str().unwrap()]);
    assert_eq!(code, 0, "{err}");
    let files: Vec<String> = TIME_NAMES
        .iter()
        .map(|n| format!("{{\"name\":\"{n}\",\"sha256\":\"\",\"kind\":\"\"}}"))
        .collect();
    let want = route("propose", &format!("{{\"files\":[{}]}}", files.join(","))).unwrap();
    assert_eq!(out, want + "\n");
    // the minutes of every name, in the order of the names (null: no time point)
    let minutes: Vec<String> = out
        .split("\"time_min\":")
        .skip(1)
        .map(|r| r.split([',', '}']).next().unwrap().to_string())
        .collect();
    assert_eq!(
        minutes,
        ["0", "180", "480", "0", "180", "480", "null", "null", "90"]
    );
    std::fs::remove_file(p).unwrap();
}

#[test]
fn group_name_is_the_route_answer() {
    let (code, out, _) = run(&["samples", "group", "MQ_H2O2_T00", "MQ_H2O2_T01"]);
    assert_eq!((code, out.as_str()), (0, "{\"name\":\"MQ_H2O2\"}\n"));
    let (_, out, _) = run(&["samples", "group"]);
    assert_eq!(out, "{\"name\":\"\"}\n");
}

#[test]
fn bad_use_is_an_error_of_use() {
    for a in [
        vec!["samples", "nope"],
        vec![
            "samples",
            "vars",
            "--names",
            "/nonexistent/x.txt",
            "--substring",
        ],
        vec!["samples", "vars", "--pos", "x"],
    ] {
        assert_eq!(run(&a).0, 2, "{a:?}");
    }
    assert_eq!(
        run(&["samples", "vars", "--names", "/nonexistent/x.txt"]).0,
        4
    );
    assert_eq!(run(&["samples", "--help"]).0, 0);
}

/// The Italian command and subcommand names are gone: usage errors (exit 2); the help has no old-names line.
#[test]
fn the_italian_names_are_refused() {
    let f = names_file("alias");
    let n = f.to_str().unwrap();
    assert_eq!(run(&["samples", "vars", "--names", n]).0, 0);
    assert_eq!(run(&["samples", "group", "A_B", "A_C"]).0, 0);
    for bad in [
        vec!["campioni", "vars", "--names", n],
        vec!["samples", "variabili", "--names", n],
        vec!["samples", "nome-gruppo", "A_B", "A_C"],
    ] {
        assert_eq!(run(&bad).0, 2, "{bad:?}");
    }
    assert!(!run(&["samples", "--help"])
        .1
        .contains("old names still accepted"));
}

/// The synthetic pairs of tools/wiff_riferimento.py (python3 is part of the test: it fails without it).
fn pairs(tag: &str) -> std::path::PathBuf {
    let out = std::env::temp_dir().join(format!("mzlab-cli-{}-list-{tag}", std::process::id()));
    let script =
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tools/wiff_riferimento.py");
    let made = std::process::Command::new("python3")
        .arg(script)
        .arg(&out)
        .output()
        .expect("python3 is required");
    assert!(
        made.status.success(),
        "{}",
        String::from_utf8_lossy(&made.stderr)
    );
    out
}

#[test]
fn list_gives_data_on_stdout_progress_on_stderr_and_exit_codes() {
    let d = pairs("a");
    let at = |n: &str| d.join(format!("{n}.wiff")).to_str().unwrap().to_string();
    let (code, out, err) = run(&["samples", "list", &at("totale"), "--json"]);
    assert_eq!((code, out.lines().count()), (0, 1), "{err}");
    assert!(
        out.starts_with(r#"{"schema":"samples_list/1","format":"SciexWiff""#),
        "{out}"
    );
    assert!(err.contains("listing "), "{err}");
    // a refused sample is listed with its reason and the exit code says it
    let (code, out, _) = run(&["samples", "list", &at("rifiutato"), "--json"]);
    assert_eq!(code, 1);
    assert!(out.contains(r#""why":"refused""#), "{out}");
    // the probe adds what the `.wiff.scan` is
    let (code, out, _) = run(&["samples", "list", &at("totale"), "--json", "--probe"]);
    assert_eq!(code, 0);
    assert!(
        out.contains(r#""probe":{"scan_bytes":"#) && out.contains(r#""status":"ok""#),
        "{out}"
    );
    // a missing file is 4 and an error line, no usage is 2
    let (code, out, _) = run(&["samples", "list", &at("nothing"), "--json"]);
    assert_eq!(code, 4);
    assert!(out.contains(r#""schema":"error/1""#), "{out}");
    assert_eq!(run(&["samples", "list", "--json"]).0, 2);
    assert_eq!(run(&["samples", "list", &at("totale"), "--bogus"]).0, 2);
    let _ = std::fs::remove_dir_all(&d);
}

/// Every key of a JSON document, whatever its depth.
fn keys_of(v: &mzlab_core::json::Value, out: &mut std::collections::BTreeSet<String>) {
    use mzlab_core::json::Value;
    match v {
        Value::Obj(o) => {
            for (k, x) in o {
                out.insert(k.clone());
                keys_of(x, out);
            }
        }
        Value::Arr(a) => a.iter().for_each(|x| keys_of(x, out)),
        _ => {}
    }
}

/// The names the `properties` of a JSON Schema declare, at any depth.
fn schema_keys(file: &str) -> std::collections::BTreeSet<String> {
    let path = format!("{}/../../docs/schemi/{file}", env!("CARGO_MANIFEST_DIR"));
    let doc = mzlab_core::json::parse(&std::fs::read(path).unwrap()).unwrap();
    let mut all = std::collections::BTreeSet::new();
    keys_of(&doc, &mut all);
    let mut declared = std::collections::BTreeSet::new();
    fn walk(v: &mzlab_core::json::Value, d: &mut std::collections::BTreeSet<String>) {
        use mzlab_core::json::Value;
        match v {
            Value::Obj(o) => {
                for (k, x) in o {
                    if let (true, Value::Obj(p)) = (k == "properties", x) {
                        p.iter().for_each(|(n, _)| {
                            d.insert(n.clone());
                        });
                    }
                    walk(x, d);
                }
            }
            Value::Arr(a) => a.iter().for_each(|x| walk(x, d)),
            _ => {}
        }
    }
    walk(&doc, &mut declared);
    declared
}

/// The JSON of `vars`, `table` and `group` has no key outside the English schema: `table` only the keys of `samples-1.json`, `vars` and `group` only the
/// ones of `sample-vars/1` (saved formats are English only).
#[test]
fn the_json_has_only_the_keys_of_the_english_schema() {
    let vars: std::collections::BTreeSet<String> = [
        "schema",
        "common",
        "files",
        "name",
        "tokens",
        "class",
        "start",
        "end",
        "text",
        "variables",
        "id",
        "kind",
        "coverage",
        "score",
        "values",
        "key",
        "label",
        "without",
        "manual",
        "multiple",
    ]
    .iter()
    .map(|k| k.to_string())
    .collect();
    let table = schema_keys("samples-1.json");
    let group: std::collections::BTreeSet<String> = ["name".to_string()].into();
    let f = names_file("it");
    let names = f.to_str().unwrap();
    for (args, allowed) in [
        (vec!["samples", "vars", "--names", names], &vars),
        (
            vec!["samples", "vars", "--names", names, "--pos", "1"],
            &vars,
        ),
        (vec!["samples", "table", "--names", names], &table),
        (
            vec!["samples", "group", "MQ_H2O2_T00", "MQ_H2O2_T01"],
            &group,
        ),
    ] {
        let (code, out, err) = run(&args);
        assert_eq!(code, 0, "{err}");
        let mut got = std::collections::BTreeSet::new();
        keys_of(&mzlab_core::json::parse(out.as_bytes()).unwrap(), &mut got);
        let extra: Vec<_> = got.difference(allowed).collect();
        assert!(
            extra.is_empty(),
            "{args:?}: keys outside the schema: {extra:?}"
        );
    }
    std::fs::remove_file(f).unwrap();
}
