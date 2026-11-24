//! Tests of `mol cip`: the bytes of the shared fixture `cip_disegno.json`, which the molkit `.wasm` and the app give too, the keyed
//! refusals `error.cip.tooComplex` and `error.cip.digraphTooBig` of `cip_complex.json` and `cip_digraph.json` (exit 1, data) with their
//! parameters, the `capped` centre of a drawing that keeps another label, and the usage errors.
use crate::execute;

fn run(a: &[&str]) -> crate::Outcome {
    execute(&a.iter().map(|s| s.to_string()).collect::<Vec<_>>())
}

#[test]
fn the_command_prints_the_function_json() {
    // the fixture the app and the molkit `.wasm` are checked against too (tests/test_molkit_wasm.py)
    let fx: serde_json::Value =
        serde_json::from_str(include_str!("../../../tests/dati/cip_disegno.json")).unwrap();
    let req = fx["request"].to_string();
    let p = std::env::temp_dir().join(format!("mol_cip_{}.json", std::process::id()));
    std::fs::write(&p, req).unwrap();
    let o = run(&["mol", "cip", p.to_str().unwrap()]);
    std::fs::remove_file(&p).ok();
    assert_eq!(o.code, 0);
    assert_eq!(
        String::from_utf8(o.stdout).unwrap(),
        format!("{}\n", fx["cip"].as_str().unwrap())
    );
}

#[test]
fn a_structure_past_the_work_limit_or_the_node_cap_is_data_exit_1_with_its_evidence() {
    // the refusal fixtures the app and the molkit `.wasm` are checked against too (tests/test_molkit_wasm.py): `error.cip.tooComplex`
    // and `error.cip.digraphTooBig` (40001 nodes of 40000); the two-centre drawing of the second one exits 0 with `"capped":[4]`
    for (i, text) in [
        include_str!("../../../tests/dati/cip_complex.json"),
        include_str!("../../../tests/dati/cip_digraph.json"),
    ]
    .iter()
    .enumerate()
    {
        let fx: serde_json::Value = serde_json::from_str(text).unwrap();
        let p = std::env::temp_dir().join(format!("mol_cip_x{i}_{}.json", std::process::id()));
        std::fs::write(&p, fx["request"].to_string()).unwrap();
        let o = run(&["mol", "cip", p.to_str().unwrap(), "--json"]);
        assert_eq!(o.code, 1);
        let e: serde_json::Value = serde_json::from_slice(&o.stdout).unwrap();
        assert_eq!(e["error_key"], fx["error_key"]);
        assert_eq!(e["params"], fx["params"]);
        if let Some(part) = fx.get("partial") {
            std::fs::write(&p, part["request"].to_string()).unwrap();
            let o = run(&["mol", "cip", p.to_str().unwrap(), "--json"]);
            assert_eq!(o.code, 0);
            let out = String::from_utf8(o.stdout).unwrap();
            assert_eq!(out.trim_end(), part["cip"].as_str().unwrap());
        }
        std::fs::remove_file(&p).ok();
    }
}

#[test]
fn usage_errors_exit_2() {
    for a in [
        vec!["mol", "cip"],
        vec!["mol", "cip", "a", "b"],
        vec!["mol", "cip", "--nope"],
        vec!["mol", "cip", "/no/such/file.json"],
    ] {
        assert_eq!(run(&a).code, crate::exit::usage_or_missing(&a), "{a:?}");
    }
    assert_eq!(run(&["mol", "cip", "--help"]).code, 0);
}
