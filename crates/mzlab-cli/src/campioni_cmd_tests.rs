//! `mzlab-cli campioni`: the answers are the ones of `campioni::route` on the same names.

use mzlab_core::campioni::route_tutto as route;

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
    let (code, out, err) = run(&["campioni", "variabili", "--names", f.to_str().unwrap()]);
    assert_eq!(code, 0, "{err}");
    assert_eq!(
        out,
        route("variabili", &format!("{BODY}}}")).unwrap() + "\n"
    );
    assert!(out.contains("\"id\":\"matrix\""));
    let (_, out, _) = run(&[
        "campioni",
        "variabili",
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
        "campioni",
        "variabili",
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

#[test]
fn nome_gruppo_is_the_route_answer() {
    let (code, out, _) = run(&["campioni", "nome-gruppo", "MQ_H2O2_T00", "MQ_H2O2_T01"]);
    assert_eq!((code, out.as_str()), (0, "{\"nome\":\"MQ_H2O2\"}\n"));
    let (_, out, _) = run(&["campioni", "nome-gruppo"]);
    assert_eq!(out, "{\"nome\":\"\"}\n");
}

#[test]
fn bad_use_is_an_error_of_use() {
    for a in [
        vec!["campioni", "nope"],
        vec!["campioni", "variabili", "--names", "/nonexistent/x.txt"],
        vec![
            "campioni",
            "variabili",
            "--names",
            "/nonexistent/x.txt",
            "--substring",
        ],
        vec!["campioni", "variabili", "--pos", "x"],
    ] {
        assert_eq!(run(&a).0, 2, "{a:?}");
    }
    assert_eq!(run(&["campioni", "--help"]).0, 0);
}

#[test]
fn old_and_new_names_give_the_same_bytes() {
    let f = names_file("alias");
    let n = f.to_str().unwrap();
    for (old, new) in [
        (
            vec!["campioni", "variabili", "--names", n],
            vec!["samples", "vars", "--names", n],
        ),
        (
            vec!["campioni", "variabili", "--names", n, "--pos", "2"],
            vec!["samples", "vars", "--names", n, "--pos", "2"],
        ),
        (
            vec!["campioni", "nome-gruppo", "MQ_H2O2_T00", "MQ_H2O2_T01"],
            vec!["samples", "group", "MQ_H2O2_T00", "MQ_H2O2_T01"],
        ),
        (vec!["campioni", "nope"], vec!["samples", "nope"]),
        (vec!["campioni", "--help"], vec!["samples", "--help"]),
        (vec!["campioni"], vec!["samples"]),
    ] {
        let (a, b) = (run(&old), run(&new));
        assert_eq!((a.0, &a.1), (b.0, &b.1), "{old:?}");
    }
    // The mixed spellings are accepted too.
    assert_eq!(run(&["samples", "variabili", "--names", n]).0, 0);
    assert_eq!(run(&["campioni", "group", "A_B", "A_C"]).0, 0);
    assert!(run(&["samples", "--help"])
        .1
        .ends_with("old names still accepted: campioni, variabili, nome-gruppo\n"));
}
