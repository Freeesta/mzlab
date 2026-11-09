//! The read estimate of `spectra.ms2` on the fixture `tests/dati/stima_lettura.json`, by the Rust route both the page (`flow_ms2`, one file) and
//! `flow run` call: below the estimate the step is refused with `error.memoryBudget` (phase `read`) and the numbers of the fixture, above it writes the MGF.
#[path = "lettura_comune/mod.rs"]
mod comune;

use mzlab_core::budget::MemBudget;
use mzlab_feat::flow_ms2::{ms2_text, Ms2Params};

#[test]
fn the_read_is_refused_below_the_estimate_of_the_fixture() {
    let s = comune::spec();
    let b = comune::mzml(&s);
    let mb = s["mb"].as_u64().unwrap();
    for (budget, via) in comune::cases(&s) {
        let p = Ms2Params {
            cap: MemBudget::mb(budget).cap(),
            ..Ms2Params::from_params(&[])
        };
        let got = ms2_text(&p, &[("stima.mzML", &b)], None);
        if via == "memory" {
            assert!(got.unwrap().contains("BEGIN IONS"), "{budget}");
            continue;
        }
        let e = got.unwrap_err();
        assert_eq!(e.key, "error.memoryBudget");
        let q = |k: &str| e.params.iter().find(|p| p.0 == k).map(|p| p.1.clone());
        assert_eq!(
            (q("fase"), q("stima_mb"), q("budget_mb")),
            (Some("read".into()), Some(mb.to_string()), Some(budget.to_string())),
            "{e:?}"
        );
    }
}
