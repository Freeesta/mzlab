//! `mzlab-cli quant union` through the real binary: the target list equals the engine's (the function of the app route and of the site's `.wasm`), exit codes tell
//! a bad table (3), usage (2), a missing file (4) and an unwritable output (6) apart, and the list feeds `quant targets`: every cell of a series equals the pass of
//! that series alone, and a target that a file does not hold is an explicit `absent` row.

use std::path::{Path, PathBuf};
use std::process::{Command, Output};

#[path = "../../mzlab-feat/tests/common_quant/mod.rs"]
#[allow(dead_code)]
mod common_quant;
use common_quant::{mzml, Ion};

const SERIES_A: &str = "id,mz,rt,rt_range:min,rt_range:max,charge\n1,200,5,4.8,5.2,1\n2,300,6,5.8,6.2,1\n3,400.3,2,1.8,2.2,1\n";
const SERIES_B: &str =
    "id,mz,rt,rt_range:min,rt_range:max,charge\n1,200,5.1,4.9,5.3,1\n2,500,7,6.8,7.2,1\n";

fn dir(tag: &str) -> PathBuf {
    let d = std::env::temp_dir().join(format!("mzlab_quant_union_{tag}_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).unwrap();
    std::fs::write(d.join("a.csv"), SERIES_A).unwrap();
    std::fs::write(d.join("b.csv"), SERIES_B).unwrap();
    let a = [
        Ion(200.0, 5.0, 50000.0, 0.1),
        Ion(300.0, 6.0, 40000.0, 0.1),
        Ion(400.3, 2.0, 50000.0, 0.1),
    ];
    let b = [Ion(200.0, 5.1, 40000.0, 0.1), Ion(500.0, 7.0, 30000.0, 0.1)];
    std::fs::write(d.join("A.mzML"), mzml(&a, 1)).unwrap();
    std::fs::write(d.join("B.mzML"), mzml(&b, 2)).unwrap();
    d
}

fn cli(args: &[&str], at: &Path) -> Output {
    Command::new(env!("CARGO_BIN_EXE_mzlab-cli"))
        .args(args)
        .current_dir(at)
        .output()
        .unwrap()
}

fn text(b: &[u8]) -> String {
    String::from_utf8_lossy(b).into_owned()
}

fn request(d: &Path) -> String {
    let t = |n: &str| {
        format!(
            "{{\"name\":\"{n}\",\"text\":{}}}",
            mzlab_core::jsonb::write(&mzlab_core::json::Value::Str(
                std::fs::read_to_string(d.join(n)).unwrap()
            ))
        )
    };
    format!(
        "{{\"tables\":[{},{}],\"params\":{{\"rt_tol\":\"0.3\"}}}}",
        t("a.csv"),
        t("b.csv")
    )
}

#[test]
fn the_cli_gives_the_bytes_of_the_engine_as_csv_and_as_json() {
    let d = dir("bytes");
    let doc = mzlab_core::series_union::route(&request(&d)).unwrap();
    let v = mzlab_core::json::parse(doc.as_bytes()).unwrap();
    let o = cli(&["quant", "union", "a.csv", "b.csv", "--rt-tol", "0.3"], &d);
    assert_eq!(o.status.code(), Some(0), "{}", text(&o.stderr));
    assert_eq!(text(&o.stdout), v.get("csv").unwrap().str().unwrap());
    let j = cli(
        &[
            "quant",
            "union",
            "a.csv",
            "b.csv",
            "--set",
            "rt_tol=0.3",
            "--json",
        ],
        &d,
    );
    let body = text(&j.stdout);
    let head = doc.strip_suffix('}').unwrap();
    assert!(
        body.starts_with(head) && body[head.len()..].starts_with(",\"citation\""),
        "{body}"
    );
    // 5 rows, the 200 of both series is one target
    assert!(
        text(&o.stderr)
            .contains("5 rows of 2 series -> 4 targets (1 of them from more than one series)"),
        "{}",
        text(&o.stderr)
    );
    let out = cli(
        &[
            "quant", "union", "a.csv", "b.csv", "--rt-tol", "0.3", "--out", "u.csv",
        ],
        &d,
    );
    assert!(out.stdout.is_empty());
    assert_eq!(
        std::fs::read_to_string(d.join("u.csv")).unwrap(),
        text(&o.stdout)
    );
}

#[test]
fn exit_codes_tell_the_failures_apart() {
    let d = dir("exits");
    std::fs::write(d.join("bad.csv"), "id,mz,rt\nx,abc,1\n").unwrap();
    let code = |args: &[&str]| cli(args, &d).status.code();
    assert_eq!(code(&["quant", "union", "bad.csv"]), Some(3));
    assert_eq!(code(&["quant", "union", "a.csv", "--da", "9"]), Some(2));
    assert_eq!(code(&["quant", "union", "a.csv", "--bogus"]), Some(2));
    assert_eq!(code(&["quant", "union"]), Some(2));
    assert_eq!(code(&["quant", "union", "missing.csv"]), Some(4));
    assert_eq!(
        code(&["quant", "union", "a.csv", "--out", "no/such/dir/u.csv"]),
        Some(6)
    );
    assert_eq!(
        code(&["quant", "union", "a.csv", "--out", "a.csv"]),
        Some(2),
        "an input is never overwritten"
    );
    let e = text(&cli(&["quant", "union", "bad.csv"], &d).stderr);
    assert!(
        e.contains("Row 2 of the table bad.csv") && e.contains("bad_mz"),
        "{e}"
    );
}

fn rows(csv: &str) -> Vec<Vec<String>> {
    csv.lines()
        .skip(1)
        .map(|l| l.split(',').map(String::from).collect())
        .collect()
}

#[test]
fn the_union_then_quant_targets_gives_the_cells_of_each_series_alone() {
    let d = dir("chain");
    let u = cli(
        &[
            "quant",
            "union",
            "a.csv",
            "b.csv",
            "--rt-tol",
            "0.3",
            "--out",
            "union.csv",
        ],
        &d,
    );
    assert_eq!(u.status.code(), Some(0), "{}", text(&u.stderr));
    // series A alone: its own table as targets (same m/z and window)
    std::fs::write(
        d.join("own.csv"),
        "id,mz,rt_min,rt_max\na1,200,4.8,5.2\na2,300,5.8,6.2\na3,400.3,1.8,2.2\n",
    )
    .unwrap();
    let alone = cli(&["quant", "targets", "--targets", "own.csv", "A.mzML"], &d);
    let all = cli(
        &[
            "quant",
            "targets",
            "--targets",
            "union.csv",
            "A.mzML",
            "B.mzML",
        ],
        &d,
    );
    assert_eq!(
        (alone.status.code(), all.status.code()),
        (Some(0), Some(0)),
        "{}",
        text(&all.stderr)
    );
    let (alone, all) = (rows(&text(&alone.stdout)), rows(&text(&all.stdout)));
    assert_eq!(all.len(), 2 * 4, "every file x every target, none hidden");
    let union = rows(&std::fs::read_to_string(d.join("union.csv")).unwrap());
    let id_of = |mz: &str| union.iter().find(|r| r[1] == mz).unwrap()[0].clone();
    for (a, mz) in alone.iter().zip(["200", "300", "400.3"]) {
        let cell = all
            .iter()
            .find(|r| r[0] == "A.mzML" && r[1] == id_of(mz))
            .unwrap();
        // area, height, sn, rt, flag, state, points, shape: the same numbers (the window start/end columns follow the window)
        assert_eq!(a[2..10], cell[2..10], "{mz}");
    }
    let state = |file: &str, mz: &str| {
        all.iter()
            .find(|r| r[0] == file && r[1] == id_of(mz))
            .unwrap()[7]
            .clone()
    };
    assert_eq!(
        (
            state("A.mzML", "500").as_str(),
            state("B.mzML", "300").as_str()
        ),
        ("absent", "absent")
    );
    assert_eq!(
        (
            state("A.mzML", "200").as_str(),
            state("B.mzML", "200").as_str()
        ),
        ("detected", "detected")
    );
}
