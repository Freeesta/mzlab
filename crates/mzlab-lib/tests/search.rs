//! The searches over a library: polarity, windows, and the Flash hybrid score against the brute force when pairs compete.
mod common;
use common::*;
use mzlab_lib::build::{build_in_memory, BuildOpts};
use mzlab_lib::read::{Library, CACHE_BYTES};
use mzlab_lib::scores::{entropy_from_pairs, hybrid_pairs, Spec};
use mzlab_lib::search::{
    analogs, flash_scores, identity, AnalogOpts, FlashMode, IdentityOpts, Query, Tol,
};
use mzlab_lib::spec::{Meta, Spectrum};

fn sp(prec: f64, pol: i8, mz: &[f64], it: &[f64], name: &str) -> Spectrum {
    Spectrum {
        prec,
        pol,
        mz: mz.to_vec(),
        it: it.to_vec(),
        meta: Meta {
            name: name.into(),
            ..Default::default()
        },
        rt: None,
    }
}

#[test]
fn identity_respects_polarity_and_the_precursor_window() {
    let mz = [60.0, 80.5, 120.25, 150.0];
    let it = [10.0, 30.0, 100.0, 50.0];
    let set = vec![
        sp(300.0, 1, &mz, &it, "pos"),
        sp(300.0, -1, &mz, &it, "neg"),
        sp(300.0, 0, &mz, &it, "unknown"),
        sp(300.004, 1, &mz, &it, "pos-near"),
        sp(300.2, 1, &mz, &it, "pos-far"),
    ];
    let (b, _) = build_in_memory(&set, BuildOpts::default()).unwrap();
    let lib = Library::open(&b[..], CACHE_BYTES).unwrap();
    let names = |pol: i8, tol: Tol| -> Vec<String> {
        let q = Query::new(300.0, pol, &mz, &it).unwrap();
        let mut v: Vec<String> = identity(
            &lib,
            &q,
            &IdentityOpts {
                prec_tol: tol,
                frag_da: 0.01,
                pol,
                top: 50,
            },
        )
        .unwrap()
        .iter()
        .map(|h| lib.meta(h.index).unwrap().name)
        .collect();
        v.sort();
        v
    };
    assert_eq!(
        names(1, Tol::Ppm(20.0)),
        ["pos", "pos-near", "unknown"],
        "a spectrum without polarity is kept; the negative one is not"
    );
    assert_eq!(names(-1, Tol::Ppm(20.0)), ["neg", "unknown"]);
    assert_eq!(
        names(0, Tol::Ppm(20.0)),
        ["neg", "pos", "pos-near", "unknown"]
    );
    assert_eq!(
        names(1, Tol::Da(0.5)),
        ["pos", "pos-far", "pos-near", "unknown"]
    );
    let q = Query::new(300.0, 1, &mz, &it).unwrap();
    let hits = identity(
        &lib,
        &q,
        &IdentityOpts {
            prec_tol: Tol::Ppm(20.0),
            frag_da: 0.01,
            pol: 1,
            top: 50,
        },
    )
    .unwrap();
    assert!(
        (hits[0].entropy - 1.0).abs() < 1e-3
            && hits[0].shared == 4
            && hits[0].nl_shared == 4
            && (hits[0].explained - 1.0).abs() < 1e-9
    );
}

#[test]
fn a_query_that_cleans_to_nothing_is_none() {
    assert!(Query::new(100.0, 1, &[99.0, 99.9], &[1.0, 1.0]).is_none());
    assert!(Query::new(100.0, 1, &[], &[]).is_none());
}

#[test]
fn the_flash_hybrid_agrees_with_the_brute_force_when_pairs_compete() {
    // peaks one Dalton apart and a precursor difference of 1 Da: every query peak can pair unshifted or shifted
    let mut r = Rng(17);
    let mk = |r: &mut Rng, prec: f64| -> Spectrum {
        let mz: Vec<f64> = (0..14)
            .map(|k| 100.0 + k as f64 * 1.0 + if k % 3 == 0 { 0.001 } else { 0.0 })
            .collect();
        let it: Vec<f64> = (0..14).map(|_| 5.0 + r.unit() * 1000.0).collect();
        sp(prec, 1, &mz, &it, "x")
    };
    let lib: Vec<Spectrum> = (0..40)
        .map(|i| mk(&mut r, 300.0 + (i % 5) as f64 * 0.5))
        .collect();
    let (bytes, _) = build_in_memory(&lib, BuildOpts::default()).unwrap();
    let l = Library::open(&bytes[..], CACHE_BYTES).unwrap();
    let all = l.ranges(1, 0.0, f64::INFINITY).unwrap();
    for t in 0..10 {
        let qs = mk(&mut r, 301.0);
        let q = Query::new(qs.prec, 1, &qs.mz, &qs.it).unwrap();
        let got = flash_scores(&l, &q, FlashMode::Hybrid, 0.01, &all).unwrap();
        for s in got.iter().take(20) {
            let rec = l.rec(s.index).unwrap();
            let (m, i) = l.peaks(s.index).unwrap();
            let lib_spec = Spec {
                mz: m,
                it: i,
                prec: rec.prec,
            };
            let hp = hybrid_pairs(&q.spec, &lib_spec, 0.01);
            let want = entropy_from_pairs(&q.spec.it, &lib_spec.it, &hp);
            assert!(
                (s.score - want).abs() < 1e-3,
                "query {t}, spectrum {}: {} vs {want}",
                s.index,
                s.score
            );
        }
    }
}

