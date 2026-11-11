//! Tests of `mzlab-cli mol-cut` (`mol_cut_cmd.rs`): the same JSON as `cut_json`, and usage errors exit 2.

use crate::execute;

const ETHANOL: &str = "\n  qqq\n\n  3  2  0  0  0  0  0  0  0  0999 V2000\n    0.0000    0.0000    0.0000 C   0  0  0  0  0  0  0  0  0  0  0  0\n    1.0000    0.0000    0.0000 C   0  0  0  0  0  0  0  0  0  0  0  0\n    2.0000    0.0000    0.0000 O   0  0  0  0  0  0  0  0  0  0  0  0\n  1  2  1  0  0  0  0\n  2  3  1  0  0  0  0\nM  END";

fn run(a: &[&str]) -> crate::Outcome {
    execute(&a.iter().map(|s| s.to_string()).collect::<Vec<_>>())
}

#[test]
fn the_command_prints_the_function_json() {
    let p = std::env::temp_dir().join(format!("mol_cut_{}.mol", std::process::id()));
    std::fs::write(&p, ETHANOL).unwrap();
    let f = p.to_str().unwrap();
    let o = run(&[
        "mol-cut",
        f,
        "--set",
        "bonds=1-2",
        "--set",
        "mode=heterolytic",
        "--set",
        "cation=2",
        "--set",
        "gap=0.5",
    ]);
    let home = run(&["mol-cut", f, "--set", "bonds=0-1"]);
    std::fs::remove_file(&p).ok();
    assert_eq!(o.code, 0);
    let mut q = mzlab_core::json::Out::default();
    q.str(ETHANOL);
    let req = format!(
        "{{\"molfile\":{},\"bonds\":[[1,2]],\"mode\":\"heterolytic\",\"cation\":[2],\"gap\":0.5}}",
        q.0
    );
    let want = mzlab_feat::chem::cut::cut_json(&req).unwrap();
    assert_eq!(String::from_utf8(o.stdout).unwrap(), format!("{want}\n"));
    assert!(String::from_utf8(home.stdout)
        .unwrap()
        .contains("\"mode\":\"homolytic\""));
}

#[test]
fn usage_errors_exit_2() {
    for a in [
        vec!["mol-cut"],
        vec!["mol-cut", "a", "b"],
        vec!["mol-cut", "--nope"],
        vec!["mol-cut", "/no/such/file.mol"],
        vec!["mol-cut", "-", "--set", "colour=red"],
        vec!["mol-cut", "-", "--set", "bonds"],
    ] {
        assert_eq!(run(&a).code, 2, "{a:?}");
    }
    assert_eq!(run(&["mol-cut", "--help"]).code, 0);
}
