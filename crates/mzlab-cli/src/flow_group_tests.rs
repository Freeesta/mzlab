//! `features.group@1` in a plan: a synthetic per-ion table (invented masses: two compounds as [M+H]+ and [M+Na]+ with their M+1, a co-eluting third
//! compound with other levels, a lone row) gives the `ionnet/1` of the engine called with the parameters as the page sends them (descriptor defaults as
//! JSON, the `flowtab` path), so `flow run` and the browser write the same bytes; `--set` reaches the engine, a wrong species is refused, a second run is equal.

use std::path::{Path, PathBuf};

use mzlab_core::json::{parse, Out, Value};
use mzlab_feat::recipe::Recipe;
use mzlab_feat::store::{write_table, Origin};
use mzlab_feat::table::{Aligned, CELL_DETECTED};

const FILES: usize = 5;

fn table() -> Vec<u8> {
    let levels = [
        [1.0, 2.0, 3.0, 4.0, 5.0],
        [5.0, 1.0, 4.0, 2.0, 3.0],
        [2.0, 2.5, 1.0, 4.5, 3.0],
    ];
    // (m/z, RT, compound, share of its level)
    let rows = [
        (301.141_476, 3.00, 0, 1.0),
        (302.144_831, 3.00, 0, 0.14),
        (323.123_418, 3.00, 0, 0.4),
        (415.212_076, 3.02, 1, 1.0),
        (437.194_018, 3.02, 1, 0.3),
        (416.215_431, 3.02, 1, 0.19),
        (350.300_100, 3.01, 2, 1.0),
        (512.400_000, 6.00, 2, 1.0),
    ];
    let mut t = Aligned::with_capacity(FILES, rows.len());
    for (mz, rt, c, share) in rows {
        t.mz.push(mz);
        t.rt.push(rt);
        for f in 0..FILES {
            let h = 1e5 * levels[c][f] * share;
            t.area.push(h * 6.0);
            t.height.push(h as f32);
            t.apex_rt.push(rt as f32);
            t.width.push(0.08);
            t.state.push(CELL_DETECTED);
        }
    }
    let origin = Origin {
        inputs: (0..FILES)
            .map(|k| (format!("g{k}.mzML"), format!("{k:02x}").repeat(32)))
            .collect(),
        ..Origin::default()
    };
    write_table(&t, &Recipe::default(), &origin)
}

fn dir(tag: &str) -> PathBuf {
    let d = std::env::temp_dir().join(format!("mzlab_cli_group_{tag}_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(d.join("work")).unwrap();
    std::fs::write(d.join("table.mzfeat"), table()).unwrap();
    d
}

fn plan(params: &str) -> String {
    format!(
        r#"{{"format":"mzflow/2","inputs":[{{"name":"table.mzfeat","type":"table"}}],"steps":[
        {{"id":"grp","step":"features.group@1","in":{{"table":"file:table.mzfeat"}},"params":{params}}}]}}"#
    )
}

fn run(d: &Path, params: &str) -> String {
    let (run, prov) =
        crate::flow::run_plan(plan(params).as_bytes(), d, &d.join("work"), 1, None).unwrap();
    assert!(
        run.skipped.is_empty() && !prov.contains("\"error\""),
        "{prov}"
    );
    std::fs::read_to_string(d.join("work/grp.json")).unwrap()
}

/// What the browser sends: the defaults of the descriptor merged with the parameters of the instance, as JSON.
fn page_params(over: &[(&str, Value)]) -> String {
    let reg = crate::flow::steps();
    let step = mzlab_core::step::find(&reg, "features.group").unwrap();
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
    mzlab_feat::ionnet::group_text(&params, &table()).unwrap()
}

#[test]
fn the_plan_writes_the_text_of_the_browser_engine() {
    let d = dir("engine");
    let got = run(&d, "{}");
    assert_eq!(got, engine(&[]));
    let v = parse(got.as_bytes()).unwrap();
    assert_eq!(v.get("format").and_then(Value::str), Some("ionnet/1"));
    let g: Vec<f64> = v
        .get("group")
        .unwrap()
        .arr()
        .iter()
        .map(|x| x.f64().unwrap())
        .collect();
    // compound 0 = rows 0-2, compound 1 = rows 3-5, the co-eluting row and the lone row alone: no row lost
    assert_eq!(g, vec![1.0, 1.0, 1.0, 2.0, 2.0, 2.0, 3.0, 4.0]);
    let ions: Vec<Option<&str>> = v.get("ion").unwrap().arr().iter().map(Value::str).collect();
    assert_eq!(ions[0], Some("[M+H]+"));
    assert_eq!(ions[2], Some("[M+Na]+"));
    assert_eq!(ions[1], None);
    assert!(got.contains("\"EdgeType\":\"MS1 annotation\""), "{got}");
    assert_eq!(got, run(&d, "{}"));
    let _ = std::fs::remove_dir_all(&d);
}

#[test]
fn set_parameters_reach_the_engine() {
    let d = dir("set");
    let only_h = run(&d, r#"{"ions_pos":"[M+H]+,[M+NH4]+"}"#);
    assert_eq!(
        only_h,
        engine(&[("ions_pos", Value::Str("[M+H]+,[M+NH4]+".into()))])
    );
    assert!(!only_h.contains("[M+Na]+ "), "{only_h}");
    let neg = run(&d, r#"{"polarity":"negative","rt_tol":0.05}"#);
    assert_eq!(
        neg,
        engine(&[
            ("polarity", Value::Str("negative".into())),
            ("rt_tol", Value::Num(0.05))
        ])
    );
    assert!(
        neg.contains("\"polarity\":-1") && !neg.contains("MS1 annotation"),
        "{neg}"
    );
    let _ = std::fs::remove_dir_all(&d);
}

#[test]
fn an_unknown_species_is_an_error_of_the_instance() {
    let d = dir("bad");
    let (_, prov) = crate::flow::run_plan(
        plan(r#"{"ions_pos":"[M+Zz]+"}"#).as_bytes(),
        &d,
        &d.join("work"),
        1,
        None,
    )
    .unwrap();
    assert!(prov.contains("err.flow.group.ion"), "{prov}");
    let _ = std::fs::remove_dir_all(&d);
}
