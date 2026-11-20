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
        String::from_utf8(o.stdout.clone()).unwrap(),
        format!(
            "{}\n",
            mzlab_feat::chem::structure_file::open_json_bytes(bytes).unwrap()
        )
    );
    // --json is the default form and changes nothing
    let j = run(&["mol", "open", p.to_str().unwrap(), "--json"]);
    assert_eq!((j.code, &j.stdout), (0, &o.stdout));
    // a verdict: --check exit 1 keeps the open/1 document on stdout, with or without --json
    for a in [vec!["--check"], vec!["--check", "--json"]] {
        let mut args = vec!["mol", "open", p.to_str().unwrap()];
        args.extend(a);
        let c = run(&args);
        assert_eq!(c.code, 1, "a file that needed a change exits 1");
        let out = String::from_utf8(c.stdout).unwrap();
        assert!(out.contains("\"schema\":\"open/1\"") && !out.contains("error/1"), "{out}");
        assert!(out.contains("\"names\":[\"BondLength\"]"), "{out}");
    }
    std::fs::remove_file(&p).ok();
}

#[test]
fn a_file_over_the_limit_is_one_error_document_exit_1_and_an_unreadable_input_exits_4() {
    let max = mzlab_feat::chem::structure_file::MAX_BYTES;
    let p = file("big.cdxml", &vec![b' '; max + 1]);
    for a in [vec![], vec!["--json"], vec!["--check"]] {
        let mut args = vec!["mol", "open", p.to_str().unwrap()];
        args.extend(a);
        let o = run(&args);
        assert_eq!(o.code, 1, "{args:?}");
        let out = String::from_utf8(o.stdout).unwrap();
        assert!(out.contains("\"schema\":\"error/1\"") && out.contains("error.open.tooBig"), "{out}");
    }
    std::fs::remove_file(&p).ok();
    // a folder opens but cannot be read: the contract's unreadable input (4), as one error/1
    let dir = std::env::temp_dir();
    let o = run(&["mol", "open", dir.to_str().unwrap(), "--json"]);
    assert_eq!(o.code, 4);
    assert!(String::from_utf8(o.stdout).unwrap().contains("\"schema\":\"error/1\""));
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
