//! `mzlab-cli mol-export`: the command prints the bytes of `chem::sdf::export`; usage errors exit 2.

use crate::execute;

fn run(a: &[&str]) -> crate::Outcome {
    execute(&a.iter().map(|s| s.to_string()).collect::<Vec<_>>())
}

const MOL: &str = "\n  t\n\n  2  1  0  0  0  0  0  0  0  0999 V2000\n    0.0000    0.0000    0.0000 C   0  0  0  0  0  0  0  0  0  0  0  0\n    1.5000    0.0000    0.0000 O   0  0  0  0  0  0  0  0  0  0  0  0\n  1  2  1  0  0  0  0\nM  END\n";

#[test]
fn the_command_prints_the_function_text() {
    let p = std::env::temp_dir().join(format!("mol_export_{}.mol", std::process::id()));
    std::fs::write(&p, MOL).unwrap();
    let path = p.to_str().unwrap();
    let o = run(&[
        "mol-export",
        path,
        "--set",
        "ion=[M+H]+",
        "--set",
        "name=methanol",
    ]);
    let m = run(&["mol-export", path, "--set", "format=mol"]);
    std::fs::remove_file(&p).ok();
    assert_eq!(o.code, 0);
    let want = mzlab_feat::chem::sdf::export(MOL, "sdf", "[M+H]+", "methanol").unwrap();
    assert_eq!(String::from_utf8(o.stdout).unwrap(), want);
    assert!(want.contains("> <MZLAB_FORMULA>\nCH4O\n") && want.ends_with("$$$$\n"));
    assert_eq!(
        String::from_utf8(m.stdout).unwrap(),
        mzlab_feat::chem::sdf::export(MOL, "mol", "none", "").unwrap()
    );
}

#[test]
fn usage_errors_exit_2() {
    for a in [
        vec!["mol-export"],
        vec!["mol-export", "a", "b"],
        vec!["mol-export", "--nope"],
        vec!["mol-export", "a", "--set", "colour=red"],
        vec!["mol-export", "/no/such/file.mol"],
    ] {
        assert_eq!(run(&a).code, 2, "{a:?}");
    }
    assert_eq!(run(&["mol-export", "--help"]).code, 0);
}
