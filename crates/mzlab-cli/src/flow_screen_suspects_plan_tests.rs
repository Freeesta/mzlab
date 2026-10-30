//! `screen.suspects@1` through the plan runner and the registry: the cached second run, the plan check on a unit-resolution table and the defaults of the descriptor.
//! The invented list, table and MGF are those of `flow_screen_suspects_tests`.

use super::flow_screen_suspects_tests::{csv, mgf, table_bytes};
use mzlab_core::json::Value;
use mzlab_feat::flow_screen_suspects::Params;

#[test]
fn the_step_runs_in_a_plan_and_the_second_run_is_cached() {
    let d = std::env::temp_dir().join(format!("mzlab_cli_screen_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(d.join("work")).unwrap();
    std::fs::write(d.join("table.mzfeat"), table_bytes(1)).unwrap();
    std::fs::write(d.join("list.csv"), csv()).unwrap();
    std::fs::write(d.join("q.mgf"), mgf()).unwrap();
    let plan = r#"{"format":"mzflow/2","inputs":[{"name":"table.mzfeat","type":"table"},{"name":"list.csv","type":"library"},{"name":"q.mgf","type":"spectra"}],"steps":[
        {"id":"s","step":"screen.suspects@1","in":{"table":"file:table.mzfeat","suspects":["file:list.csv"],"spectra":"file:q.mgf"},"params":{"min_fragments":2}}]}"#;
    let go = || {
        crate::flow::run_plan_cached(plan.as_bytes(), &d, &d.join("work"), 1, None, Some(false))
            .unwrap()
    };
    let (run, prov) = go();
    assert!(
        run.skipped.is_empty() && !prov.contains("\"error\""),
        "{prov}"
    );
    let first = std::fs::read(d.join("work/s.json")).unwrap();
    assert!(String::from_utf8_lossy(&first).starts_with("{\"format\":\"screen/1\""));
    let (_, again) = go();
    assert!(again.contains("\"cached\":true"), "{again}");
    assert_eq!(std::fs::read(d.join("work/s.json")).unwrap(), first);
    let _ = std::fs::remove_dir_all(&d);
}

/// The step requires high resolution: a table of unit-resolution files is refused by the plan check, one of high-resolution files is not.
#[test]
fn the_step_is_not_offered_on_a_unit_resolution_table() {
    use mzlab_core::flow::{check, parse, Platform};
    let plan = |resolution: &str| {
        format!(
            r#"{{"inputs":[{{"name":"t.mzfeat","sha256":"aa","type":"table","facets":{{"resolution":["{resolution}"]}}}},{{"name":"l.csv","sha256":"bb","type":"library"}}],
            "steps":[{{"id":"s","step":"screen.suspects@1","in":{{"table":"file:t.mzfeat","suspects":["file:l.csv"]}}}}]}}"#
        )
    };
    let keys = |r: &str| {
        let p = parse(plan(r).as_bytes()).unwrap();
        check(&p, &crate::flow::steps(), Platform::Native)
            .iter()
            .map(|p| format!("{}|{:?}", p.error_key, p.params))
            .collect::<Vec<_>>()
    };
    assert!(keys("hr").is_empty(), "{:?}", keys("hr"));
    assert!(
        keys("lr")
            .iter()
            .any(|k| k.starts_with("err.flow.facet") && k.contains("resolution")),
        "{:?}",
        keys("lr")
    );
}

/// The defaults of the descriptor are the ones of `Params` (and the adducts and ppm of the Attributio screening).
#[test]
fn the_defaults_of_the_descriptor_are_those_of_the_code() {
    let step = crate::flow::steps()
        .into_iter()
        .find(|s| s.id == "screen.suspects")
        .unwrap();
    let d = Params::from_params(&[]);
    let default = |name: &str| {
        step.params
            .iter()
            .find(|p| p.name == name)
            .unwrap()
            .default
            .clone()
    };
    assert_eq!(default("ppm"), Some(Value::Num(d.ppm)));
    assert_eq!(default("adducts"), Some(Value::Str(d.adducts.clone())));
    assert_eq!(
        default("max_hits_per_feature"),
        Some(Value::Num(d.max_hits_per_feature as f64))
    );
    assert_eq!(default("ms2_rt_min"), Some(Value::Num(d.ms2_rt_min)));
    assert!(
        default("rt_window_min").is_none()
            && default("min_fragments").is_none()
            && d.rt_window_min.is_none()
            && d.min_fragments.is_none()
    );
    assert_eq!(d.ppm, mzlab_feat::attributio::reglists::screen_types::PPM);
    assert_eq!(d.adducts, mzlab_feat::flow_screen_suspects::DEFAULT_ADDUCTS);
}
