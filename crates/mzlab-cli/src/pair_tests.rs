//! `mzlab-cli pair` and the companion rule on a synthetic folder: names only (empty files), then a real synthetic pair (`tools/wiff_grande.py`, needs
//! python3 and numpy: it fails without them) opened from either half, also when the case of the names differs.

use std::path::Path;

use mzlab_core::json::parse;

use crate::execute;

fn cli(a: &[&str]) -> crate::Outcome {
    execute(&a.iter().map(|s| s.to_string()).collect::<Vec<_>>())
}

fn touch(dir: &Path, names: &[&str]) {
    for n in names {
        std::fs::write(dir.join(n), b"x").unwrap();
    }
}

fn tmp(tag: &str) -> std::path::PathBuf {
    let d = std::env::temp_dir().join(format!("mzlab-pair-{}-{tag}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).unwrap();
    d
}

#[test]
fn either_half_finds_the_other_ignoring_case_and_lists_only_names() {
    let d = tmp("names");
    touch(
        &d,
        &["Run 1.wiff", "run 1.WIFF.SCAN", "other.wiff", "Run 1.mzML"],
    );
    let w = d.join("Run 1.wiff").to_string_lossy().into_owned();
    let s = d.join("run 1.WIFF.SCAN").to_string_lossy().into_owned();
    for (input, half, found) in [(&w, "wiff", "run 1.WIFF.SCAN"), (&s, "scan", "Run 1.wiff")] {
        let o = cli(&["pair", input, "--json"]);
        assert_eq!(o.code, 0, "{}", o.stderr);
        let v = parse(&o.stdout).unwrap();
        assert_eq!(v.get("schema").unwrap().str(), Some("wiffpair/1"));
        let p = &v.get("pairs").unwrap().arr()[0];
        assert_eq!(p.get("half").unwrap().str(), Some(half));
        assert!(p.get("found").unwrap().str().unwrap().ends_with(found));
        assert_eq!(p.get("size").unwrap().f64(), Some(1.0));
    }
    let _ = std::fs::remove_dir_all(&d);
}

#[test]
fn a_missing_companion_exits_1_and_says_what_was_looked_for() {
    let d = tmp("missing");
    touch(&d, &["lone.wiff", "plain.mzML"]);
    let o = cli(&["pair", &d.join("lone.wiff").to_string_lossy(), "--json"]);
    assert_eq!(o.code, 1);
    let v = parse(&o.stdout).unwrap();
    assert_eq!(v.get("ok").unwrap().bool(), Some(false));
    let p = &v.get("pairs").unwrap().arr()[0];
    assert!(p.get("found").unwrap().is_null());
    assert_eq!(p.get("wanted").unwrap().str(), Some("lone.wiff.scan"));
    assert!(p
        .get("looked_in")
        .unwrap()
        .str()
        .unwrap()
        .contains("mzlab-pair-"));
    let plain = cli(&["pair", &d.join("lone.wiff").to_string_lossy()]);
    assert_eq!(plain.code, 1);
    assert!(String::from_utf8_lossy(&plain.stdout).contains("missing\tlone.wiff.scan"));
    let _ = std::fs::remove_dir_all(&d);
}

#[test]
fn usage_errors_exit_2() {
    let d = tmp("usage");
    touch(&d, &["a.mzML", "b.scan"]);
    for a in [
        vec!["pair"],
        vec!["pair", "--nope"],
        vec!["pair", d.join("a.mzML").to_str().unwrap()],
        vec!["pair", d.join("b.scan").to_str().unwrap()],
    ] {
        assert_eq!(cli(&a).code, 2, "{a:?}");
    }
    assert_eq!(cli(&["pair", "--help"]).code, 0);
    let _ = std::fs::remove_dir_all(&d);
}

/// A real synthetic pair given by its `.wiff` alone, or by its `.wiff.scan` alone, with the case of the two names different: the same samples.
#[test]
fn a_synthetic_pair_opens_from_either_half() {
    let d = tmp("real");
    assert!(
        crate::convert_fixtures::wiff(&d, "Pair", 2),
        "the synthetic pair needs python3 and numpy (tools/wiff_grande.py)"
    );
    // the scan renamed in capitals: the engine's rule still pairs them
    std::fs::rename(d.join("Pair.wiff.scan"), d.join("PAIR.WIFF.SCAN")).unwrap();
    let ids = |f: &str| {
        let o = cli(&["channels", &d.join(f).to_string_lossy(), "--sample", "2"]);
        assert_eq!(o.code, 0, "{}", o.stderr);
        o.stdout
    };
    assert_eq!(ids("Pair.wiff"), ids("PAIR.WIFF.SCAN"));
    let _ = std::fs::remove_dir_all(&d);
}
