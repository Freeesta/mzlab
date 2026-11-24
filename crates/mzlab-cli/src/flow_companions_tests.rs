//! `features.companions@1` in a plan: a synthetic unit-mass table (invented masses: a compound with its M+1 at 11 %, an in-source loss of 18 and an
//! unrelated ion at +1) gives the `companions/1` of the engine called with the parameters as the page sends them (descriptor defaults as JSON, the
//! `flowtab` path), so `flow run` and the browser write the same bytes; `--set` reaches the engine, a bad list is refused, a second run is equal.

use std::path::{Path, PathBuf};

use mzlab_core::json::{parse, Out, Value};
use mzlab_feat::recipe::Recipe;
use mzlab_feat::store::{write_table, Origin};
use mzlab_feat::table::{Aligned, CELL_DETECTED};

const FILES: usize = 5;

fn table() -> Vec<u8> {
    let levels = [[1.0, 2.0, 3.0, 4.0, 5.0], [5.0, 1.0, 4.0, 2.0, 3.0]];
    // (m/z, RT, level set, share of its level)
    let rows = [
        (301.0, 3.00, 0, 1.0),
        (302.0, 3.00, 0, 0.11),
        (283.0, 3.01, 0, 0.30),
        (402.0, 3.00, 1, 0.11),
        (512.0, 6.00, 1, 1.0),
    ];
    let mut t = Aligned::with_capacity(FILES, rows.len());
    for (mz, rt, c, share) in rows {
        t.mz.push(mz);
        t.rt.push(rt);
        for level in levels[c] {
            let h = 1e5 * level * share;
            t.area.push(h * 6.0);
            t.height.push(h as f32);
            t.apex_rt.push(rt as f32);
            t.width.push(0.08);
            t.state.push(CELL_DETECTED);
        }
    }
    let origin = Origin {
        inputs: (0..FILES)
            .map(|k| (format!("c{k}.mzML"), format!("{k:02x}").repeat(32)))
            .collect(),
        ..Origin::default()
    };
    write_table(&t, &Recipe::default(), &origin)
}

fn dir(tag: &str) -> PathBuf {
    let d = std::env::temp_dir().join(format!("mzlab_cli_comp_{tag}_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(d.join("work")).unwrap();
    std::fs::write(d.join("table.mzfeat"), table()).unwrap();
    d
}

fn plan(params: &str) -> String {
    format!(
        r#"{{"format":"mzflow/2","inputs":[{{"name":"table.mzfeat","type":"table"}}],"steps":[
        {{"id":"cmp","step":"features.companions@1","in":{{"table":"file:table.mzfeat"}},"params":{params}}}]}}"#
    )
}

fn run(d: &Path, params: &str) -> (String, String) {
    let (run, prov) =
        crate::flow::run_plan(plan(params).as_bytes(), d, &d.join("work"), 1, None).unwrap();
    assert!(run.skipped.is_empty(), "{prov}");
    let text = std::fs::read_to_string(d.join("work/cmp.json")).unwrap_or_default();
    (text, prov)
}

/// What the browser sends: the defaults of the descriptor merged with the parameters of the instance, as JSON.
fn page_params(over: &[(&str, Value)]) -> String {
    let reg = crate::flow::steps();
    let step = mzlab_core::step::find(&reg, "features.companions").unwrap();
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

fn engine(over: &[(&str, Value)]) -> String {
    let params = mzlab_feat::flow_ms2::json_params(&page_params(over)).unwrap();
    mzlab_feat::companions::companions_text(&params, &table())
        .unwrap()
        .text
}

#[test]
fn the_plan_writes_the_text_of_the_browser_engine() {
    let d = dir("engine");
    let (got, prov) = run(&d, "{}");
    assert!(!prov.contains("\"error\""), "{prov}");
    assert_eq!(got, engine(&[]));
    let v = parse(got.as_bytes()).unwrap();
    assert_eq!(v.get("format").and_then(Value::str), Some("companions/1"));
    let kinds: Vec<&str> = v
        .get("edges")
        .unwrap()
        .arr()
        .iter()
        .filter_map(|e| e.get("kind").and_then(Value::str))
        .collect();
    // the M+1 and the loss of 18; the unrelated ion at 402 (another profile) and the late row give none
    assert_eq!(kinds, vec!["isotope", "relation"], "{got}");
    assert_eq!(v.get("rows").unwrap().f64(), Some(5.0));
    assert_eq!(got, run(&d, "{}").0);
    let _ = std::fs::remove_dir_all(&d);
}

#[test]
fn set_parameters_reach_the_engine() {
    let d = dir("set");
    let (got, _) = run(&d, r#"{"losses":"","min_corr":0.95}"#);
    assert_eq!(
        got,
        engine(&[
            ("losses", Value::Str(String::new())),
            ("min_corr", Value::Num(0.95))
        ])
    );
    assert!(!got.contains("loss_h2o\"],\"delta_da\""), "{got}");
    let _ = std::fs::remove_dir_all(&d);
}

#[test]
fn a_bad_list_is_an_error_of_the_instance() {
    let d = dir("bad");
    let (_, prov) = run(&d, r#"{"adducts":"17,x"}"#);
    assert!(prov.contains("err.flow.companions.list"), "{prov}");
    let _ = std::fs::remove_dir_all(&d);
}

#[test]
fn fewer_files_than_min_corr_samples_are_said_in_the_record() {
    let d = dir("few");
    let (text, prov) = run(&d, r#"{"min_corr_samples":6}"#);
    assert!(prov.contains("warn.flow.companions.corrFiles"), "{prov}");
    assert!(text.contains("\"edges\":[]"), "{text}");
    assert_eq!(text, engine(&[("min_corr_samples", Value::Num(6.0))]));
    let _ = std::fs::remove_dir_all(&d);
}
