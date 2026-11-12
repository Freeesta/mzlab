//! `anal/1` against the independent reference `tools/analisi_riferimento.py` (its own AME2020 masses and IUPAC abundances, exact convolution):
//! the engine's masses are OpenChemLib's (`nuclides`), the reference's NIST ones: they differ by up to 4 µDa per atom (I-127), hence 10 µDa on m/z.
//! 22 small molecules in `tests/dati/anal_golden.json` (CHNOS, F, P, Cl, Br, I, Na, drawn isotopes, cations and anions), then the errors.
use mzlab_feat::chem::analysis::analysis_json;
use serde_json::Value;

fn close(a: &Value, b: &Value, tol: f64, what: &str) {
    let (x, y) = (a.as_f64().unwrap(), b.as_f64().unwrap());
    assert!((x - y).abs() <= tol, "{what}: {x} vs {y}");
}

#[test]
fn every_golden_molecule_matches_the_reference() {
    let cases: Vec<Value> =
        serde_json::from_str(include_str!("../../../tests/dati/anal_golden.json")).unwrap();
    assert!(cases.len() >= 15);
    for c in &cases {
        let name = c["name"].as_str().unwrap();
        let got: Value =
            serde_json::from_str(&analysis_json(&c["request"].to_string()).unwrap()).unwrap();
        let (want, s) = (&c["expect"], &got["structure"]);
        assert_eq!(got["format"], "anal/1");
        assert_eq!(s["charge"], c["charge"], "{name}");
        close(&got["average"], &want["average"], 5e-5, name);
        let (gc, wc) = (
            got["composition"].as_array().unwrap(),
            want["composition"].as_array().unwrap(),
        );
        assert_eq!(gc.len(), wc.len(), "{name}");
        for (g, w) in gc.iter().zip(wc) {
            assert_eq!((&g["el"], &g["count"]), (&w["el"], &w["count"]), "{name}");
            close(&g["pct"], &w["pct"], 2e-4, name);
        }
        // sticks near the 0.01 % threshold may fall on either side: compare those above 0.02 %
        let big = |v: &Value| -> Vec<Value> {
            v.as_array()
                .unwrap()
                .iter()
                .filter(|p| p["rel"].as_f64().unwrap() >= 0.02)
                .cloned()
                .collect()
        };
        let (gp, wp) = (big(&got["pattern"]), big(&want["pattern"]));
        assert_eq!(gp.len(), wp.len(), "{name}: {gp:?} vs {wp:?}");
        for (g, w) in gp.iter().zip(&wp) {
            assert_eq!(g["off"], w["off"], "{name}");
            close(&g["mz"], &w["mz"], 1e-5, name);
            close(&g["rel"], &w["rel"], 2e-3, name);
            close(&g["abund"], &w["abund"], 2e-4, name);
        }
        // the lightest stick of a formula without heavier-than-monoisotopic drawn atoms is the monoisotopic m/z of `etich/1`
        let mono = if s["charge"] == 0 {
            &s["mass"]
        } else {
            &s["mz"]
        };
        close(&got["pattern"][0]["mz"], mono, 2e-6, name);
    }
}

#[test]
fn errors_have_their_keys() {
    let err = |r: &str| analysis_json(r).unwrap_err().key;
    assert_eq!(err("nope"), "error.anal.request");
    assert_eq!(err(r#"{"atoms":[]}"#), "error.anal.empty");
    assert_eq!(err(r#"{"atoms":[{"el":"R#"}]}"#), "error.etich.element");
    assert_eq!(
        err(r#"{"atoms":[{"el":"C","iso":99}]}"#),
        "error.etich.isotope"
    );
}

#[test]
fn a_radical_cation_keeps_its_kind_and_mz() {
    // CH4+• drawn as a charged carbon with a doublet radical mark
    let s: Value = serde_json::from_str(
        &analysis_json(r#"{"atoms":[{"el":"C","h":4,"q":1,"r":2}]}"#).unwrap(),
    )
    .unwrap();
    assert_eq!(s["structure"]["kind"], "radicalCation");
    close(&s["pattern"][0]["mz"], &s["structure"]["mz"], 2e-6, "CH4+•");
}
