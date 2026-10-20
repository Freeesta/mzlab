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

#[test]
fn a_bad_comparison_is_refused_before_anything_runs() {
    let dir = crate::flow_ident_tests::fixture("compare_bad");
    let same = r#"[{"algorithm":"cosine"},{"label":"again","algorithm":"cosine"}]"#;
    assert_eq!(
        run(&dir, "spectra.match@1", same),
        Err("err.flow.compare.sameVariant".into())
    );
    let one = r#"[{"algorithm":"cosine"}]"#;
    assert_eq!(
        run(&dir, "spectra.match@1", one),
        Err("err.flow.compare.variants".into())
    );
    assert_eq!(
        run(&dir, "formula.ms1@1", TWO),
        Err("err.flow.compare.type".into())
    );
    assert_eq!(
        run(&dir, "nothing.here@1", TWO),
        Err("err.flow.compare.step".into())
    );
    let _ = std::fs::remove_dir_all(&dir);
}
