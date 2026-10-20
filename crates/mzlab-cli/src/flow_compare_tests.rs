//! `meta.compare@1` run on the synthetic fixture of the identification tests: the same match step with different algorithms, no lab data.

use mzlab_core::json::{parse, Value};
use std::path::Path;

fn plan(step: &str, variants: &str) -> String {
    format!(
        r#"{{"format":"mzflow/2","inputs":[{{"name":"q.mgf","type":"spectra"}},{{"name":"lib.msp","type":"library"}}],"steps":[
        {{"id":"cmp","step":"meta.compare@1","in":{{"query":"file:q.mgf","library":"file:lib.msp"}},"params":{{"step":"{step}","variants":{variants}}}}}]}}"#
    )
}

const TWO: &str =
    r#"[{"algorithm":"spectral_entropy"},{"label":"mod","algorithm":"modified_cosine"}]"#;

fn run(dir: &Path, step: &str, variants: &str) -> Result<String, String> {
    let text = plan(step, variants);
    crate::flow::run_plan(text.as_bytes(), dir, &dir.join("work"), 1, None)
        .map(|(_, prov)| prov)
        .map_err(|e| e.key.to_string())
}

fn comparison(dir: &Path) -> Value {
    parse(&std::fs::read(dir.join("work/cmp.json")).unwrap()).unwrap()
}

/// Entropy and modified cosine agree on the first query; only the modified cosine reaches the analogue of the second. The record names every variant.
#[test]
fn the_variants_run_on_the_same_input_and_the_differences_are_counted() {
    let dir = crate::flow_ident_tests::fixture("compare");
    let prov = run(&dir, "spectra.match@1", TWO).unwrap();
    for id in ["cmp~spectral_entropy", "cmp~mod", "meta.compare"] {
        assert!(prov.contains(id), "{id} in {prov}");
    }
    let first = std::fs::read(dir.join("work/cmp.json")).unwrap();
    let c = comparison(&dir);
    assert_eq!(c.get("format").unwrap().str(), Some("comparison/1"));
    assert_eq!(c.get("type").unwrap().str(), Some("candidates"));
    let pair = &c.get("pairs").unwrap().arr()[0];
    let n = |k: &str| pair.get(k).unwrap().f64().unwrap();
    assert_eq!((n("both"), n("best_agree"), n("only_b")), (1.0, 1.0, 1.0));
    let keys = c.get("keys").unwrap().arr();
    let agree: Vec<bool> = keys
        .iter()
        .map(|k| k.get("agree").unwrap().bool().unwrap())
        .collect();
    assert_eq!(agree, [true, false]);
    // the same plan again gives the same bytes
    run(&dir, "spectra.match@1", TWO).unwrap();
    assert_eq!(std::fs::read(dir.join("work/cmp.json")).unwrap(), first);
    let _ = std::fs::remove_dir_all(&dir);
}

/// The checker of the comparisons runs before anything is expanded, so these come back as problems of the plan; only what the checker does not look at
/// (the label of a variant, the type of the output) is refused by the expansion.
#[test]
fn a_bad_comparison_is_refused_before_anything_runs() {
    let dir = crate::flow_ident_tests::fixture("compare_bad");
    let refused = |step: &str, variants: &str| {
        let text = plan(step, variants);
        crate::flow::run_plan(text.as_bytes(), &dir, &dir.join("work"), 1, None)
            .map(|_| String::new())
            .map_err(|e| {
                let list = e
                    .params
                    .iter()
                    .find(|(k, _)| *k == "problems")
                    .map(|p| p.1.clone());
                (e.key.to_string(), list.unwrap_or_default())
            })
    };
    let same = r#"[{"algorithm":"cosine"},{"label":"again","algorithm":"cosine"}]"#;
    let one = r#"[{"algorithm":"cosine"}]"#;
    let want = |p: &str| Err(("err.flow.check".to_string(), p.to_string()));
    assert_eq!(
        refused("spectra.match@1", same),
        want("cmp:err.flow.compare.sameVariant")
    );
    assert_eq!(
        refused("spectra.match@1", one),
        want("cmp:err.flow.compare.few")
    );
    assert_eq!(
        refused("nothing.here@1", TWO),
        want("cmp:err.flow.step.unknown")
    );
    let blank = r#"[{"algorithm":"cosine"},{"label":"a,b","algorithm":"spectral_entropy"}]"#;
    assert_eq!(
        refused("spectra.match@1", blank),
        Err(("err.flow.compare.variants".to_string(), String::new()))
    );
    let formulas = r#"[{"algorithm":"compose_isotopes"},{"label":"b","algorithm":"compose_isotopes","params":{"x":1}}]"#;
    assert_eq!(
        refused("formula.ms1@1", formulas),
        Err(("err.flow.compare.type".to_string(), String::new()))
    );
    let _ = std::fs::remove_dir_all(&dir);
}

