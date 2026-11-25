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
    same_rows(&csv, &json, &log);
    assert!(
        json.starts_with("{\"schema\":\"quant-targets/1\",\"rows\":[{\"file\":\"Blank_01.mzML\"")
    );
    let w = folder(&d, &["--out", "t.csv"]);
    assert_eq!(w.status.code(), Some(0));
    assert_eq!(std::fs::read_to_string(d.join("t.csv")).unwrap(), csv);
    schema_lists_the_keys(&json);
    let _ = std::fs::remove_dir_all(&d);
}

/// Every row of `--json` is the row of the CSV, cell for cell (an empty cell is `null`, a number the same number), and its log is the log.
fn same_rows(csv: &str, json: &str, log: &str) {
    let doc: serde_json::Value = serde_json::from_str(json).unwrap();
    let rows = doc["rows"].as_array().unwrap();
    let mut lines = csv.lines();
    let keys: Vec<&str> = lines.next().unwrap().split(',').collect();
    let lines: Vec<&str> = lines.collect();
    assert_eq!(rows.len(), lines.len());
    for (row, line) in rows.iter().zip(&lines) {
        for (k, cell) in keys.iter().zip(line.split(',')) {
            let v = &row[*k];
            match cell.parse::<f64>() {
                _ if cell.is_empty() => assert!(v.is_null(), "{k} {line}"),
                Ok(n) if !matches!(*k, "file" | "id") => {
                    assert_eq!(v.as_f64(), Some(n), "{k} {line}")
                }
                _ => assert_eq!(v.as_str(), Some(cell), "{k} {line}"),
            }
        }
    }
    let l: serde_json::Value = serde_json::from_str(log).unwrap();
    assert_eq!(doc["log"], l);
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

fn find_index(dir: &Path) -> Option<PathBuf> {
    for e in std::fs::read_dir(dir).ok()?.flatten() {
        let p = e.path();
        if p.is_dir() {
            if let Some(f) = find_index(&p) {
                return Some(f);
            }
        } else if p.extension().is_some_and(|x| x == "mzidx") {
            return Some(p);
        }
    }
    None
}

#[test]
fn two_files_of_one_name_are_refused_and_an_index_beside_its_source_is_skipped() {
    let d = dir("names");
    // the same file name in two subfolders: the rows could not tell them apart
    std::fs::create_dir_all(d.join("runs/b")).unwrap();
    std::fs::copy(d.join("runs/S_01.mzML"), d.join("runs/b/S_01.mzML")).unwrap();
    let twice = cli(&["--targets", "targets.csv", "runs"], &d);
    assert_eq!(twice.status.code(), Some(4), "{}", text(&twice.stderr));
    assert!(
        text(&twice.stderr).contains("err.cli.input.twice"),
        "{}",
        text(&twice.stderr)
    );
    std::fs::remove_dir_all(d.join("runs/b")).unwrap();

    // the index of S_01.mzML beside it: integrated once, from the mzML
    let built = Command::new(env!("CARGO_BIN_EXE_mzlab-cli"))
        .args(["file", "index", "runs/S_01.mzML", "--cache", "cache"])
        .current_dir(&d)
        .output()
        .unwrap();
    assert_eq!(built.status.code(), Some(0), "{}", text(&built.stderr));
    std::fs::copy(
        find_index(&d.join("cache")).unwrap(),
        d.join("runs/S_01.mzidx"),
    )
    .unwrap();
    let o = cli(&["--targets", "targets.csv", "runs"], &d);
    assert_eq!(o.status.code(), Some(0), "{}", text(&o.stderr));
    assert_eq!(
        text(&o.stdout).lines().count(),
        1 + 2 * 6,
        "{}",
        text(&o.stdout)
    );
    assert!(
        text(&o.stderr).contains("S_01.mzidx skipped"),
        "{}",
        text(&o.stderr)
    );
    let _ = std::fs::remove_dir_all(&d);
}

#[test]
fn more_workers_than_the_cap_are_refused() {
    let d = dir("cap");
    let o = cli(&["--targets", "targets.csv", "runs", "--workers", "9"], &d);
    assert_eq!(o.status.code(), Some(2), "{}", text(&o.stderr));
    assert!(text(&o.stderr).contains("1 to 8"), "{}", text(&o.stderr));
    let _ = std::fs::remove_dir_all(&d);
}

/// One `mzlab-cli` command in `d`: its stdout, exit 0 asserted.
fn mzlab(d: &Path, args: &[&str]) -> Vec<u8> {
    let o = Command::new(env!("CARGO_BIN_EXE_mzlab-cli"))
        .args(args)
        .current_dir(d)
        .output()
        .unwrap();
    assert_eq!(o.status.code(), Some(0), "{args:?} {}", text(&o.stderr));
    o.stdout
}

/// The tallest peak `peak auto-lr` finds between `lo` and `hi` on the `trace xic` of `mz` (+-0.2 Da, positive scans) of runs/S_01.mzML, with
/// the step's settings: `lr_sn=0 lr_min_pts=2` (`peak auto-lr` itself defaults to S/N 5 and 5 points), valley 10 %, no smoothing.
fn tallest_by_the_commands(d: &Path, mz: &str, lo: &str, hi: &str) -> Option<serde_json::Value> {
    let xic = mzlab(
        d,
        &[
            "trace",
            "xic",
            "runs/S_01.mzML",
            "--mz",
            mz,
            "--da",
            "0.2",
            "--polarity",
            "+",
            "--format",
            "csv",
        ],
    );
    std::fs::write(d.join("x.csv"), xic).unwrap();
    let set = [
        "--set",
        "lr_sn=0",
        "--set",
        "lr_min_pts=2",
        "--set",
        "lr_valley=10",
        "--set",
        "lr_half=0",
    ];
    let mut args = vec!["peak", "auto-lr", "x.csv", "--from", lo, "--to", hi];
    args.extend(set);
    let lr: serde_json::Value = serde_json::from_slice(&mzlab(d, &args)).unwrap();
    let h = |p: &serde_json::Value| p["height"].as_f64().unwrap();
    lr["peaks"]
        .as_array()
        .unwrap()
        .iter()
        .max_by(|a, b| h(a).total_cmp(&h(b)))
        .cloned()
}

/// Every area, height and apex RT of the table is what the real routes give for that target: `trace xic` of the file into `peak auto-lr` over
/// the window with the step's settings (every peak kept, then judged by `min_sn` and the shape), tallest peak of the window.
#[test]
fn areas_are_those_of_the_trace_xic_and_peak_auto_lr_commands() {
    let d = dir("routes");
    let table = text(&mzlab(
        &d,
        &[
            "quant",
            "targets",
            "--targets",
            "targets.csv",
            "runs/S_01.mzML",
            "--da",
            "0.2",
        ],
    ));
    let mut checked = 0;
    for t in TARGETS.lines().skip(1) {
        let c: Vec<&str> = t.split(',').collect();
        let id = c[0];
        let row: Vec<&str> = table
            .lines()
            .find(|l| l.starts_with(&format!("S_01.mzML,{id},")))
            .unwrap()
            .split(',')
            .collect();
        let Some(p) = tallest_by_the_commands(&d, c[1], c[2], c[3]) else {
            assert_eq!(row[2], "", "{id}: no peak, no area");
            continue;
        };
        for (cell, key, tol) in [
            (row[2], "area", 0.05),
            (row[3], "height", 0.05),
            (row[5], "rt", 1e-9),
        ] {
            let v = p[key].as_f64().unwrap();
            assert!(
                (cell.parse::<f64>().unwrap() - v).abs() <= tol,
                "{id} {key} {cell} vs {v}"
            );
        }
        checked += 1;
    }
    assert_eq!(
        checked, 3,
        "clean, small and edge have a peak; std and shifted are 0.3 Da off without an offset"
    );
    let _ = std::fs::remove_dir_all(&d);
}
