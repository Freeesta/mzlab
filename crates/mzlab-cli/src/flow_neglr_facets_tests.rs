//! The check of `flow run` on the facets the files give (`nature::fill_facets`): a step that needs high resolution is refused on unit-resolution
//! files, a plan that writes its facets keeps them. Child of `flow_neglr_tests.rs`, whose series and plans it reuses.

use std::path::Path;

use super::{error_of, fixture, formula_plan, run, with, DIG};

/// The problems of the check of `text` once its inputs are the files of `d`, as `flow run` sees them: `instance.port:key k=v,…`.
fn problems(d: &Path, text: &str) -> Vec<String> {
    use mzlab_core::flow::{check, parse, Platform};
    let mut plan = parse(text.as_bytes()).unwrap();
    crate::flow_run::fill_inputs(&mut plan, |n| d.join(n)).unwrap();
    let mut out = vec![];
    for p in check(&plan, &crate::flow::steps(), Platform::Native) {
        let kv: Vec<_> = p.params.iter().map(|(k, v)| format!("{k}={v}")).collect();
        out.push(format!(
            "{} {}",
            crate::flow_run::problem_text(&p),
            kv.join(",")
        ));
    }
    out
}

#[test]
fn formula_ms1_on_unit_resolution_files_is_refused_by_the_check() {
    let d = fixture("nl_f1lr", -1, false);
    let want = "f1.features:err.flow.facet facet=resolution,step=formula.ms1,want=hr,have=lr";
    assert_eq!(problems(&d, &formula_plan(None)), [want]);
    let e = crate::flow::run_plan(formula_plan(None).as_bytes(), &d, &d.join("work"), 2, None)
        .expect_err("formula.ms1 on unit-resolution files");
    assert_eq!(e.key, "err.flow.check");
    // a plan that writes its facets by hand keeps them
    let facets = r#""type":"raw","facets":{"resolution":["hr"]}}"#;
    let hand = formula_plan(None).replace(r#""type":"raw"}"#, facets);
    assert_eq!(problems(&d, &hand), Vec::<String>::new());
    // high-resolution files of the same series pass
    let hr = fixture("nl_f1hr", -1, true);
    assert_eq!(problems(&hr, &formula_plan(None)), Vec::<String>::new());
    let _ = std::fs::remove_dir_all(&d);
    let _ = std::fs::remove_dir_all(&hr);
}

#[test]
fn a_unit_resolution_plan_without_facets_takes_them_from_the_files() {
    let d = fixture("nl_g6f", -1, false);
    let prov = run(&d, &with(&format!("{DIG}]}}")));
    for id in ["gen", "lnk", "flt", "pos", "net", "dig"] {
        assert_eq!(error_of(&prov, id), None, "{id}");
    }
    let _ = std::fs::remove_dir_all(&d);
}