/// The tolerances the page writes on a comparison are parameters of the compare instance, so the expanded plan passes the check and the run goes on.
#[test]
fn the_tolerances_of_the_page_are_accepted() {
    let dir = crate::flow_ident_tests::fixture("compare_tol");
    let text = plan("spectra.match@1", TWO).replace(
        r#""variants""#,
        r#""mz_tolerance_ppm":5,"rt_tolerance_min":0.2,"variants""#,
    );
    crate::flow::run_plan(text.as_bytes(), &dir, &dir.join("work"), 1, None).unwrap();
    let bad = plan("spectra.match@1", TWO)
        .replace(r#""variants""#, r#""rt_tolerance_min":-1,"variants""#);
    let e = crate::flow::run_plan(bad.as_bytes(), &dir, &dir.join("work"), 1, None).unwrap_err();
    assert_eq!(e.key, "err.flow.check");
    let _ = std::fs::remove_dir_all(&dir);
}

/// The answer of the checker for one case, as `flow_cases.rs` of `mzlab-feat` writes it: `instance|port|key|k=v,k=v` lines.
fn answer(plan: &str) -> Result<Vec<String>, String> {
    let plan = mzlab_core::flow::parse(plan.as_bytes()).map_err(|e| e.key.to_string())?;
    let mut reg = crate::flow::steps();
    reg.extend(mzlab_feat::steps::tabui());
    Ok(
        crate::flow_compare::check(&plan, &reg, mzlab_core::flow::Platform::Native)
            .iter()
            .map(|p| {
                let params: Vec<String> =
                    p.params.iter().map(|(k, v)| format!("{k}={v}")).collect();
                format!(
                    "{}|{}|{}|{}",
                    p.instance,
                    p.port,
                    p.error_key,
                    params.join(",")
                )
            })
            .collect(),
    )
}

/// The cases of `tests/data/flow_compare_cases.json` are also run through the checker of the page by `tests/test_flow_grafico.py`, so the two cannot drift
/// apart. `MZLAB_UPDATE_FLOW_CASES=1` writes the answers of this checker into the file (review the diff: the answer is the contract).
#[test]
fn the_shared_cases_of_the_comparison_give_the_written_answers() {
    let path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/data/flow_compare_cases.json"
    );
    let mut cases: serde_json::Value =
        serde_json::from_slice(&std::fs::read(path).unwrap()).unwrap();
    let update = std::env::var("MZLAB_UPDATE_FLOW_CASES").is_ok();
    for case in cases.as_array_mut().unwrap() {
        let name = case["name"].as_str().unwrap().to_string();
        let lines = answer(&case["plan"].to_string()).unwrap();
        if update {
            case["expect"] = serde_json::json!(lines);
        } else {
            assert_eq!(serde_json::json!(lines), case["expect"], "{name}");
        }
    }
    if update {
        std::fs::write(path, serde_json::to_string_pretty(&cases).unwrap() + "\n").unwrap();
    }
}
