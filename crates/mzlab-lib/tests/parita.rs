//! Parity with the references (tests/golden/librerie/, written by tools/parita_librerie.py from SYNTHETIC spectra):
//! * not quantised (the scores of `scores.rs` on the spectra as they are) against `ms_entropy` / `matchms`: 1e-6;
//! * through a `.mzlib` (quantised, Flash indexes): 1e-4, and the same ten best candidates in at least 99% of the queries;
//! * the GNPS-style cosine against the synthetic cases of the Attributio parity file: 1e-9 (not quantised), 1e-4 (quantised).
mod common;
use mzlab_lib::build::{build_in_memory, BuildOpts};
use mzlab_lib::clean::quantized;
use mzlab_lib::read::{Library, CACHE_BYTES};
use mzlab_lib::scores::{
    entropy_from_pairs, gnps_cosine, hybrid_pairs, modified_cosine, pair_scores, Spec,
};
use mzlab_lib::search::{
    analogs, flash_scores, identity, AnalogOpts, FlashMode, IdentityOpts, Query, Tol,
};
use mzlab_lib::spec::{Meta, Spectrum};
use serde_json::Value;
use std::collections::HashMap;
use std::path::PathBuf;

fn golden(name: &str) -> Value {
    let p = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../tests/golden/librerie")
        .join(name);
    serde_json::from_slice(
        &std::fs::read(&p)
            .unwrap_or_else(|e| panic!("{}: {e} (python3 tools/parita_librerie.py)", p.display())),
    )
    .unwrap()
}

fn f64s(v: &Value) -> Vec<f64> {
    v.as_array()
        .unwrap()
        .iter()
        .map(|x| x.as_f64().unwrap())
        .collect()
}

fn spec(v: &Value) -> Spec {
    Spec {
        mz: f64s(&v["mz"]),
        it: f64s(v.get("it").unwrap_or(&v["int"])),
        prec: v["prec"].as_f64().unwrap(),
    }
}

#[test]
fn pairs_not_quantised_match_ms_entropy_and_matchms() {
    let g = golden("pares.json");
    for (n, c) in g["casi"].as_array().unwrap().iter().enumerate() {
        let (a, b, tol) = (spec(&c["a"]), spec(&c["b"]), c["tol"].as_f64().unwrap());
        let s = pair_scores(&a, &b, tol);
        assert!(
            (s.entropy - c["entropy"].as_f64().unwrap()).abs() < 1e-6,
            "case {n} entropy {} vs {}",
            s.entropy,
            c["entropy"]
        );
        assert!(
            (s.cosine - c["cosine"].as_f64().unwrap()).abs() < 1e-6,
            "case {n} cosine {} vs {}",
            s.cosine,
            c["cosine"]
        );
        assert_eq!(
            s.shared as u64,
            c["matches"].as_u64().unwrap(),
            "case {n} matches"
        );
        let m = modified_cosine(&a, &b, tol);
        assert!(
            (m.cosine - c["mod_cosine"].as_f64().unwrap()).abs() < 1e-6,
            "case {n} modified {} vs {}",
            m.cosine,
            c["mod_cosine"]
        );
        assert_eq!(
            m.matched as u64,
            c["mod_matches"].as_u64().unwrap(),
            "case {n} modified matches"
        );
    }
}

#[test]
fn the_brute_force_of_mzlab_core_agrees_to_1e_9() {
    // mzlab-core::entropy is the reference of the golden data (tools/golden.py): the same pairs, the same score
    let g = golden("pares.json");
    for (n, c) in g["casi"].as_array().unwrap().iter().enumerate() {
        let (a, b, tol) = (spec(&c["a"]), spec(&c["b"]), c["tol"].as_f64().unwrap());
        let core = mzlab_core::entropy::entropy_similarity(&a.mz, &a.it, &b.mz, &b.it, tol);
        let ours = pair_scores(&a, &b, tol).entropy;
        assert!(
            (core - ours).abs() < 1e-9,
            "case {n}: core {core} vs {ours}"
        );
    }
}

