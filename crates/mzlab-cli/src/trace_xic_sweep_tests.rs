//! `trace xic --sweep LO:HI:STEP`: the bytes of `--mz` with the expanded list, the default half-window, the errors and the memory budget.

use mzlab_core::json::parse;

use crate::extract::tests::{mzml_file, run};

fn text(o: crate::Outcome) -> String {
    assert_eq!(o.code, 0, "{}", o.stderr);
    String::from_utf8(o.stdout).unwrap()
}

/// The list `--sweep LO:HI:STEP` stands for, as `--mz` takes it: `LO + i*STEP` with the decimals of STEP.
fn expanded(lo: f64, hi: f64, step: f64, dec: usize) -> String {
    let n = ((hi - lo) / step + 1e-9).floor() as usize;
    (0..=n)
        .map(|i| format!("{:.dec$}", lo + i as f64 * step))
        .collect::<Vec<_>>()
        .join(",")
}

#[test]
fn a_sweep_is_the_bytes_of_the_list_it_expands_to() {
    let (d, f) = mzml_file("xic_sweep");
    let cases: [(&str, String, [&str; 2]); 4] = [
        (
            "100:300:1",
            expanded(100.0, 300.0, 1.0, 0),
            ["--tol", "0.5da"],
        ),
        (
            "100:110:0.3",
            expanded(100.0, 110.0, 0.3, 1),
            ["--tol", "0.5da"],
        ),
        (
            "200:201:0.01",
            expanded(200.0, 201.0, 0.01, 2),
            ["--tol", "10ppm"],
        ),
        (
            "149:158:0.5",
            expanded(149.0, 158.0, 0.5, 1),
            ["--ppm", "3000"],
        ),
    ];
    for (spec, list, tol) in &cases {
        for fmt in ["json", "csv"] {
            for extra in [
                &[][..],
                &["--smooth", "5"],
                &["--rt", "1,3"],
                &["--smooth", "5", "--rt", "1,3"],
            ] {
                let run_with = |what: [&str; 2]| {
                    let mut v = vec![
                        "trace",
                        "xic",
                        f.as_str(),
                        what[0],
                        what[1],
                        tol[0],
                        tol[1],
                        "--format",
                        fmt,
                    ];
                    v.extend_from_slice(extra);
                    run(&v)
                };
                let (a, b) = (run_with(["--sweep", spec]), run_with(["--mz", list]));
                assert_eq!((a.code, b.code), (0, 0), "{spec} {}", a.stderr);
                assert_eq!(a.stdout, b.stdout, "{spec} {fmt} {extra:?}");
                assert_eq!(a.stderr, b.stderr, "the notes of {spec} {fmt} {extra:?}");
            }
        }
    }
    // the sweep with a signal is not all zero, and a list of two or more is the sweep of its engine too (the order given is kept)
    let some = text(run(&["trace", "xic", &f, "--sweep", "149:158:1"]));
    assert!(some.contains("\"n_traces\":10"), "{some}");
    assert!(
        some.contains("\"empty\":true") && some.matches("\"empty\":true").count() < 10,
        "{some}"
    );
    let rev = text(run(&[
        "trace",
        "xic",
        &f,
        "--mz",
        "257.3,241.3,150",
        "--da",
        "0.4",
    ]));
    for mz in ["257.3", "241.3", "150"] {
        let one = text(run(&["trace", "xic", &f, "--mz", mz, "--da", "0.4"]));
        assert!(rev.contains(one.trim()), "{mz}");
    }
    assert!(rev.find("xic:257.3").unwrap() < rev.find("xic:241.3").unwrap());
    let _ = std::fs::remove_dir_all(d);
}

