//! The parts of mzFinder ported to Rust (`legacy/`) against the frozen answers of the former Python engine: `formula_info`, the
//! names of `classify`, the kinetic descriptors, the MS2 consensus, the three components of the MS2 similarity and the MSn tree with its
//! library and localisation, numbers within 1e-9.

use mzlab_feat::legacy::edge::{origin_evidence, Iimn};
use mzlab_feat::legacy::fragtree::{elements_of, two_level, vec_of, TwoLevel};
use mzlab_feat::legacy::{
    formula_info, identity_graph, kinetics, ms2, msn, polarity_pairs, sample, similarity,
};
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

#[test]
fn identity_graph_matches_python() {
    for (i, c) in golden()["identity"].as_array().unwrap().iter().enumerate() {
        let got: Value =
            serde_json::from_str(&identity_graph(&c["payload"].to_string()).unwrap()).unwrap();
        same(&got, &c["out"], &format!("identity case {i}"));
    }
}

#[test]
fn origin_evidence_matches_python() {
    for (i, c) in golden()["origin"].as_array().unwrap().iter().enumerate() {
        let got: Value =
            serde_json::from_str(&origin_evidence(&c["payload"].to_string()).unwrap()).unwrap();
        same(&got, &c["out"], &format!("origin case {i}"));
    }
}

#[test]
fn polarity_pairs_match_python() {
    for (i, c) in golden()["polarity"].as_array().unwrap().iter().enumerate() {
        let got: Value =
            serde_json::from_str(&polarity_pairs(&c["payload"].to_string()).unwrap()).unwrap();
        same(&got, &c["out"], &format!("polarity case {i}"));
    }
}

#[test]
fn frag_tree_matches_python() {
    let losses = Iimn::new().losses;
    for (i, c) in golden()["tree"].as_array().unwrap().iter().enumerate() {
        let ion = c["ion"].as_str();
        let els = ion.map(|f| elements_of(f).unwrap()).unwrap_or_default();
        let vec = ion.map(|f| vec_of(f, &els).unwrap());
        let (mzs, rel) = (floats(&c["mzs"]), floats(&c["rel"]));
        let t = TwoLevel {
            ion: vec.as_deref(),
            els: &els,
            mz0: c["mz0"].as_f64().unwrap(),
            mzs: &mzs,
            rel: &rel,
            smiles: c["smiles"].as_str(),
            ppm: c["ppm"].as_f64().unwrap(),
            abs_da: c["abs_da"].as_f64().unwrap(),
            supposed: c["supposed"].as_bool().unwrap(),
            mode: c["mode"].as_str().unwrap(),
            n_scans: c["n_scans"].as_i64(),
        };
        let got: Value = serde_json::from_str(&two_level(&t, &losses)).unwrap();
        same(&got, &c["out"], &format!("tree {i}"));
    }
}

fn scans_of(v: &Value) -> Vec<msn::MsnScan> {
    v.as_array()
        .unwrap()
        .iter()
        .map(|s| msn::MsnScan {
            level: s["level"].as_u64().unwrap() as u8,
            path: s["path"].as_array().map(|p| {
                p.iter()
                    .map(|x| {
                        (
                            x[0].as_f64().unwrap(),
                            x[1].as_str().unwrap().to_string(),
                            x[2].as_f64().unwrap(),
                        )
                    })
                    .collect()
            }),
            mz: floats(&s["mz"]),
            it: floats(&s["it"]),
        })
        .collect()
}

fn parsed(s: &str) -> Value {
    serde_json::from_str(s).unwrap()
}

/// `top_sites` in groups of equal log-likelihood (within 1e-9): the order inside a group comes from the order of the sums (numpy's BLAS
/// against a plain loop: values equal on paper differ in the last bit), so a group is compared as a set; the last one, cut at six, by size.
fn same_sites(a: &Value, b: &Value, what: &str) {
    let groups = |v: &Value| {
        let mut g: Vec<(f64, Vec<String>)> = Vec::new();
        for s in v.as_array().unwrap() {
            let ll = s["ll"].as_f64().unwrap();
            let name = format!("{}{}", s["site"], s["atoms"]);
            match g.last_mut() {
                Some(l) if (l.0 - ll).abs() <= 1e-9 * ll.abs().max(1.0) => l.1.push(name),
                _ => g.push((ll, vec![name])),
            }
        }
        g.iter_mut().for_each(|x| x.1.sort());
        g
    };
    let (ga, gb) = (groups(a), groups(b));
    assert_eq!(ga.len(), gb.len(), "{what}: groups");
    for (k, (x, y)) in ga.iter().zip(&gb).enumerate() {
        close(x.0, y.0, what);
        if k + 1 < ga.len() {
            assert_eq!(x.1, y.1, "{what}: group {k}");
        } else {
            assert_eq!(x.1.len(), y.1.len(), "{what}: last group");
        }
    }
}