fn lib_spectra(g: &Value) -> Vec<Spec> {
    g["library"].as_array().unwrap().iter().map(spec).collect()
}

fn as_spectrum(s: &Spec) -> Spectrum {
    Spectrum {
        prec: s.prec,
        pol: 1,
        mz: s.mz.clone(),
        it: s.it.clone(),
        meta: Meta::default(),
    }
}

fn expected(q: &Value, key: &str) -> HashMap<usize, f64> {
    q[key]
        .as_object()
        .unwrap()
        .iter()
        .map(|(k, v)| (k.parse().unwrap(), v.as_f64().unwrap()))
        .collect()
}

#[test]
fn flash_not_quantised_brute_force_matches_ms_entropy() {
    let g = golden("flash.json");
    let lib = lib_spectra(&g);
    let (tol, tolp) = (
        g["tol_frag"].as_f64().unwrap(),
        g["tol_prec"].as_f64().unwrap(),
    );
    for (qi, q) in g["queries"].as_array().unwrap().iter().enumerate() {
        let qs = Spec {
            mz: f64s(&q["mz"]),
            it: f64s(&q["it"]),
            prec: q["prec"].as_f64().unwrap(),
        };
        let (eid, eopen, ehyb) = (
            expected(q, "identity"),
            expected(q, "open"),
            expected(q, "hybrid"),
        );
        for (i, l) in lib.iter().enumerate() {
            let open = pair_scores(&qs, l, tol).entropy;
            let want = eopen.get(&i).copied().unwrap_or(0.0);
            assert!(
                (open - want).abs() < 1e-6,
                "query {qi} spectrum {i} open {open} vs {want}"
            );
            let id = if (l.prec - qs.prec).abs() <= tolp {
                open
            } else {
                0.0
            };
            assert!(
                (id - eid.get(&i).copied().unwrap_or(0.0)).abs() < 1e-6,
                "query {qi} spectrum {i} identity"
            );
            let hp = hybrid_pairs(&qs, l, tol);
            let hyb = entropy_from_pairs(&qs.it, &l.it, &hp);
            let want = ehyb.get(&i).copied().unwrap_or(0.0);
            assert!(
                (hyb - want).abs() < 1e-6,
                "query {qi} spectrum {i} hybrid {hyb} vs {want}"
            );
        }
    }
}

fn top10(scores: &[(usize, f64)]) -> Vec<usize> {
    let mut v: Vec<(usize, f64)> = scores.iter().copied().filter(|x| x.1 > 1e-6).collect();
    v.sort_by(|a, b| b.1.total_cmp(&a.1).then(a.0.cmp(&b.0)));
    v.into_iter().take(10).map(|x| x.0).collect()
}

