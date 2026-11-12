//! Tests of `mzlab-cli mol-suspects` (`mol_suspects_cmd.rs`): the same bytes as `tray_text` (the CSV as is, JSON with a line break), `--out`, and
//! usage errors exit 2.

use crate::execute;

fn run(a: &[&str]) -> crate::Outcome {
    execute(&a.iter().map(|s| s.to_string()).collect::<Vec<_>>())
}

fn with_file(req: &str, tag: &str, f: impl Fn(&str) -> crate::Outcome) -> crate::Outcome {
    let p = std::env::temp_dir().join(format!("mol_suspects_{tag}_{}.json", std::process::id()));
    std::fs::write(&p, req).unwrap();
    let o = f(p.to_str().unwrap());
    std::fs::remove_file(&p).ok();
    o
}

#[test]
fn the_csv_is_written_as_the_function_gives_it() {
    let req = r#"{"op":"csv","entries":[{"name":"ethanol","smiles":"CCO","formula":"C2H6O","mass":46.041865}],"adducts":["[M+H]+"]}"#;
    let o = with_file(req, "csv", |p| run(&["mol-suspects", p]));
    assert_eq!(o.code, 0);
    let want = mzlab_feat::chem::tray::tray_text(req).unwrap();
    assert_eq!(
        want,
        "name,smiles,formula,mass,charge,adducts\nethanol,CCO,C2H6O,46.041865,0,[M+H]+\n"
    );
    assert_eq!(String::from_utf8(o.stdout).unwrap(), want);
}

#[test]
fn a_json_answer_ends_with_a_line_break_and_goes_to_out() {
    let req = r#"{"op":"entries","structures":[{"name":"m","smiles":"CO","atoms":[{"el":"C","h":3},{"el":"O","h":1}]}]}"#;
    let out = std::env::temp_dir().join(format!("mol_suspects_out_{}.json", std::process::id()));
    let o = with_file(req, "json", |p| {
        run(&["mol-suspects", p, "--out", out.to_str().unwrap()])
    });
    assert_eq!(o.code, 0);
    let got = std::fs::read_to_string(&out).unwrap();
    std::fs::remove_file(&out).ok();
    assert_eq!(
        got,
        format!("{}\n", mzlab_feat::chem::tray::tray_text(req).unwrap())
    );
}

#[test]
fn usage_errors_exit_2() {
    for a in [
        vec!["mol-suspects"],
        vec!["mol-suspects", "a", "b"],
        vec!["mol-suspects", "--nope"],
        vec!["mol-suspects", "/no/such/file.json"],
    ] {
        assert_eq!(run(&a).code, 2, "{a:?}");
    }
    let bad = with_file(r#"{"op":"csv","entries":[],"adducts":[]}"#, "bad", |p| {
        run(&["mol-suspects", p])
    });
    assert_eq!(bad.code, 2);
    assert_eq!(run(&["mol-suspects", "--help"]).code, 0);
}
