//! Tests of `mzlab-cli mol open` (`mol_open_cmd.rs`): the same JSON as `open_json_bytes`, `--check` exits 1 for a file that needed a change, and usage errors exit 2.

use crate::execute;

fn run(a: &[&str]) -> crate::Outcome {
    execute(&a.iter().map(|s| s.to_string()).collect::<Vec<_>>())
}

fn file(name: &str, bytes: &[u8]) -> std::path::PathBuf {
    let d = std::env::temp_dir().join(format!("mol_open_{}_{name}", std::process::id()));
    std::fs::write(&d, bytes).unwrap();
    d
}

#[test]
fn the_command_prints_the_function_json() {
    let bytes = b"<CDXML BondLength=\"\" Name=\"a\"><page/></CDXML>";
    let p = file("a.cdxml", bytes);
    let o = run(&["mol", "open", p.to_str().unwrap()]);
    assert_eq!(o.code, 0);
    assert_eq!(
        String::from_utf8(o.stdout).unwrap(),
        format!(
            "{}\n",
            mzlab_feat::chem::structure_file::open_json_bytes(bytes)
        )
    );
    let c = run(&["mol", "open", p.to_str().unwrap(), "--check"]);
    assert_eq!(c.code, 1, "a file that needed a change exits 1");
    assert!(!c.stdout.is_empty());
    std::fs::remove_file(&p).ok();
}

#[test]
fn a_file_read_as_it_is_exits_0_under_check() {
    let p = file("b.cdxml", b"<CDXML Name=\"a\"><page/></CDXML>");
    assert_eq!(
        run(&["mol", "open", p.to_str().unwrap(), "--check"]).code,
        0
    );
    std::fs::remove_file(&p).ok();
}

#[test]
fn usage_errors_exit_2() {
    for a in [
        vec!["mol", "open"],
        vec!["mol", "open", "a", "b"],
        vec!["mol", "open", "--nope"],
        vec!["mol", "open", "/no/such/file.cdxml"],
    ] {
        assert_eq!(run(&a).code, crate::exit::usage_or_missing(&a), "{a:?}");
    }
    assert_eq!(run(&["mol", "open", "--help"]).code, 0);
}
