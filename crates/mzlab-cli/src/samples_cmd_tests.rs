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
fn variabili_is_the_route_answer() {
    let f = names_file("a");
    let (code, out, err) = run(&["samples", "vars", "--names", f.to_str().unwrap()]);
    assert_eq!(code, 0, "{err}");
    assert_eq!(
        out,
        route("variabili", &format!("{BODY}}}")).unwrap() + "\n"
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
        "variabili",
        &format!("{BODY},\"manuale\":{{\"pos\":-1,\"sep\":\"_\"}}}}"),
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
        "variabili",
        &format!("{BODY},\"manuale\":{{\"has\":[\"O2\",\"N2\"],\"substring\":true}}}}"),
    )
    .unwrap();
    assert_eq!(out, want + "\n");
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
    let p = std::env::temp_dir().join(format!("mzlab-cli-{}-nomi-tempo.txt", std::process::id()));
    std::fs::write(&p, TIME_NAMES.join("\n")).unwrap();
    let (code, out, err) = run(&["samples", "table", "--names", p.to_str().unwrap()]);
    assert_eq!(code, 0, "{err}");
    let files: Vec<String> = TIME_NAMES
        .iter()
        .map(|n| format!("{{\"name\":\"{n}\",\"sha256\":\"\",\"kind\":\"\"}}"))
        .collect();
    let want = route("proponi", &format!("{{\"files\":[{}]}}", files.join(","))).unwrap();
    assert_eq!(out, want + "\n");
    // the minutes of every name, in the order of the names (null: no time point)
    let minutes: Vec<String> = out
        .split("\"tempo_min\":")
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
fn nome_gruppo_is_the_route_answer() {
    let (code, out, _) = run(&["samples", "group", "MQ_H2O2_T00", "MQ_H2O2_T01"]);
    assert_eq!((code, out.as_str()), (0, "{\"nome\":\"MQ_H2O2\"}\n"));
    let (_, out, _) = run(&["samples", "group"]);
    assert_eq!(out, "{\"nome\":\"\"}\n");
}

#[test]
fn bad_use_is_an_error_of_use() {
    for a in [
        vec!["samples", "nope"],
        vec!["samples", "vars", "--names", "/nonexistent/x.txt"],
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
