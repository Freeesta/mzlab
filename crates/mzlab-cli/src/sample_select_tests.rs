//! `--sample LIST`: `samples select`, `convert`, `channels` and `features` on a synthetic `.wiff` of three samples (`tools/wiff_grande.py`). python3 is part
//! of the test: without it the test fails, it never skips.

use std::path::{Path, PathBuf};
use std::process::Command;

use crate::{execute, Outcome};

fn pair(name: &str) -> (PathBuf, String) {
    let dir = std::env::temp_dir().join(format!("mzlab_cli_sample_{name}_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    let tools = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tools");
    let made = Command::new("python3")
        .arg(tools.join("wiff_grande.py"))
        .arg(dir.join("in"))
        .args(["--samples", "3", "--scans", "40", "--points", "50"])
        .output()
        .expect("python3 is needed by this test");
    assert!(
        made.status.success(),
        "{}",
        String::from_utf8_lossy(&made.stderr)
    );
    let wiff = dir.join("in/grande.wiff").to_string_lossy().into_owned();
    (dir, wiff)
}

fn run(a: &[&str]) -> Outcome {
    execute(&a.iter().map(|s| s.to_string()).collect::<Vec<_>>())
}

fn s(p: &Path) -> String {
    p.to_string_lossy().into_owned()
}

#[test]
fn select_names_the_samples_in_the_order_of_the_file_and_the_exit_codes_differ() {
    let (dir, wiff) = pair("select");
    let o = run(&["samples", "select", &wiff, "--sample", "3,1", "--json"]);
    assert_eq!(
        (o.code, String::from_utf8_lossy(&o.stdout).as_ref()),
        (0, "{\"samples\":[1,3]}\n"),
        "{}",
        o.stderr
    );
    let o = run(&["samples", "select", &wiff, "--sample", "2-3"]);
    assert_eq!(
        (o.code, String::from_utf8_lossy(&o.stdout).as_ref()),
        (0, "2 3\n")
    );
    // an id the file does not have: 5, the ids present named, an error/1 with --json
    let o = run(&["samples", "select", &wiff, "--sample", "1,9", "--json"]);
    assert_eq!(o.code, 5);
    assert!(
        o.stderr.contains("no sample 9") && o.stderr.contains("1, 2, 3"),
        "{}",
        o.stderr
    );
    let j = String::from_utf8_lossy(&o.stdout).into_owned();
    assert!(
        j.contains("\"schema\":\"error/1\"")
            && j.contains("\"code\":5")
            && j.contains("err.sample.unknown"),
        "{j}"
    );
    // not a list: usage, 2; a missing file: 4
    assert_eq!(
        run(&["samples", "select", &wiff, "--sample", "1,,2"]).code,
        2
    );
    assert_eq!(
        run(&["samples", "select", &wiff, "--sample", "3-1"]).code,
        2
    );
    assert_eq!(run(&["samples", "select", &wiff]).code, 2);
    let o = run(&[
        "samples",
        "select",
        &s(&dir.join("none.wiff")),
        "--sample",
        "1",
        "--json",
    ]);
    assert_eq!(o.code, 4);
    assert!(String::from_utf8_lossy(&o.stdout).contains("error/1"));
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn convert_writes_only_the_chosen_samples_with_the_bytes_of_a_full_run() {
    let (dir, wiff) = pair("convert");
    let (all, some) = (s(&dir.join("all")), s(&dir.join("some")));
    let o = run(&["convert", &wiff, "--out", &all]);
    assert_eq!(o.code, 0, "{}", o.stderr);
    let o = run(&["convert", &wiff, "--out", &some, "--sample", "1,3"]);
    assert_eq!(o.code, 0, "{}", o.stderr);
    let mut names: Vec<String> = std::fs::read_dir(&some)
        .unwrap()
        .flatten()
        .filter(|e| e.path().extension().is_some_and(|x| x == "mzML"))
        .map(|e| e.file_name().to_string_lossy().into_owned())
        .collect();
    names.sort();
    assert_eq!(names, ["grande-1.mzML", "grande-3.mzML"]);
    for n in &names {
        assert_eq!(
            std::fs::read(dir.join("some").join(n)).unwrap(),
            std::fs::read(dir.join("all").join(n)).unwrap(),
            "{n}"
        );
    }
    assert!(
        !o.stderr.contains("grande.wiff@2") && !o.stderr.contains("grande-2"),
        "{}",
        o.stderr
    );
    // one chosen sample still gets its NAME-ID name
    let one = s(&dir.join("one"));
    assert_eq!(
        run(&["convert", &wiff, "--out", &one, "--sample", "2"]).code,
        0
    );
    assert_eq!(
        std::fs::read(dir.join("one/grande-2.mzML")).unwrap(),
        std::fs::read(dir.join("all/grande-2.mzML")).unwrap()
    );
    // a wrong id stops the run before anything is written
    let bad = s(&dir.join("bad"));
    let o = run(&["convert", &wiff, "--out", &bad, "--sample", "4"]);
    assert_eq!(o.code, 5, "{}", o.stderr);
    assert!(
        o.stderr.contains("no sample 4") && !dir.join("bad").join("grande-1.mzML").exists(),
        "{}",
        o.stderr
    );
    assert_eq!(
        run(&["convert", &wiff, "--out", &bad, "--sample", "x"]).code,
        2
    );
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn channels_of_a_list_are_one_line_per_sample_each_the_bytes_of_its_own_run() {
    let (dir, wiff) = pair("channels");
    let list = run(&["channels", &wiff, "--sample", "3,1"]);
    assert_eq!(list.code, 0, "{}", list.stderr);
    let text = String::from_utf8(list.stdout).unwrap();
    let one = |k: &str| String::from_utf8(run(&["channels", &wiff, "--sample", k]).stdout).unwrap();
    assert_eq!(text, format!("{}\n{}", one("1").trim_end(), one("3")));
    assert_eq!(
        run(&["channels", &wiff, "--sample", "1,2", "--channel", "uv:twc"]).code,
        2
    );
    let o = run(&["channels", &wiff, "--sample", "1,9"]);
    assert_eq!(
        (
            o.code,
            String::from_utf8_lossy(&o.stdout)
                .matches("\"schema\":")
                .count()
        ),
        (5, 1),
        "no partial answer, one error/1: {}",
        o.stderr
    );
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn features_takes_one_member_per_chosen_sample() {
    let (dir, wiff) = pair("features");
    let o = run(&["features", &wiff, "--sample", "1,2"]);
    assert_eq!(o.code, 0, "{}", o.stderr);
    let states = |o: &Outcome| {
        String::from_utf8_lossy(&o.stdout)
            .lines()
            .next()
            .unwrap()
            .matches(":feature_state")
            .count()
    };
    assert_eq!(states(&o), 2, "one column group per chosen sample");
    assert_eq!(run(&["features", &wiff, "--sample", "7"]).code, 5);
    // without --sample a file of several samples is the first sample, as before
    let o = run(&["features", &wiff]);
    assert_eq!(o.code, 0, "{}", o.stderr);
    assert_eq!(states(&o), 1);
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn features_with_sample_reports_an_unreadable_file_not_a_single_sample_file() {
    let (dir, wiff) = pair("features_unreadable");
    // a .wiff whose companion scan file is gone cannot be listed: the error is the file's, exit 4, not "single sample" (2)
    let broken = dir.join("broken.wiff");
    std::fs::write(&broken, b"not a wiff container").unwrap();
    let o = run(&["features", broken.to_str().unwrap(), "--sample", "1"]);
    assert_eq!(o.code, 4, "{}", o.stderr);
    assert!(!o.stderr.contains("err.sample.single"), "{}", o.stderr);
    let _ = wiff;
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn select_default_ticks_a_small_set_and_nothing_when_it_would_not_fit() {
    let (dir, wiff) = pair("default");
    let big = run(&["samples", "select", &wiff, "--default", "--json"]);
    assert_eq!(
        (big.code, String::from_utf8_lossy(&big.stdout).trim()),
        (0, "{\"samples\":[1,2,3]}")
    );
    let tight = run(&[
        "samples",
        "select",
        &wiff,
        "--default",
        "--budget-mb",
        "0.0001",
        "--json",
    ]);
    assert_eq!(
        String::from_utf8_lossy(&tight.stdout).trim(),
        "{\"samples\":[]}"
    );
    // exactly one of the two ways to choose
    assert_eq!(
        run(&["samples", "select", &wiff, "--default", "--sample", "1"]).code,
        2
    );
    assert_eq!(run(&["samples", "select", &wiff]).code, 2);
    let _ = std::fs::remove_dir_all(dir);
}
