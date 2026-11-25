//! `mzlab-cli quant targets --rt-ref` through the real binary on two synthetic batches whose retention times differ by 0.3 min: the column and
//! the log carry the shift and its evidence, stderr points at `--rt-ref auto` and `--offset auto` only when the data ask for them, and a bad
//! value is exit 2 with its key.

use std::path::{Path, PathBuf};
use std::process::{Command, Output};

#[path = "../../mzlab-feat/tests/common_quant/mod.rs"]
#[allow(dead_code)]
mod common_quant;
use common_quant::{mzml, sample, Ion};

/// Two batches of one sample, the second 0.3 min later, and a narrow window list whose `std` row is the internal reference.
fn drift_dir(tag: &str) -> PathBuf {
    let d = std::env::temp_dir().join(format!("mzlab_quant_rt_{tag}_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(d.join("runs")).unwrap();
    let later: Vec<Ion> = sample()
        .iter()
        .map(|o| Ion(o.0, o.1 + 0.3, o.2, o.3))
        .collect();
    std::fs::write(d.join("runs/S_01.mzML"), mzml(&sample(), 1)).unwrap();
    std::fs::write(d.join("runs/S_02.mzML"), mzml(&later, 3)).unwrap();
    std::fs::write(
        d.join("narrow.csv"),
        "id,mz,rt_min,rt_max,polarity,role\nclean,200.0,4.8,5.2,+,\nstd,400.0,1.5,2.5,+,standard\n",
    )
    .unwrap();
    d
}

fn run(d: &Path, extra: &[&str]) -> Output {
    let mut a = vec![
        "quant",
        "targets",
        "--targets",
        "narrow.csv",
        "runs/S_01.mzML",
        "runs/S_02.mzML",
    ];
    a.extend_from_slice(extra);
    Command::new(env!("CARGO_BIN_EXE_mzlab-cli"))
        .args(a)
        .current_dir(d)
        .output()
        .unwrap()
}

fn text(b: &[u8]) -> String {
    String::from_utf8_lossy(b).into_owned()
}

#[test]
fn rt_ref_moves_the_windows_and_writes_the_shift_and_its_evidence() {
    let d = drift_dir("on");
    // off: the old header and a missed peak in the later batch
    let off = run(&d, &[]);
    assert_eq!(off.status.code(), Some(0), "{}", text(&off.stderr));
    let off = text(&off.stdout);
    assert!(off.starts_with(&format!("{}\n", mzlab_feat::quant_targets::HEADER)));
    assert!(
        !off.lines()
            .any(|l| l.starts_with("S_02.mzML,clean,") && l.contains(",ok,")),
        "{off}"
    );
    // on: the column, the moved window, the evidence in the log
    let on = run(&d, &["--rt-ref", "auto", "--log", "log.json"]);
    assert_eq!(on.status.code(), Some(0), "{}", text(&on.stderr));
    let csv = text(&on.stdout);
    let header = csv.lines().next().unwrap();
    assert!(header.ends_with(",mz_offset_da,rt_shift_min"), "{csv}");
    let row = csv
        .lines()
        .find(|l| l.starts_with("S_02.mzML,clean,"))
        .unwrap();
    assert!(row.contains(",detected,"), "{row}");
    let shift: f64 = row.rsplit(',').next().unwrap().parse().unwrap();
    assert!((shift - 0.3).abs() < 0.03, "{row}");
    let log: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(d.join("log.json")).unwrap()).unwrap();
    let ev = &log["files"][1]["rt_ref"];
    assert_eq!(
        (ev["reference"].as_str(), ev["status"].as_str()),
        (Some("std"), Some("ok")),
        "{log}"
    );
    let first = log["files"][0]["rt_ref"]["shift_min"].as_f64().unwrap();
    assert!(first.abs() < 0.03, "{log}");
    assert_eq!(log["params"]["rt_ref"], "auto");
    // --json carries the same column, the dry run the parameter and no data
    let j = run(&d, &["--rt-ref", "std@2.0", "--json"]);
    let json: serde_json::Value = serde_json::from_str(&text(&j.stdout)).unwrap();
    assert!(json["rows"][0].get("rt_shift_min").is_some(), "{json}");
    let plan = text(&run(&d, &["--rt-ref", "auto", "--dry-run"]).stdout);
    assert!(plan.contains("\"rt_ref\":\"auto\""), "{plan}");
    let _ = std::fs::remove_dir_all(&d);
}

#[test]
fn a_bad_rt_ref_is_usage_and_the_hints_name_the_switches_only_when_the_data_ask() {
    let d = drift_dir("hint");
    // an unknown id or a bad value is usage (exit 2) with the key on stderr
    for bad in ["nope", "auto@x", "IS@b"] {
        let e = run(&d, &["--rt-ref", bad]);
        assert_eq!(e.status.code(), Some(2), "{bad}: {}", text(&e.stderr));
        assert!(text(&e.stderr).contains("err.quant.targets.param"));
    }
    // the reference peaks 0.3 min apart between the batches: stderr names the switch, and stops once it is on
    let hint = text(&run(&d, &[]).stderr);
    assert!(
        hint.contains("hint: ")
            && hint.contains("--rt-ref auto")
            && hint.contains("note.quant.targets.rtRef")
            && hint.contains("0.3 min apart"),
        "{hint}"
    );
    assert!(!text(&run(&d, &["--rt-ref", "auto"]).stderr).contains("note.quant.targets.rtRef"));
    // a reference missed at offset 0 names --offset auto, which then measures it
    let missed = text(&run(&d, &["--da", "0.2"]).stderr);
    assert!(
        missed.contains("--offset auto") && missed.contains("note.quant.targets.offsetAuto"),
        "{missed}"
    );
    // ... as one possible cause: the drift and the window are named too
    assert!(
        missed.contains("One possible cause")
            && missed.contains("--rt-ref auto")
            && missed.contains("rt_min"),
        "{missed}"
    );
    let measured = text(&run(&d, &["--da", "0.2", "--offset", "auto"]).stderr);
    assert!(
        !measured.contains("note.quant.targets.offsetAuto"),
        "{measured}"
    );
    let _ = std::fs::remove_dir_all(&d);
}

#[test]
fn the_hints_are_in_the_json_answer_and_never_in_the_log_file() {
    let d = drift_dir("hintjson");
    let on = run(&d, &["--json", "--log", "log.json"]);
    assert_eq!(on.status.code(), Some(0), "{}", text(&on.stderr));
    let json: serde_json::Value = serde_json::from_str(&text(&on.stdout)).unwrap();
    assert_eq!(
        json["hints"][0]["key"], "note.quant.targets.rtRef",
        "{json}"
    );
    assert_eq!(json["hints"][0]["params"]["id"], "std");
    assert!(json["hints"][0]["text"]
        .as_str()
        .unwrap()
        .contains("--rt-ref auto"));
    // the log file is the log of the site, byte for byte: it carries no hint, and the answer's own `log` member is the same
    let log = std::fs::read_to_string(d.join("log.json")).unwrap();
    assert!(!log.contains("hints"), "{log}");
    assert_eq!(
        json["log"],
        serde_json::from_str::<serde_json::Value>(&log).unwrap()
    );
    // with the switch on, or nothing to point at, the key is absent
    let off = run(&d, &["--rt-ref", "auto", "--json"]);
    assert!(
        !text(&off.stdout).contains("\"hints\""),
        "{}",
        text(&off.stdout)
    );
    let _ = std::fs::remove_dir_all(&d);
}
