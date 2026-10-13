//! The parts of mzFinder ported to Rust (`legacy/`) against the frozen Python answers (`tools/finder_riferimento.py --golden`): `formula_info`, the
//! names of `classify`, the kinetic descriptors, the MS2 consensus and the three components of the MS2 similarity, numbers within 1e-9.

use mzlab_feat::legacy::{formula_info, kinetics, ms2, sample, similarity};
use serde_json::Value;

fn golden() -> Value {
    let p = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../tests/golden/feat/finder.json");
    serde_json::from_slice(&std::fs::read(p).unwrap()).unwrap()
}

fn floats(v: &Value) -> Vec<f64> {
    v.as_array()
        .unwrap()
        .iter()
        .map(|x| x.as_f64().unwrap_or(f64::NAN))
        .collect()
}

fn close(a: f64, b: f64, what: &str) {
    assert!(
        (a.is_nan() && b.is_nan()) || (a - b).abs() <= 1e-9 * b.abs().max(1.0),
        "{what}: {a} vs {b}"
    );
}

fn all_close(a: &[f64], b: &[f64], what: &str) {
    assert_eq!(a.len(), b.len(), "{what}: lengths");
    a.iter().zip(b).for_each(|(x, y)| close(*x, *y, what));
}

/// Same JSON, numbers within 1e-9.
fn same(a: &Value, b: &Value, what: &str) {
    match (a, b) {
        (Value::Number(x), Value::Number(y)) => {
            close(x.as_f64().unwrap(), y.as_f64().unwrap(), what)
        }
        (Value::Object(x), Value::Object(y)) => {
            let (mut kx, mut ky): (Vec<_>, Vec<_>) = (x.keys().collect(), y.keys().collect());
            kx.sort();
            ky.sort();
            assert_eq!(kx, ky, "{what}: keys");
            x.iter()
                .for_each(|(k, v)| same(v, &y[k], &format!("{what}.{k}")));
        }
        (Value::Array(x), Value::Array(y)) => {
            assert_eq!(x.len(), y.len(), "{what}: lengths");
            x.iter().zip(y).for_each(|(p, q)| same(p, q, what));
        }
        _ => assert_eq!(a, b, "{what}"),
    }
}

#[test]
fn formula_info_matches_python() {
    for c in golden()["formula_info"].as_array().unwrap() {
        let (t, a) = (c["text"].as_str().unwrap(), c["adduct"].as_str().unwrap());
        let got: Value = serde_json::from_str(&formula_info(t, a)).unwrap();
        same(&got, &c["out"], &format!("formula_info({t:?}, {a:?})"));
    }
}

#[test]
fn names_match_python() {
    for c in golden()["guess"].as_array().unwrap() {
        let name = c["name"].as_str().unwrap();
        let g = sample::guess(name);
        let o = &c["out"];
        assert_eq!(g.label, o["label"].as_str().unwrap(), "{name}: label");
        assert_eq!(g.kind, o["type"].as_str().unwrap(), "{name}: type");
        assert_eq!(g.time, o["time"].as_f64(), "{name}: time");
    }
}

#[test]
fn kinetics_match_python() {
    for (i, c) in golden()["kinetics"].as_array().unwrap().iter().enumerate() {
        let (t, y) = (floats(&c["times"]), floats(&c["y"]));
        let d = kinetics::descriptors(&t, &y);
        let Some(o) = c["out"].as_object() else {
            assert!(d.is_none(), "case {i}: Python has no descriptors");
            continue;
        };
        let d = d.unwrap_or_else(|| panic!("case {i}: no descriptors"));
        close(d.onset, o["onset"].as_f64().unwrap(), "onset");
        close(d.tmax, o["tmax"].as_f64().unwrap(), "tmax");
        close(
            d.residual_fraction,
            o["residual_fraction"].as_f64().unwrap(),
            "residual",
        );
        close(
            d.unimodal_residual,
            o["unimodal_residual"].as_f64().unwrap(),
            "unimodal residual",
        );
        assert_eq!(
            d.unimodal(),
            o["unimodal"].as_bool().unwrap(),
            "case {i}: unimodal"
        );
        assert_eq!(
            d.class_text(),
            o["class_text"].as_str().unwrap(),
            "case {i}: class"
        );
        all_close(&d.profile, &floats(&o["profile"]), "profile");
        all_close(&d.times, &floats(&o["times"]), "times");
        if let Some(a) = c["absent"].as_bool() {
            let tr: Vec<f64> = t
                .iter()
                .zip(&y)
                .filter(|p| *p.0 > 0.0)
                .map(|p| *p.1)
                .collect();
            assert_eq!(
                kinetics::not_in_reference(&tr, c["ref_max"].as_f64().unwrap()),
                a,
                "case {i}: absent"
            );
        }
    }
}

#[test]
fn consensus_matches_python() {
    for c in golden()["consensus"].as_array().unwrap() {
        let sp: Vec<(Vec<f64>, Vec<f64>)> = c["spectra"]
            .as_array()
            .unwrap()
            .iter()
            .map(|s| (floats(&s[0]), floats(&s[1])))
            .collect();
        let r = ms2::consensus(&sp);
        let o = &c["out"];
        all_close(&r.mz, &floats(&o["mz"]), "mz");
        all_close(&r.rel, &floats(&o["rel"]), "rel");
        let n: Vec<usize> = o["n_found"]
            .as_array()
            .unwrap()
            .iter()
            .map(|x| x.as_u64().unwrap() as usize)
            .collect();
        assert_eq!(r.n_found, n);
        assert_eq!(r.n_scans as u64, o["n_scans"].as_u64().unwrap());
    }
}

#[test]
fn similarity_matches_python() {
    for (i, c) in golden()["similarity"]
        .as_array()
        .unwrap()
        .iter()
        .enumerate()
    {
        let (a, b) = (&c["a"], &c["b"]);
        let (pa, pb) = (a[2].as_f64().unwrap(), b[2].as_f64().unwrap());
        let sa = similarity::prepare(&floats(&a[0]), &floats(&a[1]), pa);
        let sb = similarity::prepare(&floats(&b[0]), &floats(&b[1]), pb);
        all_close(&sa.0, &floats(&c["prep_a"][0]), "prepared m/z");
        all_close(&sa.1, &floats(&c["prep_a"][1]), "prepared weights");
        if !sa.1.is_empty() {
            all_close(
                &similarity::entropy_weights(&sa.1),
                &floats(&c["ent_a"]),
                "entropy weights",
            );
        }
        let mut o = mzlab_core::json::Out::default();
        similarity::pair_components(&sa, pa, &sb, pb).json(&mut o);
        let got: Value = serde_json::from_str(&o.0).unwrap();
        same(&got, &c["out"], &format!("similarity case {i}"));
    }
}