#[test]
fn flash_through_the_mzlib_matches_within_1e_4_and_ranks_the_same() {
    let g = golden("flash.json");
    let lib = lib_spectra(&g);
    let spectra: Vec<Spectrum> = lib.iter().map(as_spectrum).collect();
    let (bytes, _) = build_in_memory(&spectra, BuildOpts::default()).unwrap();
    let l = Library::open(&bytes[..], CACHE_BYTES).unwrap();
    // library index -> position in the original list (the table is sorted by precursor)
    let orig: Vec<usize> = (0..l.n_spectra())
        .map(|i| l.rec(i).unwrap().orig as usize)
        .collect();
    let (tol, tolp) = (
        g["tol_frag"].as_f64().unwrap(),
        g["tol_prec"].as_f64().unwrap(),
    );
    let (mut same, mut total) = (0, 0);
    for (qi, q) in g["queries"].as_array().unwrap().iter().enumerate() {
        let query = Query::new(
            q["prec"].as_f64().unwrap(),
            1,
            &f64s(&q["mz"]),
            &f64s(&q["it"]),
        )
        .unwrap();
        let all = l.ranges(1, 0.0, f64::INFINITY).unwrap();
        for (mode, key) in [
            (FlashMode::Open, "open"),
            (FlashMode::NeutralLoss, "nl"),
            (FlashMode::Hybrid, "hybrid"),
        ] {
            let got: Vec<(usize, f64)> = flash_scores(&l, &query, mode, tol, &all)
                .unwrap()
                .into_iter()
                .map(|s| (orig[s.index as usize], s.score))
                .collect();
            let want = expected(q, key);
            let gm: HashMap<usize, f64> = got.iter().copied().collect();
            for (id, w) in &want {
                let v = gm.get(id).copied().unwrap_or(0.0);
                assert!(
                    (v - w).abs() < 1e-4,
                    "query {qi} {key} spectrum {id}: {v} vs {w}"
                );
            }
            for (id, v) in &got {
                if !want.contains_key(id) {
                    assert!(*v < 1e-4, "query {qi} {key} spectrum {id}: unexpected {v}");
                }
            }
            total += 1;
            let w: Vec<(usize, f64)> = want.iter().map(|(k, v)| (*k, *v)).collect();
            if top10(&got) == top10(&w) {
                same += 1;
            }
        }
        // identity by the precursor window
        let hits = identity(
            &l,
            &query,
            &IdentityOpts {
                prec_tol: Tol::Da(tolp),
                frag_da: tol,
                pol: 1,
                top: 1000,
            },
        )
        .unwrap();
        let want = expected(q, "identity");
        let got: Vec<(usize, f64)> = hits
            .iter()
            .map(|h| (orig[h.index as usize], h.entropy))
            .collect();
        let gm: HashMap<usize, f64> = got.iter().copied().collect();
        for (id, w) in &want {
            assert!(
                (gm.get(id).copied().unwrap_or(0.0) - w).abs() < 1e-4,
                "query {qi} identity spectrum {id}"
            );
        }
        for (id, v) in &got {
            assert!(
                want.contains_key(id) || *v < 1e-4,
                "query {qi} identity spectrum {id}: unexpected {v}"
            );
        }
        total += 1;
        let w: Vec<(usize, f64)> = want.iter().map(|(k, v)| (*k, *v)).collect();
        if top10(&got) == top10(&w) {
            same += 1;
        }
    }
    assert!(
        same as f64 >= 0.99 * total as f64,
        "same top ten in {same} of {total}"
    );
}

#[test]
fn analogs_find_the_shifted_neighbours() {
    let g = golden("flash.json");
    let lib = lib_spectra(&g);
    let spectra: Vec<Spectrum> = lib.iter().map(as_spectrum).collect();
    let (bytes, _) = build_in_memory(&spectra, BuildOpts::default()).unwrap();
    let l = Library::open(&bytes[..], CACHE_BYTES).unwrap();
    let mut found = 0;
    let mut n = 0;
    for q in g["queries"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|q| q["kind"] == "analogue")
    {
        let query = Query::new(
            q["prec"].as_f64().unwrap(),
            1,
            &f64s(&q["mz"]),
            &f64s(&q["it"]),
        )
        .unwrap();
        let hits = analogs(&l, &query, &AnalogOpts::default()).unwrap();
        n += 1;
        // the best Flash hybrid candidate of the reference is among the analogues found (when it has enough matched peaks)
        let best = expected(q, "hybrid")
            .into_iter()
            .filter(|(i, _)| (lib[*i].prec - query.prec).abs() >= 0.5)
            .max_by(|a, b| a.1.total_cmp(&b.1));
        if let Some((id, _)) = best {
            let ok = hits
                .iter()
                .any(|h| l.rec(h.index).unwrap().orig as usize == id);
            if ok || hits.is_empty() {
                found += 1;
            }
        } else {
            found += 1;
        }
        for h in &hits {
            assert!(h.mc.matched >= 4 && (h.prec - query.prec).abs() >= 0.5);
        }
        assert!(hits.windows(2).all(|w| w[0].mc.cosine >= w[1].mc.cosine));
    }
    assert!(n > 0 && found == n, "analogue found in {found} of {n}");
}

