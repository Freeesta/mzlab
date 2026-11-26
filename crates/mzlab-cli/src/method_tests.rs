//! `mzlab-cli file method` on the synthetic `.wiff` of `tools/wiff_riferimento.py` (`dispersione`: precursor 253.1, CE 35, CES 15, DP 60, a sample
//! named for another mass): the parameters of the segments, null with a reason where the format has none, the name warning, the pinned exit codes
//! and the hints that make the command findable. Without python3 the tests skip locally; `MZLAB_GOLDEN_STRICT=1`, `CI` or `GITHUB_ACTIONS` make that a failure.

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
    // CI must run these tests: a missing python3 there is a failure, not a skip
    assert!(
        ["MZLAB_GOLDEN_STRICT", "CI", "GITHUB_ACTIONS"]
            .iter()
            .all(|k| std::env::var(k).is_err()),
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
    assert_eq!(u.get("fill_time_ms").unwrap().str(), Some("unknown"));
    assert_eq!(u.get("q1_resolution").unwrap().str(), Some("unknown"));
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

/// A damaged method stream is refused with a stable error (never a panic, never a silent drop) and, where the method document is still built, it is
/// listed in `damaged` of the experiment, so the missing parameters are not read as "not in the method".
#[test]
fn damaged_method_streams_are_reported_and_never_panic() {
    let Some(w) = fixture() else { return };
    let dir = w.parent().unwrap();
    for (name, key) in [
        ("cut_header", "err.file.wiff"),
        ("cut_range", "err.file.wiffMassRange"),
        ("bad_tag", "err.file.wiffMassRange"),
    ] {
        let file = dir.join("damaged").join(format!("{name}.wiff"));
        let o = run(&["file", "method", &file.to_string_lossy()]);
        assert_eq!(o.code, 4, "{name}: {}", o.stderr);
        assert!(o.stderr.contains(key), "{name}: {}", o.stderr);
        let methods = mzlab_core::methodinfo::read_methods(std::fs::read(&file).unwrap(), name)
            .unwrap_or_default();
        let mut out = mzlab_core::json::Out::default();
        if let Some(v) = methods
            .first()
            .and_then(|m| mzlab_core::method::ms_section(m, None))
        {
            v.write(&mut out);
        }
        let doc = out.0;
        assert!(doc.contains("\"damaged\":[\""), "{name}: {doc}");
    }
}

/// The converter writes the method parameters the format holds, in every build: CE (`MS:1000045`) of the precursor, the declustering potential as the
/// PSI-MS term `MS:1000875` with its unit, and the collision energy spread as a userParam (PSI-MS has no term for it). This pins the bytes of the new params.
#[test]
fn the_converted_mzml_carries_ce_dp_and_the_spread() {
    let Some(w) = fixture() else { return };
    let out = w.parent().unwrap().join("converted");
    let o = run(&[
        "file",
        "convert",
        &w.to_string_lossy(),
        "--out",
        &out.to_string_lossy(),
    ]);
    assert_eq!(o.code, 0, "{}", o.stderr);
    let mzml = std::fs::read_dir(&out)
        .unwrap()
        .filter_map(Result::ok)
        .map(|e| e.path())
        .find(|p| p.extension().is_some_and(|x| x == "mzML"))
        .expect("an mzML");
    let text = std::fs::read_to_string(mzml).unwrap();
    assert!(
        text.contains(r#"accession="MS:1000875" name="declustering potential" value="60" unitCvRef="UO" unitAccession="UO:0000218" unitName="volt""#),
        "{}",
        &text[..text.len().min(3000)]
    );
    assert!(text.contains(r#"name="collision energy spread (eV)" value="15""#));
    assert!(text.contains(r#"accession="MS:1000045" name="collision energy" value="35""#));
}
