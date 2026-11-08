//! The detection estimate on the fixture `tests/dati/stima_detect.json`: `detect_estimate_json` (the `.wasm` `estimate_detect`) gives the expected MB and
//! way at every budget, `detect_fits` (asked before `detect` by the page, the app and `flow run`) agrees with it, and its refusal carries the phase and
//! the numbers of the estimate.
#![cfg(feature = "detect")]

#[path = "stima_comune/mod.rs"]
mod comune;

use mzlab_core::idxread::Index;
use mzlab_feat_wasm::{run, stima};

fn open() -> Index<Vec<u8>> {
    let b = comune::index(&comune::spec());
    let n = b.len() as u64;
    Index::open(b, n).unwrap()
}

#[test]
fn the_estimate_and_the_refusal_follow_the_fixture_at_every_budget() {
    let s = comune::spec();
    let mb = s["mb"].as_u64().unwrap();
    let mut ix = open();
    let r = run::recipe_for("", &mut ix).unwrap();
    for (budget, via) in comune::cases(&s) {
        let got = stima::detect_estimate_json(&mut ix, &r, budget).unwrap();
        assert_eq!(
            got,
            format!("{{\"mb\":{mb},\"via\":\"{via}\"}}"),
            "{budget}"
        );
        let fit = stima::detect_fits(&mut ix, &r, (budget * 1048576.0) as usize);
        match via.as_str() {
            "memory" => assert!(run::detect(&mut ix, "stima", &r).is_ok() && fit.is_ok()),
            _ => {
                let e = fit.unwrap_err().to_string();
                let want =
                    format!("error.memoryBudget fase=detect stima_mb={mb} budget_mb={budget} ");
                assert!(e.starts_with(&want), "{e}");
            }
        }
    }
}

/// No false refusal: the high-resolution examples of the site fit the default budget of the page (the smallest one, 192 MB) with their own recipe.
#[test]
fn the_examples_of_the_site_fit_the_default_budget_of_the_page() {
    let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../mzlab/web/esempi");
    for n in [
        "HRMS_t000.mzML",
        "HRMS_t005.mzML",
        "HRMS_t010.mzML",
        "HRMS_t030.mzML",
    ] {
        let b = std::fs::read(dir.join(n)).unwrap();
        let mut src = b.clone();
        let p = mzlab_core::upload::prepare(&mut src, b.len() as u64, n).unwrap();
        let idx = mzlab_core::mzidx::build_mzml_to_vec(&p.table, &b, 64 << 20)
            .unwrap()
            .0;
        let size = idx.len() as u64;
        let mut ix = Index::open(idx, size).unwrap();
        let r = run::recipe_for("", &mut ix).unwrap();
        let got = stima::detect_estimate_json(&mut ix, &r, 0.0).unwrap();
        assert!(got.ends_with("\"via\":\"memory\"}"), "{n} {got}");
        assert!(stima::detect_fits(
            &mut ix,
            &r,
            mzlab_core::budget::MemBudget::from_request(0.0).cap()
        )
        .is_ok());
    }
}
