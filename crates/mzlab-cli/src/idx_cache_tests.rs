//! The index cache: a second call reuses the entry and prints the same bytes, a changed file or a damaged entry is rebuilt, `file index` fills it.

use super::*;
use crate::execute;
use std::path::Path;

fn args(a: &[&str]) -> Vec<String> {
    a.iter().map(|s| s.to_string()).collect()
}

/// A folder of its own for the test, with a synthetic mzML in it.
fn scratch(tag: &str) -> (PathBuf, PathBuf) {
    let dir = std::env::temp_dir().join(format!("mzlab-cli-{}-cache-{tag}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let f = dir.join("s.mzML");
    std::fs::write(&f, crate::tests::mzml(2.0)).unwrap();
    (dir, f)
}

#[test]
fn a_second_call_reuses_the_index_and_prints_the_same_bytes() {
    let (dir, f) = scratch("reuse");
    let (c, f) = (dir.join("c"), f.display().to_string());
    let cold = execute(&args(&["file", "items", &f]));
    let first = execute(&args(&[
        "file",
        "items",
        &f,
        "--cache",
        &c.display().to_string(),
    ]));
    let second = execute(&args(&[
        "file",
        "items",
        &f,
        "--cache",
        &c.display().to_string(),
    ]));
    assert_eq!((cold.code, first.code, second.code), (0, 0, 0));
    assert!(first.stderr.contains("index built"), "{}", first.stderr);
    assert!(second.stderr.contains("index reused"), "{}", second.stderr);
    assert_eq!(cold.stdout, first.stdout);
    assert_eq!(cold.stdout, second.stdout);
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn a_modified_file_gets_a_new_entry() {
    let (dir, f) = scratch("modified");
    let (c, fs) = (dir.join("c").display().to_string(), f.display().to_string());
    let run = || execute(&args(&["file", "items", &fs, "--cache", &c]));
    assert!(run().stderr.contains("index built"));
    assert!(run().stderr.contains("index reused"));
    // same length, other bytes, other modification time
    let mut b = std::fs::read(&f).unwrap();
    let at = b.windows(4).position(|w| w == b"\"t2\"").unwrap();
    b[at + 2] = b'3';
    std::fs::write(&f, b).unwrap();
    std::fs::File::options()
        .write(true)
        .open(&f)
        .unwrap()
        .set_modified(std::time::SystemTime::now() + std::time::Duration::from_secs(5))
        .unwrap();
    assert!(run().stderr.contains("index built"));
    let n = std::fs::read_dir(dir.join("c"))
        .unwrap()
        .filter(|e| {
            e.as_ref()
                .unwrap()
                .file_name()
                .to_string_lossy()
                .ends_with(".mzidx")
        })
        .count();
    assert_eq!(n, 2, "one entry per content");
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn a_damaged_entry_is_rebuilt_with_a_warning() {
    let (dir, f) = scratch("damaged");
    let (c, fs) = (dir.join("c"), f.display().to_string());
    let cs = c.display().to_string();
    let good = execute(&args(&["file", "items", &fs, "--cache", &cs]));
    for e in std::fs::read_dir(&c).unwrap() {
        let p = e.unwrap().path();
        if p.extension().is_some_and(|x| x == "mzidx") {
            let len = std::fs::metadata(&p).unwrap().len();
            std::fs::File::options()
                .write(true)
                .open(&p)
                .unwrap()
                .set_len(len / 2)
                .unwrap();
        }
    }
    let again = execute(&args(&["file", "items", &fs, "--cache", &cs]));
    assert!(
        again.stderr.contains("warn.cache.stale"),
        "{}",
        again.stderr
    );
    assert!(again.stderr.contains("index built"));
    assert_eq!(good.stdout, again.stdout);
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn file_index_fills_the_cache_and_the_next_command_reuses_it() {
    let (dir, f) = scratch("prebuild");
    let (cs, fs) = (dir.join("c").display().to_string(), f.display().to_string());
    let a = execute(&args(&["file", "index", &fs, "--cache", &cs]));
    assert_eq!(a.code, 0, "{}", a.stderr);
    let doc = String::from_utf8(a.stdout).unwrap();
    assert!(doc.starts_with("{\"schema\":\"index-build/1\""), "{doc}");
    assert!(doc.contains("\"status\":\"built\""));
    let b = execute(&args(&["file", "index", &fs, "--cache", &cs]));
    assert!(String::from_utf8(b.stdout)
        .unwrap()
        .contains("\"status\":\"reused\""));
    let c = execute(&args(&["file", "channels", &fs, "--cache", &cs]));
    assert!(c.stderr.contains("index reused"), "{}", c.stderr);
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn file_index_without_a_cache_is_a_usage_error_and_an_unwritable_cache_is_exit_6() {
    let (dir, f) = scratch("errors");
    let fs = f.display().to_string();
    assert_eq!(execute(&args(&["file", "index", &fs])).code, 2);
    // a file where the cache folder should be
    let blocker = dir.join("blocker");
    std::fs::write(&blocker, b"x").unwrap();
    let bad = blocker.join("c").display().to_string();
    assert_eq!(
        execute(&args(&["file", "index", &fs, "--cache", &bad])).code,
        6
    );
    // the other commands go on without it and say so
    let o = execute(&args(&["file", "items", &fs, "--cache", &bad]));
    assert_eq!(o.code, 0, "{}", o.stderr);
    assert!(o.stderr.contains("warn.cache.unusable"), "{}", o.stderr);
    assert_eq!(execute(&args(&["file", "items", &fs, "--cache"])).code, 2);
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn features_on_a_series_gives_the_same_table_with_the_cache_and_reuses_every_index() {
    let (dir, a) = scratch("series");
    let b = dir.join("t.mzML");
    std::fs::write(&b, crate::tests::mzml(4.0)).unwrap();
    let (cs, a, b) = (
        dir.join("c").display().to_string(),
        a.display().to_string(),
        b.display().to_string(),
    );
    let plain = execute(&args(&["features", &a, &b]));
    let cold = execute(&args(&["features", &a, &b, "--cache", &cs]));
    let warm = execute(&args(&["features", &a, &b, "--cache", &cs]));
    assert_eq!(
        (plain.code, cold.code, warm.code),
        (0, 0, 0),
        "{}",
        warm.stderr
    );
    assert_eq!(plain.stdout, cold.stdout);
    assert_eq!(plain.stdout, warm.stdout);
    assert_eq!(
        cold.stderr.matches("index built").count(),
        2,
        "{}",
        cold.stderr
    );
    assert_eq!(
        warm.stderr.matches("index reused").count(),
        2,
        "{}",
        warm.stderr
    );
    let _ = std::fs::remove_dir_all(&dir);
}

fn entries(c: &Path) -> Vec<PathBuf> {
    let mut v: Vec<PathBuf> = std::fs::read_dir(c)
        .unwrap()
        .map(|e| e.unwrap().path())
        .filter(|p| p.extension().is_some_and(|x| x == "mzidx"))
        .collect();
    v.sort();
    v
}

#[test]
fn a_file_changed_with_the_same_size_and_the_same_time_is_not_taken_for_the_old_one() {
    let (dir, f) = scratch("same-stamp");
    let (c, fs) = (dir.join("c"), f.display().to_string());
    let run = || {
        execute(&args(&[
            "file",
            "items",
            &fs,
            "--cache",
            &c.display().to_string(),
        ]))
    };
    assert!(run().stderr.contains("index built"));
    let time = std::fs::metadata(&f).unwrap().modified().unwrap();
    let mut b = std::fs::read(&f).unwrap();
    let at = b.windows(4).position(|w| w == b"\"t2\"").unwrap();
    b[at + 2] = b'3';
    std::fs::write(&f, &b).unwrap();
    std::fs::File::options()
        .write(true)
        .open(&f)
        .unwrap()
        .set_modified(time)
        .unwrap();
    let again = run();
    assert!(again.stderr.contains("index built"), "{}", again.stderr);
    assert_eq!(entries(&c).len(), 2, "one entry per content");
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn damage_that_keeps_the_length_of_an_entry_is_found_by_its_checksum() {
    let (dir, f) = scratch("same-length");
    let (c, fs) = (dir.join("c"), f.display().to_string());
    let cs = c.display().to_string();
    let good = execute(&args(&["file", "items", &fs, "--cache", &cs]));
    let entry = entries(&c).remove(0);
    let mut b = std::fs::read(&entry).unwrap();
    let mid = b.len() / 2;
    b[mid] ^= 0xff;
    std::fs::write(&entry, &b).unwrap();
    let again = execute(&args(&["file", "items", &fs, "--cache", &cs]));
    assert!(
        again.stderr.contains("warn.cache.stale"),
        "{}",
        again.stderr
    );
    assert_eq!(good.stdout, again.stdout);
    let third = execute(&args(&["file", "items", &fs, "--cache", &cs]));
    assert!(third.stderr.contains("index reused"), "{}", third.stderr);
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn what_a_caller_writes_to_its_index_never_reaches_the_entry() {
    let (dir, f) = scratch("copy");
    let c = dir.join("c");
    set(Some(Ctx {
        dir: c.clone(),
        notes: Default::default(),
    }));
    let (a, b) = (dir.join("a.mzidx"), dir.join("b.mzidx"));
    let budget = mzlab_core::budget_auto::native_index_budget();
    through(&f, "", &a, budget).unwrap();
    through(&f, "", &b, budget).unwrap();
    assert!(take_notes().contains("index reused"));
    std::fs::write(&b, b"scribble").unwrap();
    let entry = entries(&c).remove(0);
    assert_eq!(std::fs::read(&a).unwrap(), std::fs::read(&entry).unwrap());
    let c2 = dir.join("c2.mzidx");
    through(&f, "", &c2, budget).unwrap();
    assert!(take_notes().contains("index reused"));
    assert_eq!(std::fs::read(&c2).unwrap(), std::fs::read(&entry).unwrap());
    set(None);
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn scratch_files_of_a_crashed_call_are_swept_and_a_live_one_is_left() {
    let (dir, f) = scratch("sweep");
    let c = dir.join("c");
    std::fs::create_dir_all(&c).unwrap();
    let (old, young) = (c.join(".tmp-1-1"), c.join(".tmp-2-2"));
    std::fs::write(&old, b"x").unwrap();
    std::fs::write(&young, b"x").unwrap();
    std::fs::File::options()
        .write(true)
        .open(&old)
        .unwrap()
        .set_modified(std::time::SystemTime::now() - std::time::Duration::from_secs(7200))
        .unwrap();
    let o = execute(&args(&[
        "file",
        "items",
        &f.display().to_string(),
        "--cache",
        &c.display().to_string(),
    ]));
    assert_eq!(o.code, 0, "{}", o.stderr);
    assert!(!old.exists() && young.exists());
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn file_index_of_an_unknown_sample_is_exit_5() {
    let dir = std::env::temp_dir().join(format!("mzlab-cli-{}-cache-wiff", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    let tools = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tools");
    let made = std::process::Command::new("python3")
        .arg(tools.join("wiff_riferimento.py"))
        .arg(dir.join("in"))
        .output()
        .expect("python3 is needed for the two-sample .wiff fixture");
    assert!(
        made.status.success(),
        "{}",
        String::from_utf8_lossy(&made.stderr)
    );
    let wiff = dir.join("in/two.wiff").display().to_string();
    let cs = dir.join("c").display().to_string();
    let ok = execute(&args(&[
        "file", "index", &wiff, "--sample", "2", "--cache", &cs,
    ]));
    assert_eq!(ok.code, 0, "{}", ok.stderr);
    let bad = execute(&args(&[
        "file", "index", &wiff, "--sample", "9", "--cache", &cs,
    ]));
    assert_eq!(bad.code, 5, "{}", bad.stderr);
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn db_normative_keeps_its_own_cache_flag() {
    let (dir, _) = scratch("normative");
    let c = dir.join("sources").display().to_string();
    // normative's parser answers, not the index cache's ("--cache needs a folder")
    let bare = execute(&args(&["db", "normative", "build", "--cache"]));
    assert_eq!(bare.code, 2);
    assert!(
        bare.stderr.contains("--cache needs a value"),
        "{}",
        bare.stderr
    );
    // its folder reached it (no `--out` is its own usage error) and no index cache was made there
    let o = execute(&args(&["db", "normative", "build", "--cache", &c]));
    assert!(!o.stderr.contains("needs a folder"), "{}", o.stderr);
    assert!(o.stderr.contains("--out"), "{}", o.stderr);
    assert!(!Path::new(&c).exists(), "no index cache folder");
    let _ = std::fs::remove_dir_all(&dir);
}
