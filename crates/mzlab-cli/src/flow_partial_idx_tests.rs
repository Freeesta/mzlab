//! `index.mzidx` on a cut vendor file: the index keeps what its read lost (`source.lost`), and the run record says it as `convert.mzml` does (the cause
//! among the warnings, the counts as the note of the file) in `flowrun/1` and `prov.json`; the `lost` the page reads of the same index and hands to
//! `record_prov` is that note. The whole file writes no `lost` and says nothing.
use mzlab_core::json::{parse, Value};

use crate::convert_fixtures::{folder, wiff};
use crate::flow_run_tests::{flow_run, p};

const PLAN: &str = r#"{"format":"mzflow/2","inputs":[{"name":"c.wiff@1","type":"raw"}],"steps":[
{"id":"imp","step":"import.raw@1","in":{"files":["file:c.wiff@1"]}},
{"id":"idx","step":"index.mzidx@1","in":{"source":["imp.raw"]}},
{"id":"cv","step":"convert.mzml@1","in":{"raw":"file:c.wiff@1"}}]}"#;

/// (warnings without `warn.flow.profile`, note of `c.wiff@1`) of the step or activity `c`.
fn said(c: &Value) -> (Vec<String>, Option<Value>) {
    let w = c.get("warnings").map_or(vec![], |w| {
        w.arr()
            .iter()
            .filter_map(Value::str)
            .filter(|w| *w != "warn.flow.profile")
            .map(str::to_string)
            .collect()
    });
    (w, c.get("files").and_then(|f| f.get("c.wiff@1")).cloned())
}

fn step<'a>(doc: &'a Value, id: &str) -> &'a Value {
    let s = doc.get("steps").unwrap().arr().iter();
    s.into_iter()
        .find(|s| s.get("instance").and_then(Value::str) == Some(id))
        .unwrap()
}

/// `flow run --json` of the plan on `data` into `out`.
fn run(dir: &std::path::Path, out: &str) -> Value {
    let (plan, data, out) = (dir.join("plan.json"), dir.join("in"), dir.join(out));
    let o = flow_run(&[p(&plan), "--in", p(&data), "--out", p(&out), "--json"]);
    assert_eq!(o.code, 0, "{}", o.stderr);
    parse(&o.stdout).unwrap()
}

#[test]
fn a_cut_vendor_file_is_in_the_record_of_its_index() {
    let dir = folder("flow_cut_idx");
    let src = dir.join("in");
    std::fs::create_dir_all(&src).unwrap();
    if !wiff(&src, "c", 1) {
        return;
    }
    std::fs::write(dir.join("plan.json"), PLAN).unwrap();
    let whole = run(&dir, "whole");
    assert_eq!(said(step(&whole, "idx")), (vec![], None));
    let idx_whole = std::fs::read_to_string(dir.join("whole/prov.json")).unwrap();
    assert!(!idx_whole.contains("warn.file.truncated"), "{idx_whole}");
    let ix = crate::flow_base::open(&dir.join("whole/idx-c-1.mzidx")).unwrap();
    assert!(
        ix.source.get("lost").is_none(),
        "a whole read writes no lost"
    );

    // the data file cut at two thirds: the index lists scans past its end
    let scan = src.join("c.wiff.scan");
    let b = std::fs::read(&scan).unwrap();
    std::fs::write(&scan, &b[..b.len() * 2 / 3]).unwrap();
    let doc = run(&dir, "out");
    let (w, note) = said(step(&doc, "idx"));
    assert_eq!(w, ["warn.file.truncated"]);
    let note = note.expect("the note of the file");
    let n = |k: &str| note.get(k).and_then(Value::f64).unwrap();
    assert!(n("read") < n("declared"), "{note:?}");
    // the same counts as the conversion of the same file
    assert_eq!(said(step(&doc, "cv")), (w.clone(), Some(note.clone())));
    let prov = parse(&std::fs::read(dir.join("out/prov.json")).unwrap()).unwrap();
    let act = prov.get("activity").and_then(|a| a.get("idx")).unwrap();
    assert_eq!(said(act), (w.clone(), Some(note.clone())), "prov.json");

    // the page hands the `lost` of the same index to `record_prov`, which reads it as these warnings and this note (`run_web_tests.rs`)
    let ix = crate::flow_base::open(&dir.join("out/idx-c-1.mzidx")).unwrap();
    let lost = ix.source.get("lost").expect("source.lost");
    // the same fields (the record writes its keys sorted)
    for k in ["declared", "read", "cause"] {
        assert_eq!(lost.get(k), note.get(k), "source.lost {k}");
    }
    assert_eq!(lost.get("cause").and_then(Value::str), Some(w[0].as_str()));
    let _ = std::fs::remove_dir_all(&dir);
}