#[test]
fn analogues_need_a_distance_in_precursor_and_enough_peaks() {
    let mz = [60.5, 100.1, 150.2, 200.3, 250.4];
    let it = [100.0, 400.0, 900.0, 300.0, 200.0];
    let shifted: Vec<f64> = mz
        .iter()
        .map(|m| if *m > 120.0 { m + 16.0 } else { *m })
        .collect();
    let set = vec![
        sp(300.1, 1, &mz, &it, "analogue"),
        sp(316.1, 1, &shifted, &it, "identity"),
        sp(
            310.0,
            1,
            &[55.0, 95.0, 120.0, 180.0, 222.0],
            &[100.0, 300.0, 800.0, 300.0, 100.0],
            "unrelated",
        ),
        sp(900.0, 1, &mz, &it, "far"),
    ];
    let (b, _) = build_in_memory(&set, BuildOpts::default()).unwrap();
    let lib = Library::open(&b[..], CACHE_BYTES).unwrap();
    let q = Query::new(316.1, 1, &shifted, &it).unwrap();
    let hits = analogs(
        &lib,
        &q,
        &AnalogOpts {
            pol: 1,
            ..AnalogOpts::default()
        },
    )
    .unwrap();
    let names: Vec<String> = hits
        .iter()
        .map(|h| lib.meta(h.index).unwrap().name)
        .collect();
    assert_eq!(
        names,
        ["analogue"],
        "the same precursor is an identity and the unrelated one has no pair"
    );
    assert!(
        hits[0].mc.cosine > 0.99
            && hits[0].mc.shifted == 3
            && (hits[0].mc.delta - 16.0).abs() < 1e-6
            && hits[0].mc.matched == 5
    );
    assert!(hits[0].hybrid > 0.9);
}

/// A window with many candidates: the bounded top-N equals the head of the full ranking, ties included (index order).
#[test]
fn identity_top_n_is_the_head_of_the_full_ranking() {
    let q_mz = [50.0, 70.2, 90.4, 110.6, 130.8];
    let q_it = [10.0, 40.0, 100.0, 25.0, 60.0];
    let set: Vec<Spectrum> = (0..300)
        .map(|k| {
            // a few distinct shapes so that scores repeat across entries (ties)
            let shift = (k % 7) as f64 * 0.3;
            let it: Vec<f64> = q_it.iter().map(|v| v + (k % 5) as f64 * 3.0).collect();
            let mz: Vec<f64> = q_mz.iter().map(|m| m + shift).collect();
            sp(200.0 + (k % 3) as f64 * 0.1, 1, &mz, &it, &format!("e{k}"))
        })
        .collect();
    let (b, _) = build_in_memory(&set, BuildOpts::default()).unwrap();
    let lib = Library::open(&b[..], CACHE_BYTES).unwrap();
    let q = Query::new(200.1, 1, &q_mz, &q_it).unwrap();
    let run = |top: usize| {
        identity(
            &lib,
            &q,
            &IdentityOpts {
                prec_tol: Tol::Da(0.5),
                frag_da: 0.25,
                pol: 1,
                top,
            },
        )
        .unwrap()
    };
    let full = run(1000);
    assert_eq!(full.len(), 300);
    // an unbounded `top` must not overflow the pruning threshold
    assert_eq!(run(usize::MAX).len(), 300);
    for top in [1, 5, 17, 100] {
        let got = run(top);
        assert_eq!(got.len(), top);
        for (g, f) in got.iter().zip(&full) {
            assert_eq!(
                (g.index, g.entropy, g.shared),
                (f.index, f.entropy, f.shared)
            );
            assert_eq!(g.pairs, f.pairs);
            assert_eq!(g.spec.mz, f.spec.mz);
        }
    }
    assert!(full.windows(2).all(|w| w[0].entropy >= w[1].entropy));
}