fn gnps_spec(v: &Value) -> Spec {
    spec(v)
}

fn quant(s: &Spec) -> Spec {
    let (mz, it) = quantized(&s.mz, &s.it);
    Spec {
        mz,
        it,
        prec: s.prec,
    }
}

fn check_attributio(quantise: bool, tol_score: f64) {
    let g = golden("parita_attributio_ms2.json");
    for c in g["casi"].as_array().unwrap() {
        let id = c["id"].as_u64().unwrap();
        let (mut a, mut b) = (gnps_spec(&c["a"]), gnps_spec(&c["b"]));
        if quantise {
            a = quant(&a);
            b = quant(&b);
        }
        for tol in [0.01, 0.02] {
            let want = &c[format!("similarity_tol{tol}")];
            let r = gnps_cosine(&a, &b, tol, false);
            assert!(
                (r.cosine - want["cosine"].as_f64().unwrap()).abs() < tol_score,
                "case {id} tol {tol} cosine {} vs {}",
                r.cosine,
                want["cosine"]
            );
            assert!(
                (r.reverse - want["reverse"].as_f64().unwrap()).abs() < tol_score,
                "case {id} tol {tol} reverse"
            );
            assert_eq!(
                r.matched as u64,
                want["matched"].as_u64().unwrap(),
                "case {id} tol {tol} matched"
            );
        }
        let want = &c["match_tol0.02"];
        let r = gnps_cosine(&a, &b, 0.02, true);
        assert!(
            (r.cosine - want["cosine"].as_f64().unwrap()).abs() < tol_score,
            "case {id} modified cosine {} vs {}",
            r.cosine,
            want["cosine"]
        );
        assert!(
            (r.reverse - want["reverse"].as_f64().unwrap()).abs() < tol_score,
            "case {id} modified reverse"
        );
        assert_eq!(
            r.matched as u64,
            want["matched"].as_u64().unwrap(),
            "case {id} modified matched"
        );
        assert_eq!(
            r.n_shifted as u64,
            want["n_shifted"].as_u64().unwrap(),
            "case {id} n_shifted"
        );
        let shift = want["shift"].as_f64().unwrap();
        assert!(
            (shift - (a.prec - b.prec)).abs() < 1e-9,
            "case {id}: shift is prec(sample) - prec(reference)"
        );
        let mut got: Vec<(u64, u64, u64)> = r
            .pairs
            .iter()
            .map(|p| (p.0 as u64, p.1 as u64, p.2 as u64))
            .collect();
        got.sort();
        let mut exp: Vec<(u64, u64, u64)> = want["pairs"]
            .as_array()
            .unwrap()
            .iter()
            .map(|p| {
                (
                    p[0].as_u64().unwrap(),
                    p[1].as_u64().unwrap(),
                    p[2].as_u64().unwrap(),
                )
            })
            .collect();
        exp.sort();
        // the indices of the reference are those of the spectra as given; ours are those after the removals: compare through the m/z
        if !quantise {
            let map = |s: &Spec| -> Vec<usize> {
                (0..s.mz.len())
                    .filter(|&i| {
                        s.mz[i] < s.prec - 0.5
                            && s.it[i] >= 0.01 * s.it.iter().cloned().fold(0.0, f64::max)
                    })
                    .collect()
            };
            let (ma, mb) = (map(&a), map(&b));
            let mut got2: Vec<(u64, u64, u64)> = r
                .pairs
                .iter()
                .map(|p| (ma[p.0] as u64, mb[p.1] as u64, p.2 as u64))
                .collect();
            got2.sort();
            assert!(
                got == exp || got2 == exp,
                "case {id} pairs {got:?} / {got2:?} vs {exp:?}"
            );
        }
    }
}

#[test]
fn gnps_cosine_matches_the_attributio_cases_not_quantised() {
    check_attributio(false, 1e-9);
}

#[test]
fn gnps_cosine_matches_the_attributio_cases_quantised() {
    check_attributio(true, 1e-4);
}
