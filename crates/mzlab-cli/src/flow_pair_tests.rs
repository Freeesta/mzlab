//! `features.pair@1` in a plan: a positive and a negative synthetic table (invented neutral masses; two known pairs, a mass decoy at the right RT, an RT
//! decoy at the right mass and a row alone) give the pairs and the `pair/1` of the engine called with the parameters as the page sends them
//! (descriptor defaults as JSON, the `flowtab` path), so `flow run` and the browser write the same bytes; `--set` reaches the engine and a bad value is refused.

use std::path::{Path, PathBuf};

use mzlab_core::json::{parse, Out, Value};
use mzlab_feat::recipe::Recipe;
use mzlab_feat::store::{write_table, Origin};
use mzlab_feat::table::{Aligned, CELL_DETECTED};

const H: f64 = 1.007276;

fn table(rows: &[(f64, f64)]) -> Vec<u8> {
    let mut t = Aligned::with_capacity(2, rows.len());
    for (mz, rt) in rows {
        t.mz.push(*mz);
        t.rt.push(*rt);
        for h in [1e5, 2e5] {
            t.area.push(h * 6.0);
            t.height.push(h as f32);
            t.apex_rt.push(*rt as f32);
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

/// Neutral masses A 180.06339, B 250.12, C 310.2 (alone in the positive table).
fn tables() -> [(&'static str, Vec<u8>); 2] {
    [
        (
            "p.mzfeat",
            table(&[(180.06339 + H, 5.0), (250.12 + H, 8.2), (310.2 + H, 3.1)]),
        ),
        (
            "n.mzfeat",
            table(&[
                (180.06339 - H, 5.03),
                (250.12 - H, 8.18),
                (310.2 - H, 9.9),
                (310.3 - H, 3.12),
            ]),
        ),
    ]
}

fn dir(tag: &str) -> PathBuf {
    let d = std::env::temp_dir().join(format!("mzlab_cli_pair_{tag}_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(d.join("work")).unwrap();
    for (n, b) in tables() {
        std::fs::write(d.join(n), b).unwrap();
    }
    d
}

fn plan(params: &str) -> String {
    format!(
        r#"{{"format":"mzflow/2","inputs":[{{"name":"p.mzfeat","type":"table"}},{{"name":"n.mzfeat","type":"table"}}],"steps":[
        {{"id":"pr","step":"features.pair@1","in":{{"pos":"file:p.mzfeat","neg":"file:n.mzfeat"}},"params":{params}}}]}}"#
    )
}

fn run(d: &Path, params: &str) -> (String, String) {
    let (run, prov) =
        crate::flow::run_plan(plan(params).as_bytes(), d, &d.join("work"), 1, None).unwrap();
    assert!(run.skipped.is_empty(), "{prov}");
    let read = |n: &str| std::fs::read_to_string(d.join("work").join(n)).unwrap_or_default();
    (read("pr.csv"), read("pr.pair.json"))
}

/// What the browser sends: the defaults of the descriptor merged with the parameters of the instance, as JSON.
fn page_params(over: &[(&str, Value)]) -> String {
    let reg = crate::flow::steps();
    let step = mzlab_core::step::find(&reg, "features.pair").unwrap();
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
    let t = tables();
    let m = mzlab_feat::pair::pair_text(&params, (t[0].0, &t[0].1), (t[1].0, &t[1].1)).unwrap();
    (m.csv, m.doc)
}

#[test]
fn the_plan_writes_the_bytes_of_the_browser_engine() {
    let d = dir("engine");
    let (csv, doc) = run(&d, "{}");
    assert_eq!((csv.clone(), doc.clone()), engine(&[]));
    // two pairs; the RT decoy (mass of C, 6.8 min away) and the mass decoy (0.1 Da off) stay unmatched, and so does C
    let v = parse(doc.as_bytes()).unwrap();
    assert_eq!(v.get("pairs").unwrap().arr().len(), 2, "{doc}");
    assert_eq!(
        v.get("unmatched").unwrap().get("neg").unwrap().arr().len(),
        2,
        "{doc}"
    );
    assert_eq!(
        v.get("unmatched").unwrap().get("pos").unwrap().arr().len(),
        1,
        "{doc}"
    );
    assert_eq!(csv.lines().count(), 1 + 2 + 3, "{csv}");
    assert!(
        csv.lines().nth(1).unwrap().contains("[M+H]+ / [M-H]-"),
        "{csv}"
    );
    assert_eq!(run(&d, "{}").0, csv);
    let _ = std::fs::remove_dir_all(&d);
}

#[test]
fn set_parameters_reach_the_engine() {
    let d = dir("set");
    let (csv, doc) = run(&d, r#"{"rt_tol":0.01,"mass_tol":0.2,"mass_unit":"da"}"#);
    assert_eq!(
        (csv, doc),
        engine(&[
            ("rt_tol", Value::Num(0.01)),
            ("mass_tol", Value::Num(0.2)),
            ("mass_unit", Value::Str("da".into()))
        ])
    );
    let _ = std::fs::remove_dir_all(&d);
}

#[test]
fn a_value_out_of_range_is_refused_by_the_check_and_by_the_engine() {
    let d = dir("bad");
    let e = crate::flow::run_plan(
        plan(r#"{"rt_tol":-1}"#).as_bytes(),
        &d,
        &d.join("work"),
        1,
        None,
    )
    .unwrap_err();
    assert!(
        e.params
            .iter()
            .any(|(_, v)| v.contains("err.flow.param.range")),
        "{e:?}"
    );
    // a page that skipped the check still gets the engine's own refusal; an adduct of the wrong charge sign is one too
    let t = tables();
    for bad in [r#"{"rt_tol":-1}"#, r#"{"pos_adducts":"[M-H]-"}"#] {
        let p = mzlab_feat::flow_ms2::json_params(bad).unwrap();
        let e = mzlab_feat::pair::pair_text(&p, (t[0].0, &t[0].1), (t[1].0, &t[1].1)).unwrap_err();
        assert_eq!(e.key, "err.pair.param", "{bad}");
    }
    let _ = std::fs::remove_dir_all(&d);
}

/// What an agent sees: `flow defaults` lists the parameters with their defaults and a refused value has its own exit code.
#[test]
fn an_agent_gets_the_defaults_and_a_distinct_exit_code() {
    let args = |a: &[&str]| a.iter().map(|s| s.to_string()).collect::<Vec<_>>();
    let o = crate::execute(&args(&["flow", "defaults", "features.pair", "--json"]));
    assert_eq!(o.code, 0, "{}", o.stderr);
    let text = String::from_utf8_lossy(&o.stdout).into_owned();
    for k in [
        "rt_tol",
        "mass_tol",
        "mass_unit",
        "pos_adducts",
        "neg_adducts",
        "[M+H]+",
        "[M-H]-",
    ] {
        assert!(text.contains(k), "{k} missing from {text}");
    }
    let d = dir("agent");
    let (plan_path, inp, out) = (
        d.join("plan.json").to_string_lossy().to_string(),
        d.to_string_lossy().to_string(),
        d.join("out").to_string_lossy().to_string(),
    );
    std::fs::write(&plan_path, plan("{}")).unwrap();
    let go = |extra: &[&str]| {
        let mut a = vec![
            "flow", "run", &plan_path, "--in", &inp, "--out", &out, "--json",
        ];
        a.extend_from_slice(extra);
        crate::execute(&args(&a))
    };
    let ok = go(&[]);
    assert_eq!(ok.code, 0, "{}", ok.stderr);
    assert!(parse(&ok.stdout).is_ok());
    let bad = go(&["--set", "pr.rt_tol=-1"]);
    assert_eq!(bad.code, 3, "{}", bad.stderr);
    let doc = parse(&bad.stdout).expect("error/1 on stdout");
    assert_eq!(
        doc.get("error_key").and_then(Value::str),
        Some("err.flow.check")
    );
    let _ = std::fs::remove_dir_all(&d);
}
