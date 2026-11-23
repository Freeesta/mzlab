//! `mzlab-cli file convert --name sample`: the mzML is called after the sample, the bytes do not depend on the name, clashes and unsafe names are settled
//! deterministically and logged, the default naming is unchanged, a bad value is a usage error (exit 2).

use std::path::PathBuf;

use crate::convert_fixtures::{folder, wiff};
use crate::convert_tests::{convert, mzmls};
use crate::flow_run_tests::p;

fn names(dir: &std::path::Path) -> Vec<String> {
    mzmls(dir).into_iter().map(|f| f.0).collect()
}

#[test]
fn name_sample_calls_each_mzml_after_its_sample_with_the_same_bytes() {
    let dir = folder("name_sample");
    let src = dir.join("in");
    std::fs::create_dir_all(&src).unwrap();
    if !wiff(&src, "c", 2) {
        return;
    }
    let (plain, named) = (dir.join("plain"), dir.join("named"));
    let o = convert(&[p(&src.join("c.wiff")), "--out", p(&plain)]);
    assert_eq!(o.code, 0, "{}", o.stderr);
    let o = convert(&[
        p(&src.join("c.wiff")),
        "--out",
        p(&named),
        "--name",
        "sample",
    ]);
    assert_eq!(o.code, 0, "{}", o.stderr);
    // the default stays `<file>.wiff`-less `c-<id>`; the sample naming is the sample name, the same bytes
    assert_eq!(names(&plain), ["c-1.mzML", "c-2.mzML"]);
    assert_eq!(names(&named), ["c_s1.mzML", "c_s2.mzML"]);
    let (a, b) = (mzmls(&plain), mzmls(&named));
    assert_eq!(a[0].1, b[0].1);
    assert_eq!(a[1].1, b[1].1);
    // `--name file` is the default spelled out
    let spelled = dir.join("spelled");
    let o = convert(&[
        p(&src.join("c.wiff")),
        "--out",
        p(&spelled),
        "--name",
        "file",
    ]);
    assert_eq!(o.code, 0, "{}", o.stderr);
    assert_eq!(mzmls(&spelled), a);
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn equal_sample_names_get_their_id_and_a_note() {
    let dir = folder("name_clash");
    let src = dir.join("in");
    let (one, two) = (src.join("a"), src.join("b"));
    std::fs::create_dir_all(&one).unwrap();
    std::fs::create_dir_all(&two).unwrap();
    // two wiffs of the same name hold samples with the same names
    if !wiff(&one, "c", 2) || !wiff(&two, "c", 2) {
        return;
    }
    let out = dir.join("out");
    let args = [p(&src), "--out", p(&out), "--name", "sample"];
    let o = convert(&args);
    assert_eq!(o.code, 0, "{}", o.stderr);
    assert_eq!(
        names(&out),
        ["c_s1-1.mzML", "c_s1.mzML", "c_s2-2.mzML", "c_s2.mzML"]
    );
    assert_eq!(
        o.stderr.matches("is already taken").count(),
        2,
        "{}",
        o.stderr
    );
    // deterministic: a second run in a new folder gives the same names and the same bytes
    let again = dir.join("again");
    let o = convert(&[p(&src), "--out", p(&again), "--name", "sample"]);
    assert_eq!(o.code, 0, "{}", o.stderr);
    assert_eq!(mzmls(&again), mzmls(&out));
    // the report carries the same notes under `rename`, for a dry run too
    let report = dir.join("report");
    let json = convert(&[p(&src), "--out", p(&report), "--name", "sample", "--json"]);
    let text = String::from_utf8(json.stdout).unwrap();
    assert_eq!(text.matches("\"rename\":").count(), 2, "{text}");
    let dry = convert(&[
        p(&src),
        "--out",
        p(&report),
        "--name",
        "sample",
        "--dry-run",
        "--json",
    ]);
    let text = String::from_utf8(dry.stdout).unwrap();
    assert_eq!(text.matches("\"rename\":").count(), 2, "{text}");
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn the_dry_run_names_the_files_and_writes_nothing() {
    let dir = folder("name_dry");
    let src = dir.join("in");
    std::fs::create_dir_all(&src).unwrap();
    if !wiff(&src, "c", 2) {
        return;
    }
    let out: PathBuf = dir.join("out");
    let o = convert(&[
        p(&src.join("c.wiff")),
        "--out",
        p(&out),
        "--name",
        "sample",
        "--dry-run",
        "--json",
    ]);
    assert_eq!(o.code, 0, "{}", o.stderr);
    let text = String::from_utf8(o.stdout).unwrap();
    assert!(
        text.contains("c_s1.mzML") && text.contains("c_s2.mzML"),
        "{text}"
    );
    assert!(!out.join("c_s1.mzML").exists());
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn a_name_that_is_not_file_or_sample_is_a_usage_error() {
    let o = convert(&["x.wiff", "--out", "o", "--name", "id"]);
    assert_eq!(o.code, 2, "{}", o.stderr);
    assert!(o.stderr.contains("expected file or sample"), "{}", o.stderr);
    let o = convert(&["x.wiff", "--out", "o", "--name"]);
    assert_eq!(o.code, 2, "{}", o.stderr);
}

#[test]
fn another_input_never_replaces_a_named_mzml_without_fresh() {
    let dir = folder("name_overwrite");
    let src = dir.join("in");
    let (one, two) = (src.join("a"), src.join("b"));
    std::fs::create_dir_all(&one).unwrap();
    std::fs::create_dir_all(&two).unwrap();
    if !wiff(&one, "c", 2) || !wiff(&two, "c", 2) {
        return;
    }
    let out = dir.join("out");
    let run = |from: &std::path::Path, extra: &[&str]| {
        let wiff = from.join("c.wiff");
        let mut args = vec![p(&wiff), "--out", p(&out), "--name", "sample"];
        args.extend(extra);
        convert(&args)
    };
    assert_eq!(run(&one, &[]).code, 0);
    // the same input again is the same file: allowed (and cached)
    assert_eq!(run(&one, &[]).code, 0);
    let before = mzmls(&out);
    // another input would take the name of the first: refused, nothing changes, a dry run only looks
    let o = run(&two, &[]);
    assert_eq!(o.code, 2, "{}", o.stderr);
    assert!(
        o.stderr.contains("not the mzML of this input"),
        "{}",
        o.stderr
    );
    assert_eq!(mzmls(&out), before);
    assert_eq!(run(&two, &["--dry-run"]).code, 0);
    // `--fresh` is the explicit replacement
    assert_eq!(run(&two, &["--fresh"]).code, 0);
    // a file the table does not know (made by hand) is refused too
    std::fs::write(out.join("c_s1.mzML"), b"mine").unwrap();
    let o = run(&one, &[]);
    assert_eq!(o.code, 2, "{}", o.stderr);
    let _ = std::fs::remove_dir_all(&dir);
}
