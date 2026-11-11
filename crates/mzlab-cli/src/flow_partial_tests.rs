//! `convert.mzml` on a cut file (H6-8): the run record says what the read lost. `prov.json`, `corsa.json` and `flowrun/1` carry the cause among the
//! warnings and the counts as the note of the file, the same counts `mzlab-cli convert` writes in `conv/1`, also when the step comes from the cache;
//! the whole file says nothing.
use mzlab_core::json::{parse, Value};

use crate::convert_fixtures::{folder, mzxml};
use crate::flow_run_tests::{flow_run, p};

const PLAN: &str = r#"{"format":"mzflow/2","inputs":[{"name":"t.mzXML","type":"raw"}],"steps":[
{"id":"c","step":"convert.mzml@1","in":{"raw":"file:t.mzXML"}}]}"#;

/// The synthetic mzXML of the convert tests, whole or cut in half.
fn mzxml_file(cut: bool) -> Vec<u8> {
    let o = mzxml::Opz {
        zlib: false,
        ..mzxml::FLAT
    };
    let whole = mzxml::mzxml(&mzxml::run(), o);
    if cut {
        whole[..whole.len() / 2].to_vec()
    } else {
        whole
    }
}

/// `flow run --json` of the plan on `t.mzXML` into `out`: the `flowrun/1` document.
fn run(dir: &std::path::Path, out: &str) -> Value {
    let (plan, data, out) = (dir.join("plan.json"), dir.join("data"), dir.join(out));
    let o = flow_run(&[p(&plan), "--in", p(&data), "--out", p(&out), "--json"]);
    assert_eq!(o.code, 0, "{}", o.stderr);
    parse(&o.stdout).unwrap()
}

fn step_c(doc: &Value) -> &Value {
    doc.get("steps")
        .unwrap()
        .arr()
        .iter()
        .find(|s| s.get("instance").and_then(Value::str) == Some("c"))
        .unwrap()
}

/// (warnings, declared, read, cause) of the instance `c` in a record (`prov.json`, `corsa.json` activity or `flowrun/1` step).
fn said(c: &Value) -> (Vec<String>, f64, f64, String) {
    let w = c.get("warnings").map_or(vec![], |w| {
        w.arr()
            .iter()
            .filter_map(Value::str)
            .map(str::to_string)
            .collect()
    });
    let f = c.get("files").and_then(|f| f.get("t.mzXML"));
    let n = |k: &str| {
        f.and_then(|f| f.get(k))
            .and_then(Value::f64)
            .unwrap_or(-1.0)
    };
    let cause = f
        .and_then(|f| f.get("cause"))
        .and_then(Value::str)
        .unwrap_or_default();
    (w, n("declared"), n("read"), cause.to_string())
}

/// The activity of `c` in the `prov.json` (keyed by instance) or the step of `c` in the `corsa.json` (a list) of `out`.
fn activity(out: &std::path::Path, file: &str) -> Value {
    let t = std::fs::read_to_string(out.join(file)).unwrap();
    let v = parse(t.as_bytes()).unwrap();
    let c = match v.get("activity") {
        Some(a) => a.get("c").cloned(),
        None => Some(step_c(&v).clone()),
    };
    c.unwrap_or_else(|| panic!("{file}: {t}"))
}

#[test]
fn a_cut_file_is_in_the_record_of_the_run_with_its_counts() {
    let dir = folder("flow_cut");
    std::fs::create_dir_all(dir.join("data")).unwrap();
    std::fs::write(dir.join("data/t.mzXML"), mzxml_file(true)).unwrap();
    std::fs::write(dir.join("plan.json"), PLAN).unwrap();
    // the counts of `mzlab-cli convert` on the same file
    let report = dir.join("r.json");
    let (data, conv) = (dir.join("data"), dir.join("conv"));
    let args = [p(&data), "--out", p(&conv), "--report", p(&report)];
    let mut a = vec!["convert".to_string()];
    a.extend(args.iter().map(|x| x.to_string()));
    let conv = crate::execute(&a);
    assert_eq!(conv.code, 3, "{}", conv.stderr);
    let r = parse(&std::fs::read(&report).unwrap()).unwrap();
    let c1 = r.arr()[0].get("conv/1").unwrap();
    let n = |k: &str| c1.get(k).and_then(Value::f64).unwrap();
    let want = (
        vec!["warn.file.truncated".to_string()],
        n("dichiarate"),
        n("lette"),
        "warn.file.truncated".to_string(),
    );
    assert!(want.2 < want.1, "{want:?}");

    let doc = run(&dir, "out");
    assert_eq!(said(step_c(&doc)), want, "flowrun/1");
    for f in ["prov.json", "corsa.json"] {
        assert_eq!(said(&activity(&dir.join("out"), f)), want, "{f}");
    }
    // the same folder again: the step comes from the cache and says the same
    let again = run(&dir, "out");
    assert_eq!(
        step_c(&again).get("status").and_then(Value::str),
        Some("cached")
    );
    assert_eq!(said(step_c(&again)), want, "flowrun/1 from the cache");
    assert_eq!(
        said(&activity(&dir.join("out"), "prov.json")),
        want,
        "prov.json from the cache"
    );

    // the whole file: no warning, no note
    std::fs::write(dir.join("data/t.mzXML"), mzxml_file(false)).unwrap();
    let whole = run(&dir, "whole");
    assert_eq!(said(step_c(&whole)), (vec![], -1.0, -1.0, String::new()));
    let prov = std::fs::read_to_string(dir.join("whole/prov.json")).unwrap();
    assert!(!prov.contains("warn.file.truncated"), "{prov}");
    let _ = std::fs::remove_dir_all(&dir);
}
