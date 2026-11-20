//! The index cache seen from outside: real processes, the environment variable and the flag, and nothing written when no cache was asked for.

use std::path::{Path, PathBuf};
use std::process::Command;

const BIN: &str = env!("CARGO_BIN_EXE_mzlab-cli");

/// A folder of its own for the test, with a synthetic mzML (the generator of the lab tests) in it.
fn fixture(tag: &str) -> (PathBuf, PathBuf) {
    let dir = std::env::temp_dir().join(format!("mzlab-cli-{}-procs-{tag}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let tools = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tools");
    let script = "import sys, numpy as np; sys.path.insert(0, sys.argv[1]); from pathlib import Path; \
                  import dati_sintetici as d; d.full_scan(Path(sys.argv[2]), 30, np.random.default_rng(1))";
    let f = dir.join("s.mzML");
    let made = Command::new("python3")
        .args(["-c", script])
        .arg(&tools)
        .arg(&f)
        .output()
        .expect("python3 with numpy makes the synthetic mzML");
    assert!(
        made.status.success(),
        "{}",
        String::from_utf8_lossy(&made.stderr)
    );
    (dir, f)
}

fn run(env: &[(&str, &Path)], args: &[&str]) -> std::process::Output {
    let mut c = Command::new(BIN);
    c.env_remove("MZLAB_CACHE").args(args);
    for (k, v) in env {
        c.env(k, v);
    }
    c.output().unwrap()
}

fn err(o: &std::process::Output) -> String {
    String::from_utf8_lossy(&o.stderr).into_owned()
}

fn count(dir: &Path) -> usize {
    std::fs::read_dir(dir)
        .map(|d| {
            d.filter(|e| {
                e.as_ref()
                    .unwrap()
                    .file_name()
                    .to_string_lossy()
                    .ends_with(".mzidx")
            })
            .count()
        })
        .unwrap_or(0)
}

#[test]
fn the_variable_serves_and_the_flag_wins_over_it() {
    let (dir, f) = fixture("env");
    let (from_env, from_flag) = (dir.join("env"), dir.join("flag"));
    let fs = f.to_str().unwrap();
    let a = run(&[("MZLAB_CACHE", &from_env)], &["file", "items", fs]);
    assert!(err(&a).contains("index built"), "{}", err(&a));
    let b = run(&[("MZLAB_CACHE", &from_env)], &["file", "items", fs]);
    assert!(err(&b).contains("index reused"), "{}", err(&b));
    let c = run(
        &[("MZLAB_CACHE", &from_env)],
        &["file", "items", fs, "--cache", from_flag.to_str().unwrap()],
    );
    assert!(err(&c).contains("index built"), "{}", err(&c));
    assert_eq!((count(&from_env), count(&from_flag)), (1, 1));
    assert_eq!(a.stdout, c.stdout);
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn two_processes_on_the_same_entry_both_answer_and_leave_one_sound_entry() {
    let (dir, f) = fixture("race");
    let cache = dir.join("c");
    let fs = f.to_str().unwrap().to_string();
    let go = |cache: PathBuf| {
        let fs = fs.clone();
        std::thread::spawn(move || {
            run(
                &[],
                &["file", "items", &fs, "--cache", cache.to_str().unwrap()],
            )
        })
    };
    let (a, b) = (go(cache.clone()), go(cache.clone()));
    let (a, b) = (a.join().unwrap(), b.join().unwrap());
    assert!(
        a.status.success() && b.status.success(),
        "{} {}",
        err(&a),
        err(&b)
    );
    assert_eq!(a.stdout, b.stdout);
    assert_eq!(count(&cache), 1);
    let again = run(
        &[],
        &["file", "items", &fs, "--cache", cache.to_str().unwrap()],
    );
    assert!(err(&again).contains("index reused"), "{}", err(&again));
    assert_eq!(again.stdout, a.stdout);
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn nothing_is_written_anywhere_when_no_cache_was_asked_for() {
    let (dir, f) = fixture("none");
    let home = dir.join("home");
    std::fs::create_dir_all(&home).unwrap();
    let env = [
        ("HOME", home.as_path()),
        ("XDG_CACHE_HOME", home.as_path()),
        ("TMPDIR", home.as_path()),
    ];
    let o = run(&env, &["file", "items", f.to_str().unwrap()]);
    assert!(o.status.success(), "{}", err(&o));
    assert!(!err(&o).contains("index "), "{}", err(&o));
    let left: Vec<_> = std::fs::read_dir(&home).unwrap().collect();
    assert!(left.is_empty(), "left behind: {left:?}");
    let _ = std::fs::remove_dir_all(&dir);
}
