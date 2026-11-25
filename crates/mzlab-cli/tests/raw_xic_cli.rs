//! `mzlab-cli raw xic` through the real binary on the synthetic mzML of the `quant targets` tests: area, height, S/N and the rest of a `peak` row
//! are the cells `quant targets` writes for the same file, mass and window (one integrator); the exits are pinned, `--dry-run` reads no peak and
//! the output is the same with `--json`. Weak whole-count data gets its verdict with the counts behind it.

use std::path::{Path, PathBuf};
use std::process::{Command, Output};

#[path = "../../mzlab-feat/tests/common_quant/mod.rs"]
#[allow(dead_code)]
mod common_quant;
use common_quant::{mzml, sample, sparse, Ion};

fn dir(tag: &str) -> PathBuf {
    let d = std::env::temp_dir().join(format!("mzlab_raw_xic_{tag}_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).unwrap();
    std::fs::write(d.join("S_01.mzML"), mzml(&sample(), 1)).unwrap();
    d
}

fn cli(args: &[&str], at: &Path) -> Output {
    Command::new(env!("CARGO_BIN_EXE_mzlab-cli"))
        .args(args)
        .current_dir(at)
        .output()
        .unwrap()
}

fn doc(o: &Output) -> serde_json::Value {
    serde_json::from_slice(&o.stdout)
        .unwrap_or_else(|e| panic!("{e}: {}", String::from_utf8_lossy(&o.stdout)))
}

fn code(o: &Output) -> i32 {
    o.status.code().unwrap()
}

/// The `quant targets` row of `id` in `S_01.mzML` for a target `mz`, `rt_min`..`rt_max`.
fn quant_row(d: &Path, id: &str, mz: f64, a: f64, b: f64) -> serde_json::Value {
    std::fs::write(
        d.join("t.csv"),
        format!("id,mz,rt_min,rt_max,polarity,role\n{id},{mz},{a},{b},+,\n"),
    )
    .unwrap();
    let o = cli(
        &[
            "quant",
            "targets",
            "--targets",
            "t.csv",
            "S_01.mzML",
            "--json",
        ],
        d,
    );
    assert_eq!(code(&o), 0, "{}", String::from_utf8_lossy(&o.stderr));
    doc(&o)["rows"][0].clone()
}

#[test]
fn a_peak_row_has_the_numbers_of_quant_targets() {
    let d = dir("parity");
    for (id, mz, a, b) in [
        ("clean", 200.0, 4.0, 6.0),
        ("small", 220.0, 4.0, 6.0),
        ("edge", 240.0, 4.0, 6.0),
        ("std", 400.0, 1.5, 2.5),
    ] {
        let q = quant_row(&d, id, mz, a, b);
        let o = cli(
            &[
                "raw",
                "xic",
                "S_01.mzML",
                "--mz",
                &mz.to_string(),
                "--rt",
                &format!("{a},{b}"),
            ],
            &d,
        );
        assert_eq!(code(&o), 0, "{}", String::from_utf8_lossy(&o.stderr));
        let r = &doc(&o)["rows"][0];
        assert_ne!(r["verdict"], "no_data", "{id}");
        for (x, y) in [
            ("area", "area"),
            ("height", "height"),
            ("sn", "sn"),
            ("rt", "apex_rt"),
            ("points", "points"),
            ("shape", "shape"),
            ("start_rt", "start_rt"),
            ("end_rt", "end_rt"),
        ] {
            assert_eq!(q[x], r[y], "{id} {x}: quant {} raw xic {}", q[x], r[y]);
        }
    }
    // the window holding no ion: the same absence
    let q = quant_row(&d, "empty", 300.0, 4.0, 6.0);
    let o = cli(
        &["raw", "xic", "S_01.mzML", "--mz", "300", "--rt", "4,6"],
        &d,
    );
    let r = &doc(&o)["rows"][0];
    assert!(q["area"].is_null(), "quant targets: {}", q["area"]);
    assert!(r["area"].is_null(), "raw xic: {}", r["area"]);
    assert_eq!(r["verdict"], "nothing");
    let _ = std::fs::remove_dir_all(&d);
}

#[test]
fn weak_whole_counts_have_their_counts_and_the_rule_that_decided() {
    let d = dir("counts");
    let ions = [Ion(200.0, 5.0, 12.0, 0.1), Ion(220.0, 5.0, 1.0, 0.03)];
    std::fs::write(d.join("W.mzML"), sparse(&ions, 180.0, 3)).unwrap();
    let o = cli(
        &[
            "raw",
            "xic",
            "W.mzML",
            "--mz",
            "200,220,260",
            "--rt",
            "3,7",
            "--da",
            "0.5",
        ],
        &d,
    );
    assert_eq!(code(&o), 0, "{}", String::from_utf8_lossy(&o.stderr));
    let v = doc(&o);
    let rows = v["rows"].as_array().unwrap();
    assert_eq!(rows.len(), 3);
    assert_eq!(rows[0]["verdict"], "peak", "{}", rows[0]);
    assert!(rows[0]["counts_gross"].as_f64().unwrap() > 10.0);
    assert!(rows[0]["sn_floor"].as_f64().unwrap() >= 3.0);
    assert!(rows[0]["rule"].as_str().unwrap().ends_with("-> peak"));
    assert_eq!(rows[2]["verdict"], "nothing", "{}", rows[2]);
    assert!(rows[2]["ul_counts"].as_f64().unwrap() > 0.0);
    assert!(rows[2]["ul_area"].as_f64().is_some());
    assert!(v["files"][0]["samples"][0]["ion_quantum"]["cps"]
        .as_f64()
        .is_some());
    // every automatic choice is logged and the parameters used are in the document
    let choices: Vec<&str> = v["provenance"]["choices"]
        .as_array()
        .unwrap()
        .iter()
        .map(|c| c["what"].as_str().unwrap())
        .collect();
    assert!(
        choices.iter().any(|c| c.starts_with("ion_quantum")) && choices.contains(&"background"),
        "{choices:?}"
    );
    assert_eq!(v["provenance"]["params"]["set"]["alpha"], 0.05);
    // a stricter threshold from --set is used and logged
    let o = cli(
        &[
            "raw",
            "xic",
            "W.mzML",
            "--mz",
            "200",
            "--rt",
            "3,7",
            "--set",
            "min_sn=1000",
        ],
        &d,
    );
    let v = doc(&o);
    assert_ne!(v["rows"][0]["verdict"], "peak");
    assert_eq!(v["provenance"]["params"]["set"]["min_sn"], 1000.0);
    let _ = std::fs::remove_dir_all(&d);
}

#[test]
fn the_exits_are_pinned_and_every_error_is_an_error_document() {
    let d = dir("exits");
    let ok = ["raw", "xic", "S_01.mzML", "--mz", "200", "--rt", "4,6"];
    assert_eq!(code(&cli(&ok, &d)), 0);
    // 1: a file that cannot be read keeps the document on stdout
    std::fs::write(d.join("bad.raw"), b"\x00\x01garbage").unwrap();
    let o = cli(
        &[
            "raw",
            "xic",
            "S_01.mzML",
            "bad.raw",
            "--mz",
            "200",
            "--rt",
            "4,6",
        ],
        &d,
    );
    assert_eq!(code(&o), 1);
    let v = doc(&o);
    assert!(v["files"][1]["error"]["error_key"].is_string());
    assert_eq!(v["rows"].as_array().unwrap().len(), 1);
    // 2: usage (a missing flag, a window the wrong way, a value out of range, an unknown --set name)
    for bad in [
        vec!["--mz", "200"],
        vec!["--mz", "200", "--rt", "6,4"],
        vec!["--mz", "200", "--rt", "4,6", "--da", "9"],
        vec!["--mz", "200", "--rt", "4,6", "--set", "nope=1"],
    ] {
        let mut a = vec!["raw", "xic", "S_01.mzML"];
        a.extend(bad.iter().copied());
        let o = cli(&a, &d);
        assert_eq!(code(&o), 2, "{bad:?}");
        assert!(!o.stderr.is_empty(), "{bad:?}");
        // the same failure as one error/1 document on stdout
        assert_eq!(doc(&o)["schema"], "error/1");
        assert_eq!(doc(&o)["code"], 2);
    }
    // 4: a missing file; 6: --out not writable
    assert_eq!(
        code(&cli(
            &["raw", "xic", "absent.mzML", "--mz", "200", "--rt", "4,6"],
            &d
        )),
        4
    );
    let mut w = ok.to_vec();
    w.extend(["--out", "/nonexistent/dir/x.json"]);
    assert_eq!(code(&cli(&w, &d)), 6);
    // --sample on a single-sample mzML is a usage error (exit 5, an unknown sample of a multi-sample file, is the shared `sample_select` path of `raw info`)
    let mut s = ok.to_vec();
    s.extend(["--sample", "999"]);
    assert_eq!(code(&cli(&s, &d)), 2);
    let _ = std::fs::remove_dir_all(&d);
}

#[test]
fn the_dry_run_reads_nothing_and_the_json_flag_changes_no_byte() {
    let d = dir("plan");
    let ok = ["raw", "xic", "S_01.mzML", "--mz", "200,220", "--rt", "4,6"];
    let mut p = ok.to_vec();
    p.push("--dry-run");
    let o = cli(&p, &d);
    assert_eq!(code(&o), 0);
    let v = doc(&o);
    assert_eq!(v["schema"], "raw-plan/1");
    assert_eq!(v["command"], "raw xic");
    assert_eq!(v["files"][0]["cache"], "none");
    assert_eq!(v["query"]["mz"].as_array().unwrap().len(), 2);
    let plain = cli(&ok, &d);
    let mut j = ok.to_vec();
    j.push("--json");
    assert_eq!(cli(&j, &d).stdout, plain.stdout);
    // the same question twice gives the same bytes
    assert_eq!(cli(&ok, &d).stdout, plain.stdout);
    let _ = std::fs::remove_dir_all(&d);
}

/// Exit 5: a `.wiff` of two samples (`tools/wiff_riferimento.py`) has no sample 9; the answer is the `error/1` of `err.sample.unknown` with the ids that exist,
/// and `--sample 1` and `--sample 2` each read their own sample. Skipped without python3 (never in CI).
#[test]
fn an_unknown_sample_of_a_multi_sample_file_is_exit_5() {
    let d = std::env::temp_dir().join(format!("mzlab_raw_xic_wiff_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    let tools = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tools");
    let made = Command::new("python3")
        .arg(tools.join("wiff_riferimento.py"))
        .arg(&d)
        .output();
    if !made.is_ok_and(|o| o.status.success()) {
        assert!(
            std::env::var("MZLAB_GOLDEN_STRICT").is_err()
                && std::env::var("GITHUB_ACTIONS").is_err(),
            "python3 not available"
        );
        eprintln!("skipped: python3 not available");
        return;
    }
    let ask = |sample: &str| {
        cli(
            &[
                "raw", "xic", "two.wiff", "--mz", "200", "--rt", "1,2", "--sample", sample,
            ],
            &d,
        )
    };
    let bad = ask("9");
    assert_eq!(code(&bad), 5, "{}", String::from_utf8_lossy(&bad.stderr));
    let v = doc(&bad);
    assert_eq!(v["schema"], "error/1");
    assert_eq!(v["error_key"], "err.sample.unknown");
    assert_eq!(v["params"]["ids"], "1, 2");
    for id in ["1", "2"] {
        let o = ask(id);
        assert!(
            code(&o) <= 1,
            "{id}: {}",
            String::from_utf8_lossy(&o.stderr)
        );
        assert_eq!(doc(&o)["query"]["sample"], id);
    }
    let _ = std::fs::remove_dir_all(&d);
}
