//! `mzlab-cli quant targets` through the real binary on synthetic mzML files: the CSV is the engine's table byte for byte, `--json` holds the
//! same rows, `--dry-run` reads no data, a folder is read in name order, and the exit codes tell usage (2), input (4) and output (6) apart.

use std::path::{Path, PathBuf};
use std::process::{Command, Output};

#[path = "../../mzlab-feat/tests/common_quant/mod.rs"]
#[allow(dead_code)]
mod common_quant;
use common_quant::{mzml, sample, Ion, TARGETS};

fn dir(tag: &str) -> PathBuf {
    let d = std::env::temp_dir().join(format!("mzlab_quant_targets_{tag}_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(d.join("runs")).unwrap();
    std::fs::write(d.join("targets.csv"), TARGETS).unwrap();
    std::fs::write(d.join("runs/S_01.mzML"), mzml(&sample(), 1)).unwrap();
    let blank = [Ion(200.0, 5.0, 3000.0, 0.1), Ion(400.3, 2.0, 50000.0, 0.1)];
    std::fs::write(d.join("runs/Blank_01.mzML"), mzml(&blank, 2)).unwrap();
    d
}

fn cli(args: &[&str], at: &Path) -> Output {
    Command::new(env!("CARGO_BIN_EXE_mzlab-cli"))
        .args(["quant", "targets"])
        .args(args)
        .current_dir(at)
        .output()
        .unwrap()
}

fn text(b: &[u8]) -> String {
    String::from_utf8_lossy(b).into_owned()
}

/// The run on the folder `runs` with `offset auto` and the blank pattern, plus `extra`.
fn folder(d: &Path, extra: &[&str]) -> Output {
    let mut a = vec![
        "--targets",
        "targets.csv",
        "runs",
        "--offset",
        "auto",
        "--blank-pattern",
        "blank*",
    ];
    a.extend_from_slice(extra);
    cli(&a, d)
}

#[test]
fn a_folder_gives_one_row_per_file_and_target_with_its_flags() {
    let d = dir("csv");
    let o = folder(&d, &["--log", "log.json"]);
    assert_eq!(o.status.code(), Some(0), "{}", text(&o.stderr));
    let csv = text(&o.stdout);
    let lines: Vec<&str> = csv.lines().collect();
    assert_eq!(lines[0], mzlab_feat::quant_targets::HEADER);
    assert_eq!(lines.len(), 1 + 2 * 6);
    // folder files in name order: the blank first
    assert!(lines[1].starts_with("Blank_01.mzML,clean,"));
    let clean = lines
        .iter()
        .find(|l| l.starts_with("S_01.mzML,clean,"))
        .unwrap();
    assert!(clean.contains(",in_blank,"), "{clean}");
    let empty = lines
        .iter()
        .find(|l| l.starts_with("S_01.mzML,empty,"))
        .unwrap();
    assert!(!empty.contains(",ok,"), "{empty}");
    let stderr = text(&o.stderr);
    assert!(
        stderr.contains("[1/2] Blank_01.mzML") && stderr.contains("[2/2] S_01.mzML"),
        "{stderr}"
    );
    let log = std::fs::read_to_string(d.join("log.json")).unwrap();
    assert!(log.starts_with("{\"schema\":\"quant-targets-log/1\""));
    assert!(log.contains("\"reference\":\"std\""), "{log}");

    // the same rows as JSON, the CSV unchanged by --out
    let j = folder(&d, &["--json"]);
    assert_eq!(j.status.code(), Some(0));
    let json = text(&j.stdout);
    assert_eq!(
        json.trim_end(),
        mzlab_feat::quant_targets::to_json(&csv, &log)
    );
    assert!(
        json.starts_with("{\"schema\":\"quant-targets/1\",\"rows\":[{\"file\":\"Blank_01.mzML\"")
    );
    let w = folder(&d, &["--out", "t.csv"]);
    assert_eq!(w.status.code(), Some(0));
    assert_eq!(std::fs::read_to_string(d.join("t.csv")).unwrap(), csv);
    schema_lists_the_keys(&json);
    let _ = std::fs::remove_dir_all(&d);
}

/// The keys of every row and of the log are exactly those `docs/schemi/quant-targets-1.json` requires.
fn schema_lists_the_keys(json: &str) {
    let schema: serde_json::Value =
        serde_json::from_str(include_str!("../../../docs/schemi/quant-targets-1.json")).unwrap();
    let doc: serde_json::Value = serde_json::from_str(json).unwrap();
    let keys =
        |o: &serde_json::Value| -> Vec<String> { o.as_object().unwrap().keys().cloned().collect() };
    let required = |s: &serde_json::Value| -> Vec<String> {
        let mut v: Vec<String> = s["required"]
            .as_array()
            .unwrap()
            .iter()
            .map(|k| k.as_str().unwrap().to_string())
            .collect();
        v.sort();
        v
    };
    let sorted = |mut v: Vec<String>| {
        v.sort();
        v
    };
    let p = &schema["properties"];
    assert_eq!(sorted(keys(&doc)), required(&schema));
    for r in doc["rows"].as_array().unwrap() {
        assert_eq!(sorted(keys(r)), required(&p["rows"]["items"]));
    }
    assert_eq!(sorted(keys(&doc["log"])), required(&p["log"]));
    assert_eq!(
        sorted(keys(&doc["log"]["params"])),
        required(&p["log"]["properties"]["params"])
    );
}

#[test]
fn a_dry_run_reads_no_data_and_errors_have_their_codes() {
    let d = dir("codes");
    let p = cli(
        &[
            "--targets",
            "targets.csv",
            "runs/S_01.mzML",
            "nothing.mzML",
            "--dry-run",
        ],
        &d,
    );
    assert_eq!(p.status.code(), Some(4), "a missing file is an input error");
    let p = cli(
        &["--targets", "targets.csv", "runs/S_01.mzML", "--dry-run"],
        &d,
    );
    assert_eq!(p.status.code(), Some(0), "{}", text(&p.stderr));
    assert!(text(&p.stdout).starts_with("{\"schema\":\"quant-targets-plan/1\""));
    assert!(text(&p.stdout).trim_end().ends_with("\"rows\":6}"));
    assert_eq!(cli(&["runs"], &d).status.code(), Some(2), "no --targets");
    assert_eq!(
        cli(&["--targets", "targets.csv", "runs", "--min-sn", "x"], &d)
            .status
            .code(),
        Some(2)
    );
    std::fs::write(d.join("bad.csv"), "id,mz\nx,1\n").unwrap();
    let bad = cli(&["--targets", "bad.csv", "runs"], &d);
    assert_eq!(bad.status.code(), Some(4), "{}", text(&bad.stderr));
    assert!(text(&bad.stderr).contains("err.quant.targets.table"));
    std::fs::create_dir_all(d.join("none")).unwrap();
    assert_eq!(
        cli(&["--targets", "targets.csv", "none"], &d).status.code(),
        Some(4)
    );
    let out = cli(
        &[
            "--targets",
            "targets.csv",
            "runs/S_01.mzML",
            "--out",
            "no/such/dir/t.csv",
        ],
        &d,
    );
    assert_eq!(out.status.code(), Some(6));
    let _ = std::fs::remove_dir_all(&d);
}

#[test]
fn the_bytes_are_the_same_whatever_the_workers() {
    let d = dir("workers");
    for k in 2..8u64 {
        let mut ions = sample();
        ions[0].2 = 2000.0 * k as f64;
        std::fs::write(d.join(format!("runs/S_{k:02}.mzML")), mzml(&ions, k)).unwrap();
    }
    let args = |w: &'static str| {
        let mut a = vec![
            "--targets",
            "targets.csv",
            "runs",
            "--offset",
            "auto",
            "--blank-pattern",
            "blank*",
            "--workers",
            w,
            "--log",
        ];
        a.push(if w == "1" { "one.json" } else { "many.json" });
        a
    };
    let one = cli(&args("1"), &d);
    let many = cli(&args("4"), &d);
    assert_eq!(one.status.code(), Some(0), "{}", text(&one.stderr));
    assert_eq!(many.status.code(), Some(0), "{}", text(&many.stderr));
    assert!(
        text(&many.stderr).contains("note: 8 files on 4 workers"),
        "{}",
        text(&many.stderr)
    );
    assert_eq!(text(&one.stdout).lines().count(), 1 + 8 * 6);
    assert_eq!(
        one.stdout, many.stdout,
        "the table does not depend on the workers"
    );
    assert_eq!(
        std::fs::read(d.join("one.json")).unwrap(),
        std::fs::read(d.join("many.json")).unwrap()
    );
    assert_eq!(
        cli(&["--targets", "targets.csv", "runs", "--workers", "0"], &d)
            .status
            .code(),
        Some(2)
    );
    let _ = std::fs::remove_dir_all(&d);
}
