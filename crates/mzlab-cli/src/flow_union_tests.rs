//! `features.union@1` in a plan: two synthetic tables (invented masses, one series shifted in RT) give the target list and the `union/1` of the engine
//! called with the parameters as the page sends them (descriptor defaults as JSON, the `flowtab` path), so `flow run` and the browser write the same
//! bytes; the list is what `mzlab-cli quant union` writes for the same tables as CSV; `--set` reaches the engine and a bad value is refused.

use std::path::{Path, PathBuf};

use mzlab_core::json::{parse, Out, Value};
use mzlab_feat::recipe::Recipe;
use mzlab_feat::store::{write_table, Origin};
use mzlab_feat::table::{Aligned, CELL_DETECTED};

/// One series: (m/z, RT) rows over two files, the RT moved by `shift` min.
fn table(rows: &[(f64, f64)], shift: f64) -> Vec<u8> {
    let mut t = Aligned::with_capacity(2, rows.len());
    for (mz, rt) in rows {
        t.mz.push(*mz);
        t.rt.push(rt + shift);
        for h in [1e5, 2e5] {
            t.area.push(h * 6.0);
            t.height.push(h as f32);
            t.apex_rt.push((rt + shift) as f32);
            t.width.push(0.08);
            t.state.push(CELL_DETECTED);
        }
    }
    let origin = Origin {
        inputs: (0..2)
            .map(|k| (format!("c{k}.mzML"), format!("{k:02x}").repeat(32)))
            .collect(),
        ..Origin::default()
    };
    write_table(&t, &Recipe::default(), &origin)
}

fn tables() -> [(&'static str, Vec<u8>); 2] {
    [
        (
            "a.mzfeat",
            table(&[(301.0, 3.0), (402.2, 4.0), (512.1, 6.0)], 0.0),
        ),
        (
            "b.mzfeat",
            table(&[(301.1, 3.0), (402.2, 4.0), (230.0, 2.0)], 0.04),
        ),
    ]
}

fn dir(tag: &str) -> PathBuf {
    let d = std::env::temp_dir().join(format!("mzlab_cli_union_{tag}_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(d.join("work")).unwrap();
    for (n, b) in tables() {
        std::fs::write(d.join(n), b).unwrap();
    }
    d
}

fn plan(params: &str) -> String {
    format!(
        r#"{{"format":"mzflow/2","inputs":[{{"name":"a.mzfeat","type":"table"}},{{"name":"b.mzfeat","type":"table"}}],"steps":[
        {{"id":"uni","step":"features.union@1","in":{{"table":["file:a.mzfeat","file:b.mzfeat"]}},"params":{params}}}]}}"#
    )
}

fn run(d: &Path, params: &str) -> (String, String, String) {
    let (run, prov) =
        crate::flow::run_plan(plan(params).as_bytes(), d, &d.join("work"), 1, None).unwrap();
    assert!(run.skipped.is_empty(), "{prov}");
    let read = |n: &str| std::fs::read_to_string(d.join("work").join(n)).unwrap_or_default();
    (read("uni.csv"), read("uni.union.json"), prov)
}

/// What the browser sends: the defaults of the descriptor merged with the parameters of the instance, as JSON.
fn page_params(over: &[(&str, Value)]) -> String {
    let reg = crate::flow::steps();
    let step = mzlab_core::step::find(&reg, "features.union").unwrap();
    let mut o = Out::default();
    o.raw("{");
    for (k, p) in step.params.iter().enumerate() {
        let v = over
            .iter()
            .find(|(n, _)| *n == p.name)
            .map(|(_, v)| v.clone())
            .or(p.default.clone())
            .unwrap();
        o.raw(if k > 0 { "," } else { "" }).str(p.name).raw(":");
        mzlab_core::json::write_value(&mut o, &v);
    }
    o.raw("}");
    o.0
}

fn engine(over: &[(&str, Value)]) -> (String, String) {
    let params = mzlab_feat::flow_ms2::json_params(&page_params(over)).unwrap();
    let t = tables().map(|(n, b)| (n.to_string(), b));
    let m = mzlab_feat::union::union_text(&params, &t).unwrap();
    (m.csv, m.doc)
}

#[test]
fn the_plan_writes_the_bytes_of_the_browser_engine() {
    let d = dir("engine");
    let (csv, doc, prov) = run(&d, "{}");
    assert!(!prov.contains("\"error\""), "{prov}");
    assert_eq!((csv.clone(), doc.clone()), engine(&[]));
    // 301 and 301.1 (RT within 0.1 after no map: 0.04) are one target, 402.2 too: 4 targets, 2 of them merged
    let v = parse(doc.as_bytes()).unwrap();
    assert_eq!(v.get("targets").unwrap().arr().len(), 4, "{doc}");
    assert_eq!(csv.lines().count(), 5, "{csv}");
    assert!(
        csv.starts_with("id,mz,rt_min,rt_max,polarity,role\nU0001,"),
        "{csv}"
    );
    assert_eq!(run(&d, "{}").0, csv);
    let _ = std::fs::remove_dir_all(&d);
}

#[test]
fn the_list_is_the_one_of_quant_union_on_the_csv_of_the_tables() {
    let d = dir("cli");
    let (csv, doc, _) = run(&d, "{}");
    let req = |n: &str, b: &[u8]| {
        let t = mzlab_feat::union::series_csv(b).unwrap();
        format!("{{\"name\":{:?},\"text\":{:?}}}", n, t)
    };
    let t = tables();
    let body = format!(
        "{{\"tables\":[{},{}],\"params\":{{}}}}",
        req(t[0].0, &t[0].1),
        req(t[1].0, &t[1].1)
    );
    assert_eq!(mzlab_core::series_union::route(&body).unwrap(), doc);
    assert_eq!(
        parse(doc.as_bytes())
            .unwrap()
            .get("csv")
            .and_then(Value::str),
        Some(csv.as_str())
    );
    let _ = std::fs::remove_dir_all(&d);
}

#[test]
fn set_parameters_reach_the_engine() {
    let d = dir("set");
    let (csv, doc, _) = run(&d, r#"{"min_series":2,"polarity":"+"}"#);
    assert_eq!(
        (csv.clone(), doc),
        engine(&[
            ("min_series", Value::Num(2.0)),
            ("polarity", Value::Str("+".into()))
        ])
    );
    // only the two merged targets are seen in both series; each is written once, with the polarity
    assert_eq!(csv.lines().count(), 3, "{csv}");
    assert!(csv.lines().nth(1).unwrap().contains(",+,"), "{csv}");
    let _ = std::fs::remove_dir_all(&d);
}

#[test]
fn a_value_out_of_range_is_refused_by_the_check_and_by_the_engine() {
    let d = dir("bad");
    let e = crate::flow::run_plan(plan(r#"{"da":9}"#).as_bytes(), &d, &d.join("work"), 1, None)
        .unwrap_err();
    assert!(
        e.params
            .iter()
            .any(|(_, v)| v.contains("err.flow.param.range")),
        "{e:?}"
    );
    // a page that skipped the check still gets the engine's own refusal
    let bad = mzlab_feat::flow_ms2::json_params(r#"{"da":9}"#).unwrap();
    let t = tables().map(|(n, b)| (n.to_string(), b));
    assert_eq!(
        mzlab_feat::union::union_text(&bad, &t).unwrap_err().key,
        "err.union.param"
    );
    let _ = std::fs::remove_dir_all(&d);
}
