//! Tests of `mzlab-cli mol-mech` (`mol_mech_cmd.rs`): the same JSON as `mech_json`, and usage errors exit 2.

use crate::execute;

fn run(a: &[&str]) -> crate::Outcome {
    execute(&a.iter().map(|s| s.to_string()).collect::<Vec<_>>())
}

#[test]
fn the_command_prints_the_function_json() {
    let req = r#"{"molfile":"\n  qqq\n\n  2  1  0  0  0  0  0  0  0  0999 V2000\n    0.0000    0.0000    0.0000 C   0  0  0  0  0  0  0  0  0  0  0  0\n    1.0000    0.0000    0.0000 O   0  0  0  0  0  0  0  0  0  0  0  0\n  1  2  2  0  0  0  0\nM  END","arrows":[{"kind":"pair","from":{"bond":[0,1]},"to":{"atom":1}}],"lone":[[1,2]]}"#;
    let p = std::env::temp_dir().join(format!("mol_mech_{}.json", std::process::id()));
    std::fs::write(&p, req).unwrap();
    let o = run(&["mol-mech", p.to_str().unwrap()]);
    std::fs::remove_file(&p).ok();
    assert_eq!(o.code, 0);
    let want = mzlab_feat::chem::mech::mech_json(req).unwrap();
    assert!(want.ends_with("\"ok\":true}"), "{want}");
    assert_eq!(String::from_utf8(o.stdout).unwrap(), format!("{want}\n"));
}

#[test]
fn usage_errors_exit_2() {
    for a in [
        vec!["mol-mech"],
        vec!["mol-mech", "a", "b"],
        vec!["mol-mech", "--nope"],
        vec!["mol-mech", "/no/such/file.json"],
    ] {
        assert_eq!(run(&a).code, 2, "{a:?}");
    }
    assert_eq!(run(&["mol-mech", "--help"]).code, 0);
}
