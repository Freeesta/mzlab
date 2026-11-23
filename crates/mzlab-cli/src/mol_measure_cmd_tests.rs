//! `mzlab-cli mol measure`: the same bytes as the engine function and the fixture of `tests/dati/descriptors_disegno.json`, and the exit codes.
use crate::execute;

fn run(a: &[&str]) -> crate::Outcome {
    execute(&a.iter().map(|s| s.to_string()).collect::<Vec<_>>())
}

fn tmp(name: &str, body: &str) -> String {
    let p = std::env::temp_dir().join(format!("mol_descr_{}_{name}", std::process::id()));
    std::fs::write(&p, body).unwrap();
    p.to_str().unwrap().to_string()
}

#[test]
fn the_command_prints_the_function_json() {
    let fx: serde_json::Value = serde_json::from_str(
        &std::fs::read_to_string(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../tests/dati/descriptors_disegno.json"
        ))
        .unwrap(),
    )
    .unwrap();
    for c in fx["cases"].as_array().unwrap() {
        let req = c["request"].to_string();
        let p = tmp("req.json", &req);
        let o = run(&["mol", "measure", &p]);
        assert_eq!(o.code, 0, "{}", o.stderr);
        let want = mzlab_feat::chem::descriptors::descriptors_json(&req).unwrap();
        assert_eq!(String::from_utf8(o.stdout).unwrap(), format!("{want}\n"));
        assert_eq!(want, c["descriptors"].as_str().unwrap(), "{}", c["name"]);
    }
}

#[test]
fn errors_have_their_exit_codes() {
    for a in [
        vec!["mol", "measure"],
        vec!["mol", "measure", "a", "b"],
        vec!["mol", "measure", "--nope"],
        vec!["mol", "measure", "/no/such/file.json"],
    ] {
        assert_eq!(run(&a).code, crate::exit::usage_or_missing(&a), "{a:?}");
    }
    assert_eq!(run(&["mol", "measure", "--help"]).code, 0);
    let bad = tmp("bad.json", r#"{"molfile":"nonsense"}"#);
    let o = run(&["mol", "measure", &bad, "--json"]);
    assert_eq!(o.code, 4);
    let e: serde_json::Value = serde_json::from_slice(&o.stdout).unwrap();
    assert_eq!(e["error_key"], "error.descr.molfile");
    let not_json = tmp("not.json", "{");
    assert_eq!(run(&["mol", "measure", &not_json]).code, 4);
}
