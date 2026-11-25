//! `mzlab-cli file method` on the synthetic `.wiff` of `tools/wiff_riferimento.py` (`dispersione`: precursor 253.1, CE 35, CES 15, DP 60, a sample
//! named for another mass): the parameters of the segments, null with a reason where the format has none, the name warning, the pinned exit codes
//! and the hints that make the command findable. Without python3 the tests skip, `MZLAB_GOLDEN_STRICT=1` fails instead.

use std::path::{Path, PathBuf};
use std::process::Command;

use mzlab_core::json::{parse, Value};

use crate::execute;

fn fixture() -> Option<PathBuf> {
    let dir = std::env::temp_dir().join(format!("mzlab_cli_method_{}", std::process::id()));
    let wiff = dir.join("dispersione.wiff");
    if wiff.exists() {
        return Some(wiff);
    }
    let tools = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tools");
    let made = Command::new("python3")
        .arg(tools.join("wiff_riferimento.py"))
        .arg(&dir)
        .output();
    if made.is_ok_and(|o| o.status.success()) && wiff.exists() {
        return Some(wiff);
    }
    assert!(
        std::env::var("MZLAB_GOLDEN_STRICT").is_err(),
        "python3 not available"
    );
    eprintln!("skipped: python3 not available");
    None
}

fn run(a: &[&str]) -> crate::Outcome {
    execute(&a.iter().map(|s| s.to_string()).collect::<Vec<_>>())
}

#[test]
fn a_product_ion_experiment_gives_its_parameters_and_the_missing_ones_say_why() {
    let Some(w) = fixture() else { return };
    let o = run(&["file", "method", &w.to_string_lossy()]);
    assert_eq!(o.code, 0, "{}", o.stderr);
    let v = parse(&o.stdout).unwrap();
    assert_eq!(v.get("schema").unwrap().str(), Some("file_method/1"));
    let seg = v
        .get("segments")
        .unwrap()
        .arr()
        .iter()
        .find(|s| s.get("ms_level").and_then(Value::f64) == Some(2.0))
        .unwrap();
    let f = |k: &str| seg.get(k).unwrap().f64();
    assert_eq!(
        (f("precursor_mz"), f("ce_eV"), f("ces_eV"), f("dp_V")),
        (Some(253.1), Some(35.0), Some(15.0), Some(60.0))
    );
    assert!(
        seg.get("fill_time_ms").unwrap().is_null() && seg.get("q1_resolution").unwrap().is_null()
    );
    let u = v.get("unavailable").unwrap();
    assert_eq!(u.get("fill_time_ms").unwrap().str(), Some("not_decoded"));
    assert_eq!(u.get("q1_resolution").unwrap().str(), Some("not_decoded"));
    assert!(u.get("ces_eV").is_none());
}

#[test]
fn a_name_mass_that_is_not_the_precursor_is_a_warning_with_the_numbers_and_nothing_is_blocked() {
    let Some(w) = fixture() else { return };
    let o = run(&["file", "method", &w.to_string_lossy(), "--json"]);
    assert_eq!(o.code, 0);
    let v = parse(&o.stdout).unwrap();
    let k = &v.get("warnings").unwrap().arr()[0];
    assert_eq!(k.get("key").unwrap().str(), Some("warn.file.nameMz"));
    assert_eq!(
        (
            k.get("nearest_name_mz").unwrap().f64(),
            k.get("nearest_precursor").unwrap().f64()
        ),
        (Some(237.1), Some(253.1))
    );
    assert!(
        o.stderr.contains("237.1")
            && o.stderr.contains("253.1")
            && o.stderr.contains("(warn.file.nameMz)"),
        "{}",
        o.stderr
    );
    // file items says it too, and points at file method
    let it = run(&["file", "items", &w.to_string_lossy(), "--no-points"]);
    assert_eq!(it.code, 0, "{}", it.stderr);
    assert!(it.stderr.contains("(warn.file.nameMz)"), "{}", it.stderr);
    assert!(
        it.stderr.contains("hint:") && it.stderr.contains("file method"),
        "{}",
        it.stderr
    );
    assert!(
        parse(&it.stdout).is_ok(),
        "stdout stays the items/1 document"
    );
}

#[test]
fn sample_list_and_file_items_point_at_file_method() {
    let Some(w) = fixture() else { return };
    let l = run(&["sample", "list", &w.to_string_lossy(), "--json"]);
    assert!(
        l.stderr.contains("hint:") && l.stderr.contains("file method"),
        "{}",
        l.stderr
    );
}

#[test]
fn exit_codes_are_pinned() {
    let Some(w) = fixture() else { return };
    let wf = w.to_string_lossy().into_owned();
    let cases: Vec<(u8, Vec<&str>)> = vec![
        (0, vec!["file", "method", &wf]),
        (2, vec!["file", "method"]),
        (2, vec!["file", "method", &wf, "--bogus"]),
        (4, vec!["file", "method", "/nonexistent/none.wiff"]),
        (5, vec!["file", "method", &wf, "--sample", "9"]),
    ];
    for (code, args) in cases {
        let o = run(&args);
        assert_eq!(o.code, code, "{args:?}: {}", o.stderr);
    }
}

#[test]
fn the_command_is_listed_with_its_schema_and_is_found_by_its_tasks() {
    let o = run(&[
        "commands",
        "--find",
        "collision",
        "energy",
        "spread",
        "--json",
    ]);
    assert_eq!(o.code, 0, "{}", o.stderr);
    let v: serde_json::Value = serde_json::from_slice(&o.stdout).unwrap();
    assert!(v.to_string().contains("file method"), "{v}");
    let s = run(&["schema", "file_method/1"]);
    assert_eq!(s.code, 0, "{}", s.stderr);
}
