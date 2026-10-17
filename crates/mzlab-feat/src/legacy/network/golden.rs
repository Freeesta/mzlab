//! `best_edges` against the frozen answers of the former Python engine (the `mass_network` of mzFinder, `tests/golden/feat/tp_ms2.json`): the same
//! edges (source, product, kinetic consistency) and the exact change of mass (rounded to 1e-5 there). The rule tables have the same changes.

use super::*;

fn rules() -> Rules {
    Rules::parse(include_bytes!("../../../../../TP_Mine/data/regole_tp.json")).unwrap()
}

#[test]
fn the_edges_match_the_frozen_answers() {
    let g: serde_json::Value =
        serde_json::from_str(include_str!("../../../../../tests/golden/feat/tp_ms2.json")).unwrap();
    let r = rules();
    let cases = g["network"].as_array().unwrap();
    assert!(cases.len() >= 10);
    for (i, case) in cases.iter().enumerate() {
        let mut forms = vec![counts(case["parent"].as_str().unwrap())];
        forms.extend(
            case["forms"]
                .as_array()
                .unwrap()
                .iter()
                .map(|f| counts(f.as_str().unwrap())),
        );
        let mut tmax = vec![Some(0.0)];
        tmax.extend(case["tmax"].as_array().unwrap().iter().map(|t| t.as_f64()));
        let mut got: Vec<(usize, usize, &str, f64)> = best_edges(&forms, &tmax, &r)
            .iter()
            .map(|e| {
                (
                    e.from,
                    e.to,
                    e.kinetic,
                    neutral_mass(&forms[e.to]) - neutral_mass(&forms[e.from]),
                )
            })
            .collect();
        got.sort_by_key(|a| (a.0, a.1));
        let want = case["edges"].as_array().unwrap();
        assert_eq!(got.len(), want.len(), "case {i}: {got:?}");
        for (e, w) in got.iter().zip(want) {
            assert_eq!(
                (e.0 as u64, e.1 as u64),
                (w[0].as_u64().unwrap(), w[1].as_u64().unwrap()),
                "case {i}"
            );
            assert_eq!(e.2, w[2].as_str().unwrap(), "case {i} kinetic of {e:?}");
            assert!(
                (e.3 - w[3].as_f64().unwrap()).abs() < 1.5e-5,
                "case {i} dm of {e:?}"
            );
        }
    }
}