#[test]
fn a_sweep_without_a_tolerance_takes_half_its_step_and_says_so() {
    let (d, f) = mzml_file("xic_sweep_tol");
    let o = run(&["trace", "xic", &f, "--sweep", "149:158:1"]);
    assert_eq!(o.code, 0, "{}", o.stderr);
    assert!(
        o.stderr.contains("note: --tol defaults to STEP/2 = 0.5 Da"),
        "{}",
        o.stderr
    );
    let with = run(&["trace", "xic", &f, "--sweep", "149:158:1", "--tol", "0.5da"]);
    assert_eq!(o.stdout, with.stdout);
    assert!(!with.stderr.contains("defaults"), "{}", with.stderr);
    let plan = text(run(&[
        "trace",
        "xic",
        &f,
        "--sweep",
        "149:158:1",
        "--dry-run",
    ]));
    let v = parse(plan.trim().as_bytes()).unwrap();
    let s = v.get("sweep").unwrap();
    assert_eq!(
        (
            s.get("lo").unwrap().f64(),
            s.get("hi").unwrap().f64(),
            s.get("step").unwrap().f64()
        ),
        (Some(149.0), Some(158.0), Some(1.0))
    );
    assert_eq!(s.get("n").unwrap().f64(), Some(10.0));
    assert_eq!(s.get("tol_default").unwrap().bool(), Some(true));
    assert_eq!(v.get("matrix_bytes_per_scan").unwrap().f64(), Some(80.0));
    assert_eq!(v.get("n_traces").unwrap().f64(), Some(10.0));
    assert_eq!(
        v.get("traces").unwrap().arr()[1]
            .get("tolerance_da")
            .unwrap()
            .f64(),
        Some(0.5)
    );
    let o = run(&["trace", "xic", &f, "--sweep", "149:158:1", "--dry-run"]);
    assert!(o.stderr.contains("defaults to STEP/2"), "{}", o.stderr);
    let _ = std::fs::remove_dir_all(d);
}

#[test]
fn the_sweep_errors_are_distinct_exit_codes() {
    let (d, f) = mzml_file("xic_sweep_err");
    let code = |a: &[&str]| {
        let mut v = vec!["trace", "xic", f.as_str()];
        v.extend_from_slice(a);
        run(&v)
    };
    for bad in [
        vec!["--sweep", "100:110:1", "--mz", "100", "--da", "0.2"],
        vec!["--da", "0.2"],
        vec!["--sweep", "1:0:1"],
        vec!["--sweep", "a:b:c"],
        vec!["--sweep", "100:110:0"],
        vec!["--sweep", "1:30000:1"],
    ] {
        assert_eq!(code(&bad).code, 2, "{bad:?}");
    }
    let o = code(&["--sweep", "100:110:1", "--mz", "100"]);
    assert!(o.stderr.contains("err.xic.sweep.mz"), "{}", o.stderr);
    let o = code(&["--sweep", "1:30000:1"]);
    assert!(o.stderr.contains("err.xic.sweep.size"), "{}", o.stderr);
    // 20000 centres is the largest sweep and it is not refused for its size
    assert_eq!(code(&["--sweep", "1:20000:1", "--dry-run"]).code, 0);
    assert_eq!(code(&["--sweep", "1:20001:1", "--dry-run"]).code, 2);
    // a missing file under --dry-run is exit 4
    let gone = d.join("gone.mzML");
    let o = run(&[
        "trace",
        "xic",
        &gone.to_string_lossy(),
        "--sweep",
        "100:110:1",
        "--dry-run",
    ]);
    assert_eq!(o.code, 4, "{}", o.stderr);
    let _ = std::fs::remove_dir_all(d);
}

#[test]
fn a_sweep_of_an_unknown_sample_is_exit_5() {
    let dir = std::env::temp_dir().join(format!("mzlab_xic_sweep_wiff_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    let tools = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tools");
    let made = std::process::Command::new("python3")
        .arg(tools.join("wiff_riferimento.py"))
        .arg(dir.join("in"))
        .output();
    if !made.is_ok_and(|o| o.status.success()) {
        assert!(
            std::env::var("MZLAB_GOLDEN_STRICT").is_err(),
            "python3 not available"
        );
        eprintln!("skipped: python3 not available");
        return;
    }
    let wiff = dir.join("in/two.wiff").to_string_lossy().into_owned();
    let bad = run(&[
        "trace",
        "xic",
        &wiff,
        "--sweep",
        "100:110:1",
        "--sample",
        "9",
    ]);
    assert_eq!(bad.code, 5, "{}", bad.stderr);
    assert!(bad.stderr.contains("err.sample.unknown"), "{}", bad.stderr);
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn a_matrix_over_the_budget_is_exit_1_with_its_phase_named() {
    // the engine refuses before building (tests/xic_sweep.rs of mzlab-core); here the refusal is a data error (1), its text names the phase
    let e = mzlab_core::budget::refusal("xicSweep", 3 << 30, 1 << 30);
    let o = crate::Outcome::fail_error(&e);
    assert_eq!(o.code, 1, "{}", o.stderr);
    assert!(
        o.stderr.contains("sweep") && o.stderr.contains("error.memoryBudget"),
        "{}",
        o.stderr
    );
}
