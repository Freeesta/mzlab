//! `features.estimate@1` in a plan: each file's `estimates/1` holds the numbers `run::preview` (the page's engine) measures on the same index, the polarity
//! and the axis of the plan reach the recipe, and a second run writes the same bytes. Synthetic mzML only.

use mzlab_core::idxread::Index;
use mzlab_core::json::{parse, Value};
use mzlab_core::mzidx::build_mzml_to_vec;
use mzlab_core::upload::prepare;
use mzlab_feat_wasm::run;

use crate::flow_run_tests::fixture;
use crate::tests::mzml;

const PLAN: &str = r#"{"format":"mzflow/2","inputs":[{"name":"s_t000.mzML","type":"raw"},{"name":"s_t010.mzML","type":"raw"},{"name":"s_t030.mzML","type":"raw"}],"steps":[
{"id":"idx","step":"index.mzidx@1","in":{"source":["file:s_t000.mzML","file:s_t010.mzML","file:s_t030.mzML"]}},
{"id":"est","step":"features.estimate@1","in":{"index":["idx.index"]},"params":PARAMS}]}"#;

fn run_in(d: &std::path::Path, params: &str) -> Vec<String> {
    let plan = PLAN.replace("PARAMS", params);
    let work = d.join("work");
    std::fs::create_dir_all(&work).unwrap();
    let (run, prov) =
        crate::flow::run_plan(plan.as_bytes(), &d.join("data"), &work, 1, None).unwrap();
    assert!(
        run.skipped.is_empty() && !prov.contains("\"error\""),
        "{prov}"
    );
    ["s_t000", "s_t010", "s_t030"]
        .iter()
        .map(|k| std::fs::read_to_string(work.join(format!("est-{k}.json"))).unwrap())
        .collect()
}

/// The page's measurements of one file: the preview of the recipe of the file with the axis of the plan.
fn page(name: &str, bytes: &[u8], axis: &str) -> Value {
    let mut src = bytes.to_vec();
    let p = prepare(&mut src, bytes.len() as u64, name).unwrap();
    let idx = build_mzml_to_vec(&p.table, bytes, 64 << 20).unwrap().0;
    let mut index = Index::open(idx.clone(), idx.len() as u64).unwrap();
    let mut r = run::recipe_for_estimated("", &mut index).unwrap();
    let v = [("axis".to_string(), Value::Str(axis.into()))];
    mzlab_feat::steps::recipe_map::apply(&mut r, "features.detect", &v).unwrap();
    parse(run::preview(&mut index, &r, false).unwrap().as_bytes()).unwrap()
}

#[test]
fn the_plan_measures_what_the_page_measures_and_writes_the_same_bytes() {
    let (d, files) = fixture("estimate");
    for axis in ["ppm", "Da"] {
        let got = run_in(&d, &format!("{{\"axis\":\"{axis}\"}}"));
        for ((name, bytes), text) in files.iter().zip(&got) {
            let doc = parse(text.as_bytes()).unwrap();
            assert_eq!(doc.get("schema").and_then(Value::str), Some("estimates/1"));
            assert_eq!(doc.get("axis").and_then(Value::str), Some(axis));
            let preview = page(name, bytes, axis);
            for k in [
                "n",
                "noise",
                "min_height",
                "ppm",
                "fwhm",
                "rt_tol",
                "dead",
                "da",
            ] {
                let text = |v: &Value| v.get(k).map(mzlab_core::flow::value_text);
                assert_eq!(text(&doc), text(&preview), "{name} {axis} {k}");
            }
            assert!(doc.get("intensities").is_none() && doc.get("examples").is_none());
        }
        assert_eq!(got, run_in(&d, &format!("{{\"axis\":\"{axis}\"}}")));
    }
    let _ = std::fs::remove_dir_all(&d);
}

#[test]
fn polarity_reaches_the_recipe() {
    let (d, _) = fixture("estimate_pol");
    let pos = run_in(&d, "{\"polarity\":1}");
    let neg = run_in(&d, "{\"polarity\":-1}");
    // the files hold positive scans only: nothing to measure with the other polarity
    assert!(pos[0].contains("\"polarity\":1"), "{}", pos[0]);
    assert!(neg[0].contains("\"polarity\":-1"), "{}", neg[0]);
    assert_ne!(pos, neg);
    let _ = std::fs::remove_dir_all(&d);
}
