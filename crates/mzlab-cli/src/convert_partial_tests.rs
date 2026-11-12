//! `mzlab-cli convert` on a cut file: it is written with the scans it holds, with exit code 3, a warning line naming the counts,
//! and `conv/1` of the report saying them; the whole file is not partial.
use mzlab_core::json::parse;

use super::convert;
use crate::convert_fixtures::folder;
use crate::flow_run_tests::p;

/// An mzXML cut in half is written with the scans it holds: code 3, a warning line with the counts, and `conv/1` of the report says them.
#[test]
fn a_cut_file_is_written_with_code_3_and_its_counts() {
    use crate::convert_fixtures::mzxml;
    let dir = folder("cut");
    let src = dir.join("in");
    std::fs::create_dir_all(&src).unwrap();
    let o = mzxml::Opz {
        zlib: false,
        ..mzxml::FLAT
    };
    let whole = mzxml::mzxml(&mzxml::run(), o);
    std::fs::write(src.join("t.mzXML"), &whole[..whole.len() / 2]).unwrap();
    let (out, report) = (dir.join("out"), dir.join("r.json"));
    let o = convert(&[p(&src), "--out", p(&out), "--report", p(&report)]);
    assert_eq!(o.code, 3, "{}", o.stderr);
    assert!(o.stderr.contains("warning: ") && o.stderr.contains("partial read, "));
    let r = parse(&std::fs::read(&report).unwrap()).unwrap();
    let conv = r.arr()[0].get("conv/1").expect("conv/1");
    let n = |k: &str| conv.get(k).and_then(|v| v.f64()).unwrap();
    assert_eq!(conv.get("parziale").and_then(|v| v.bool()), Some(true));
    assert!(n("lette") < n("dichiarate"), "{}", o.stderr);
    assert!(o.stderr.contains(&format!(
        "partial read, {} of {} scans (warn.file.truncated)",
        n("lette"),
        n("dichiarate")
    )));
    // the whole file is not partial
    std::fs::write(src.join("t.mzXML"), &whole).unwrap();
    let o = convert(&[p(&src), "--out", p(&out), "--fresh"]);
    assert_eq!(o.code, 0, "{}", o.stderr);
    let _ = std::fs::remove_dir_all(&dir);
}

/// A `.wiff.scan` with the marker of one scan block damaged (the synthetic `one` of `tools/wiff_riferimento.py`): the pair is written with the
/// other scans, code 3, and `conv/1` names the cause `warn.wiff.damaged`. Skipped without python3 (`MZLAB_GOLDEN_STRICT=1` fails instead).
#[test]
fn a_damaged_scan_block_of_a_wiff_is_written_with_code_3_and_its_cause() {
    use std::path::Path;
    use std::process::Command;
    let dir = folder("danno");
    let src = dir.join("in");
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let made = Command::new("python3")
        .arg(root.join("tools/wiff_riferimento.py"))
        .arg(&src)
        .output();
    if !made.is_ok_and(|o| o.status.success()) {
        assert!(
            std::env::var("MZLAB_GOLDEN_STRICT").is_err(),
            "python3 not available"
        );
        return eprintln!("SKIP a_damaged_scan_block: python3 not available");
    }
    let scan = src.join("one.wiff.scan");
    let mut b = std::fs::read(&scan).unwrap();
    // the first block starts 44 bytes after its index offset (0): its marker `fe ff ff ff`
    assert_eq!(b[44..48], [0xfe, 0xff, 0xff, 0xff]);
    b[44] = 0;
    std::fs::write(&scan, b).unwrap();
    let (out, report) = (dir.join("out"), dir.join("r.json"));
    let o = convert(&[
        p(&src.join("one.wiff")),
        "--out",
        p(&out),
        "--report",
        p(&report),
    ]);
    assert_eq!(o.code, 3, "{}", o.stderr);
    let r = parse(&std::fs::read(&report).unwrap()).unwrap();
    let conv = r.arr()[0].get("conv/1").expect("conv/1");
    let n = |k: &str| conv.get(k).and_then(|v| v.f64()).unwrap();
    assert_eq!(
        (
            conv.get("parziale").and_then(|v| v.bool()),
            conv.get("perdita").and_then(|v| v.str())
        ),
        (Some(true), Some("warn.wiff.damaged"))
    );
    assert_eq!(n("dichiarate") - n("lette"), 1.0, "{}", o.stderr);
    assert!(o.stderr.contains("(warn.wiff.damaged)"), "{}", o.stderr);
    let _ = std::fs::remove_dir_all(&dir);
}

/// The IDA pair `ida` of `tools/wiff_riferimento.py` (its index rows hold no point count): both samples are written whole, code 0, nothing partial.
/// Skipped without python3 (`MZLAB_GOLDEN_STRICT=1` fails instead).
#[test]
fn an_ida_wiff_is_written_whole_with_code_0() {
    let dir = folder("ida");
    let src = dir.join("in");
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let made = std::process::Command::new("python3")
        .arg(root.join("tools/wiff_riferimento.py"))
        .arg(&src)
        .output();
    if !made.is_ok_and(|o| o.status.success()) {
        assert!(
            std::env::var("MZLAB_GOLDEN_STRICT").is_err(),
            "python3 not available"
        );
        return eprintln!("SKIP an_ida_wiff: python3 not available");
    }
    let (out, report) = (dir.join("out"), dir.join("r.json"));
    let o = convert(&[
        p(&src.join("ida.wiff")),
        "--out",
        p(&out),
        "--report",
        p(&report),
    ]);
    assert_eq!(o.code, 0, "{}", o.stderr);
    let r = parse(&std::fs::read(&report).unwrap()).unwrap();
    for row in r.arr() {
        let conv = row.get("conv/1").expect("conv/1");
        let n = |k: &str| conv.get(k).and_then(|v| v.f64()).unwrap();
        assert_eq!(
            (conv.get("parziale").and_then(|v| v.bool()), n("lette")),
            (Some(false), 9.0),
            "{}",
            o.stderr
        );
    }
    let _ = std::fs::remove_dir_all(&dir);
}

#[path = "convert_tests_centroids.rs"]
mod centroid_counts;
