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
        "variables",
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
    assert!(out.contains(r#""schema":"errore/1""#), "{out}");
    assert_eq!(run(&["samples", "list", "--json"]).0, 2);
    assert_eq!(run(&["samples", "list", &at("totale"), "--bogus"]).0, 2);
    let _ = std::fs::remove_dir_all(&d);
}

/// The JSON of `vars`, `table` and `group` has no key of the old Italian formats (the saved formats are English only).
#[test]
fn the_json_has_no_italian_key() {
    const OLD: &[&str] = &[
        "campioni",
        "composto",
        "concentrazione",
        "ordine_iniezione",
        "replica",
        "serie",
        "tempo",
        "tempo_min",
        "tempo_confermato",
        "tipo",
        "tipo_bianco",
        "unita",
        "unita_tempo",
        "comune",
        "variabili",
        "copertura",
        "punteggio",
        "senza",
        "valori",
        "chiave",
        "etichetta",
        "classe",
        "fine",
        "inizio",
        "testo",
        "nome",
        "manuale",
        "multipli",
        "campione_id",
        "polarita",
        "matrice",
        "lotto",
        "gruppo",
        "colonne",
    ];
    let f = names_file("it");
    let names = f.to_str().unwrap();
    for args in [
        vec!["samples", "vars", "--names", names],
        vec!["samples", "vars", "--names", names, "--pos", "1"],
        vec!["samples", "table", "--names", names],
        vec!["samples", "group", "MQ_H2O2_T00", "MQ_H2O2_T01"],
    ] {
        let (code, out, err) = run(&args);
        assert_eq!(code, 0, "{err}");
        for w in OLD {
            assert!(
                !out.contains(&format!("\"{w}\"")),
                "{args:?}: `{w}` in the output"
            );
        }
    }
    std::fs::remove_file(f).unwrap();
}
