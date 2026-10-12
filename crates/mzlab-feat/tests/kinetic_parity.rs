//! Kinetics against the frozen Python answers (`tools/feat_riferimento.py OUT --solo cinetica --golden`): same shape chosen, rates equal, scale and
//! AICc within 1e-6 relative, same leave-one-out shapes, same generation; two runs give the same bytes.

use mzlab_feat::kinetic::{generation, ChainBank, FormBank, Resample, K_PARENT_MIN, MARGIN};
use serde_json::Value;
use sha2::{Digest, Sha256};

fn golden(name: &str) -> Vec<Value> {
    let p = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../tests/golden/feat")
        .join(name);
    serde_json::from_slice::<Value>(&std::fs::read(p).unwrap())
        .unwrap()
        .as_array()
        .unwrap()
        .clone()
}

fn floats(v: &Value) -> Vec<f64> {
    v.as_array()
        .unwrap()
        .iter()
        .map(|x| x.as_f64().unwrap())
        .collect()
}

fn close(a: f64, b: f64, what: &str) {
    assert!(
        (a - b).abs() <= 1e-6 * b.abs().max(1e-12),
        "{what}: {a} vs {b}"
    );
}

#[test]
fn shapes_match_python() {
    for case in golden("cinetica_forme.json") {
        let name = case["name"].as_str().unwrap();
        let (t, y) = (floats(&case["times"]), floats(&case["y"]));
        let bank = FormBank::new(&t);
        let fits = bank.fit(&y);
        let want = case["fits"].as_array().unwrap();
        assert_eq!(fits.len(), want.len(), "{name}");
        for (f, w) in fits.iter().zip(want) {
            assert_eq!(
                f.form.name(),
                w["form"].as_str().unwrap(),
                "{name}: order of the shapes"
            );
            // the rise-and-fall shape is symmetric in its two rates (formation and decay swap): compare the sorted pair
            let (wr, n) = (floats(&w["rates"]), floats(&w["rates"]).len());
            let mut got = f.rates[..n].to_vec();
            got.sort_by(f64::total_cmp);
            let mut wr = wr;
            wr.sort_by(f64::total_cmp);
            for (g, r) in got.iter().zip(&wr) {
                close(*g, *r, &format!("{name} rate"));
            }
            close(f.aicc, w["aicc"].as_f64().unwrap(), &format!("{name} aicc"));
            close(
                f.scale,
                w["scale"].as_f64().unwrap(),
                &format!("{name} scale"),
            );
        }
        let st = bank.stability(&y, Resample::LeaveOneOut).unwrap();
        let loo = case["leave_one_out"].as_array().unwrap();
        let same = loo
            .iter()
            .filter(|s| s.as_str().unwrap() == fits[0].form.name())
            .count();
        assert_eq!(
            (st.n as usize, st.same as usize),
            (loo.len(), same),
            "{name}: leave one out"
        );
    }
}

#[test]
fn generation_matches_python() {
    let cases = golden("cinetica_generazione.json");
    let t = floats(&cases[0]["times"]);
    let (forms, chain) = (FormBank::new(&t), ChainBank::new(&t, K_PARENT_MIN));
    for case in cases {
        let name = case["name"].as_str().unwrap();
        let g = generation(&forms, &chain, &floats(&case["y"]), MARGIN).unwrap();
        assert_eq!(
            g.verdict.name(),
            case["verdict"].as_str().unwrap(),
            "{name}"
        );
        close(g.aicc_first, case["aicc_first"].as_f64().unwrap(), name);
        close(g.aicc_second, case["aicc_second"].as_f64().unwrap(), name);
        // the chain is symmetric in its rates: the same set, in any order
        let (mut a, mut b) = (g.rates_second.to_vec(), floats(&case["rates_second"]));
        a.sort_by(f64::total_cmp);
        b.sort_by(f64::total_cmp);
        for (x, y) in a.iter().zip(&b) {
            close(*x, *y, name);
        }
    }
}

/// Everything the module answers on the golden profiles, as bytes.
fn dump() -> Vec<u8> {
    let mut out = Vec::new();
    let cases = golden("cinetica_generazione.json");
    let t = floats(&cases[0]["times"]);
    let (forms, chain) = (FormBank::new(&t), ChainBank::new(&t, K_PARENT_MIN));
    for case in cases {
        let y = floats(&case["y"]);
        for f in forms.fit(&y) {
            for v in [f.rates[0], f.rates[1], f.scale, f.sse, f.aicc] {
                out.extend(v.to_le_bytes());
            }
        }
        let g = generation(&forms, &chain, &y, MARGIN).unwrap();
        out.extend(
            [g.delta, g.aicc_first, g.aicc_second]
                .iter()
                .flat_map(|v| v.to_le_bytes()),
        );
        let s = forms
            .stability(&y, Resample::Bootstrap { draws: 50, seed: 7 })
            .unwrap();
        out.extend(s.counts.iter().flat_map(|c| c.to_le_bytes()));
    }
    out
}

#[test]
fn two_runs_same_bytes() {
    let (a, b) = (Sha256::digest(dump()), Sha256::digest(dump()));
    assert_eq!(a, b);
}
