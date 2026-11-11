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