/// The localisation: `top_sites` by groups, the rest as JSON.
fn same_localization(mut a: Value, b: &Value, what: &str) {
    let mut b = b.clone();
    if let (Some(x), Some(y)) = (a.get("top_sites").cloned(), b.get("top_sites").cloned()) {
        same_sites(&x, &y, what);
        a.as_object_mut().unwrap().remove("top_sites");
        b.as_object_mut().unwrap().remove("top_sites");
    }
    same(&a, &b, what);
}

#[test]
fn msn_tree_library_and_localization_match_python() {
    for (i, c) in golden()["msn"].as_array().unwrap().iter().enumerate() {
        let what = format!("msn case {i}");
        let smiles = c["smiles"].as_str();
        let t = msn::build_tree(
            &scans_of(&c["scans"]),
            c["ion"].as_str().unwrap(),
            smiles,
            c["calibrate"].as_str().unwrap(),
        )
        .unwrap();
        same(
            &parsed(&msn::json::summary(&t)),
            &c["summary"],
            &format!("{what} summary"),
        );
        same(
            &parsed(&msn::json::calibration(&t.cal)),
            &c["calibration"],
            &format!("{what} calibration"),
        );
        same(
            &parsed(&msn::json::tree_summary(&t)),
            &c["tree_summary"],
            &format!("{what} msn_tree"),
        );
        same(
            &parsed(&msn::json::frag_tree(&t, smiles)),
            &c["frag_tree"],
            &format!("{what} frag_tree"),
        );
        let Some(subs) = t.subs.as_ref() else {
            assert!(
                c.get("library").is_none(),
                "{what}: library without a structure"
            );
            continue;
        };
        let dda = c["dda"].as_array().map(|d| (floats(&d[0]), floats(&d[1])));
        let lib = msn::library::build(&t, dda.as_ref().map(|d| (&d.0[..], &d.1[..])));
        same(
            &parsed(&msn::json::library(&lib)),
            &c["library"],
            &format!("{what} library"),
        );
        let top = vec_of(t.nodes[0].formula.as_deref().unwrap(), &t.els).unwrap();
        for (j, l) in c["localize"].as_array().unwrap().iter().enumerate() {
            let tp = vec_of(l["tp"].as_str().unwrap(), &t.els).unwrap();
            let loc = msn::localize::localize(
                &tp,
                &top,
                &t.els,
                &floats(&l["mz"]),
                &floats(&l["rel"]),
                &lib,
                subs,
                l["tp_mz"].as_f64(),
            );
            same_localization(
                parsed(&msn::json::localization(&loc)),
                &l["out"],
                &format!("{what} localize {j}"),
            );
            same(
                &parsed(&msn::json::compare(&t, Some(&loc))),
                &l["compare"],
                &format!("{what} compare {j}"),
            );
        }
    }
}

/// `localize_sites` scores the sites it is given with the same likelihood `localize` gives its best site, and English group names exist.
#[test]
fn localize_sites_scores_given_masks_like_localize() {
    let mut checked = 0;
    for c in golden()["msn"].as_array().unwrap() {
        let t = msn::build_tree(
            &scans_of(&c["scans"]),
            c["ion"].as_str().unwrap(),
            c["smiles"].as_str(),
            c["calibrate"].as_str().unwrap(),
        )
        .unwrap();
        let Some(subs) = t.subs.as_ref() else {
            continue;
        };
        let dda = c["dda"].as_array().map(|d| (floats(&d[0]), floats(&d[1])));
        let lib = msn::library::build(&t, dda.as_ref().map(|d| (&d.0[..], &d.1[..])));
        let top = vec_of(t.nodes[0].formula.as_deref().unwrap(), &t.els).unwrap();
        for l in c["localize"].as_array().unwrap() {
            let tp = vec_of(l["tp"].as_str().unwrap(), &t.els).unwrap();
            let (mz, rel) = (floats(&l["mz"]), floats(&l["rel"]));
            let loc = msn::localize::localize(
                &tp,
                &top,
                &t.els,
                &mz,
                &rel,
                &lib,
                subs,
                l["tp_mz"].as_f64(),
            );
            let Some(found) = loc.found else { continue };
            let (_, ll, atoms) = &found.top_sites[0];
            let mask = atoms.iter().fold(0u64, |m, a| m | 1 << a);
            let again = msn::localize::localize_sites(
                &tp,
                &top,
                &t.els,
                &mz,
                &rel,
                &lib,
                subs,
                l["tp_mz"].as_f64(),
                &[mask],
            );
            let f = again.found.expect("the given site has evidence");
            assert_eq!(f.all_ll.len(), 1);
            assert!((f.all_ll[0] - ll).abs() < 1e-9, "{} vs {ll}", f.all_ll[0]);
            checked += 1;
        }
    }
    assert!(checked > 0, "no case with evidence");
    let g = mzlab_feat::legacy::fragtree::Graph::parse("CCc1ccccc1").unwrap();
    let names: Vec<String> = msn::localize::default_groups_en(&g)
        .into_iter()
        .map(|x| x.0)
        .collect();
    assert_eq!(names, ["ring A", "chain A"]);
}
