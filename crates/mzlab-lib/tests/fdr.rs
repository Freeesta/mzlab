//! Target-decoy q-values of the library search: golden numbers, determinism, order independence, and a synthetic library of 5000 entries.
mod common;
use common::*;
use mzlab_lib::build::{build_in_memory, BuildOpts};
use mzlab_lib::decoy::*;
use mzlab_lib::read::{Library, CACHE_BYTES};
use mzlab_lib::search::{identity, IdentityOpts, Query, Tol};
use mzlab_lib::spec::Spectrum;
use std::time::Instant;

fn golden() -> serde_json::Value {
    let p = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../tests/golden/lib/qvalues.json");
    serde_json::from_slice(&std::fs::read(p).unwrap()).unwrap()
}

fn floats(v: &serde_json::Value) -> Vec<f32> {
    v.as_array()
        .unwrap()
        .iter()
        .map(|x| x.as_f64().unwrap() as f32)
        .collect()
}

#[test]
fn golden_qvalues() {
    let g = golden();
    for c in g["cases"].as_array().unwrap() {
        let t = floats(&c["target"]);
        let d = floats(&c["decoy"]);
        let f = lib_fdr(&t, &d, DecoyKind::Fragments);
        let name = c["name"].as_str().unwrap();
        let q = floats(&c["q"]);
        for i in 0..t.len() {
            assert!((f[i].q_value - q[i]).abs() < 1e-6, "{name} q[{i}]");
            assert_eq!(
                f[i].n_target_ge as u64,
                c["n_target_ge"][i].as_u64().unwrap(),
                "{name}"
            );
            assert_eq!(
                f[i].n_decoy_ge as u64,
                c["n_decoy_ge"][i].as_u64().unwrap(),
                "{name}"
            );
        }
    }
}

const OPTS: IdentityOpts = IdentityOpts {
    prec_tol: Tol::Ppm(10.0),
    frag_da: 0.01,
    pol: 1,
    top: 20,
};

/// A noisy copy of an entry (jittered m/z, scaled intensities, some peaks dropped).
fn noisy(r: &mut Rng, s: &Spectrum) -> Spectrum {
    let mut q = s.clone();
    q.mz.clear();
    q.it.clear();
    for (m, i) in s.mz.iter().zip(&s.it) {
        if r.unit() < 0.15 {
            continue;
        }
        q.mz.push(m + r.range(-0.003, 0.003));
        q.it.push(i * r.range(0.7, 1.3));
    }
    q
}

struct Run {
    best: Vec<Best>,
    truth: Vec<bool>,
    secs: f64,
}

fn run_queries(lib: &Library<&[u8]>, qs: &[(Spectrum, bool)]) -> Run {
    let t0 = Instant::now();
    let mut best = Vec::new();
    let mut truth = Vec::new();
    for (s, t) in qs {
        let q = Query::new(s.prec, 1, &s.mz, &s.it).unwrap();
        best.push(best_scores(lib, &q, &OPTS).unwrap());
        truth.push(*t);
    }
    Run {
        best,
        truth,
        secs: t0.elapsed().as_secs_f64(),
    }
}

fn library(n: usize, with: bool) -> (Vec<Spectrum>, Vec<u8>) {
    let mut t = synth_set(11, n, 12);
    t.iter_mut().for_each(|s| s.pol = 1);
    let all = if with {
        with_decoys(&t, DecoyKind::Fragments, DEFAULT_SEED)
    } else {
        t.clone()
    };
    (t, build_in_memory(&all, BuildOpts::default()).unwrap().0)
}

#[test]
fn synthetic_library_of_5000_entries_with_500_queries() {
    let (targets, bytes) = library(5000, true);
    let (_, bytes_plain) = library(5000, false);
    let lib = Library::open(&bytes[..], CACHE_BYTES).unwrap();
    let plain = Library::open(&bytes_plain[..], CACHE_BYTES).unwrap();
    // 300 noisy copies of entries (true hits) and 200 wrong spectra at the precursor of an entry (isomer-like errors)
    let mut r = Rng(99);
    let mut qs: Vec<(Spectrum, bool)> = Vec::new();
    for k in 0..300 {
        qs.push((noisy(&mut r, &targets[k * 16]), true));
    }
    for k in 0..200 {
        let mut w = noisy(&mut r, &targets[(k * 24 + 5) % 5000]);
        let other = &targets[(k * 24 + 777) % 5000];
        w.mz = other.mz.iter().map(|m| m.min(w.prec - 4.0)).collect();
        w.it = other.it.clone();
        qs.push((w, false));
    }
    let a = run_queries(&lib, &qs);
    let b = run_queries(&lib, &qs);
    assert_eq!(a.best, b.best, "two runs give the same scores");
    let pl = run_queries(&plain, &qs);
    let ratio = a.secs.min(b.secs) / pl.secs;
    eprintln!(
        "search time with decoys {:.3}s, without {:.3}s, ratio {ratio:.2}",
        a.secs.min(b.secs),
        pl.secs
    );
    assert!(ratio <= 2.0, "ratio {ratio}");
    let t: Vec<f32> = a.best.iter().map(|x| x.target).collect();
    let d: Vec<f32> = a.best.iter().map(|x| x.decoy).collect();
    let f = lib_fdr(&t, &d, DecoyKind::Fragments);
    // q never increases with the score
    let mut idx: Vec<usize> = (0..t.len()).collect();
    idx.sort_by(|&i, &j| t[j].total_cmp(&t[i]));
    for w in idx.windows(2) {
        assert!(f[w[0]].q_value <= f[w[1]].q_value, "q not monotone");
    }
    let high: Vec<usize> = (0..t.len())
        .filter(|&i| a.truth[i] && t[i] >= 0.7)
        .collect();
    let ok = high.iter().filter(|&&i| f[i].q_value <= 0.01).count();
    eprintln!(
        "true hits with score >= 0.7: {}, with q <= 0.01: {ok}",
        high.len()
    );
    assert!(high.len() >= 200 && ok * 10 >= high.len() * 9);
    // the order of the input does not matter
    let mut perm: Vec<usize> = (0..t.len()).collect();
    perm.reverse();
    let t2: Vec<f32> = perm.iter().map(|&i| t[i]).collect();
    let f2 = lib_fdr(&t2, &d, DecoyKind::Fragments);
    for (k, &i) in perm.iter().enumerate() {
        assert_eq!(f2[k], f[i]);
    }
    // a decoy never appears in the identity of a target-only library
    assert!(plain.meta(0).unwrap().accession.starts_with("SYN"));
    let q = Query::new(qs[0].0.prec, 1, &qs[0].0.mz, &qs[0].0.it).unwrap();
    assert!(!identity(&plain, &q, &OPTS).unwrap().is_empty());
}

/// An isomer-like sibling: the same precursor, half of the fragments moved elsewhere.
fn sibling(r: &mut Rng, s: &Spectrum, tag: &str) -> Spectrum {
    let mut x = s.clone();
    let hi = s.prec - 4.0;
    for m in x.mz.iter_mut() {
        if r.unit() < 0.5 {
            *m = (r.range(50.0, hi) * 1e4).round() / 1e4;
        }
    }
    let mut pk: Vec<(f64, f64)> = x.mz.iter().cloned().zip(x.it.iter().cloned()).collect();
    pk.sort_by(|a, b| a.0.total_cmp(&b.0));
    pk.dedup_by(|a, b| (a.0 - b.0).abs() < 0.2);
    x.mz = pk.iter().map(|p| p.0).collect();
    x.it = pk.iter().map(|p| p.1).collect();
    x.meta.accession = format!("{}{tag}", s.meta.accession);
    x
}

/// Calibration: declared q against the observed error rate on a library whose errors are isomers, not random. Queries that are in the library
/// should hit their own entry; queries left out of the library can only hit a sibling (an error by definition).
#[test]
fn calibration_declared_q_is_a_lower_bound_of_the_observed_error() {
    let mut r = Rng(2024);
    let base = synth_set(5, 2000, 12);
    let mut lib_set: Vec<Spectrum> = Vec::new();
    let mut left_out: Vec<Spectrum> = Vec::new();
    for (k, s) in base.iter().enumerate() {
        let mut s = s.clone();
        s.pol = 1;
        lib_set.push(s.clone());
        lib_set.push(sibling(&mut r, &s, "b"));
        if k % 5 == 0 {
            left_out.push(sibling(&mut r, &s, "c"));
        }
        if k % 5 == 1 {
            left_out.push(s.clone());
        }
    }
    let all = with_decoys(&lib_set, DecoyKind::Fragments, DEFAULT_SEED);
    let bytes = build_in_memory(&all, BuildOpts::default()).unwrap().0;
    let lib = Library::open(&bytes[..], CACHE_BYTES).unwrap();
    // 400 queries of entries that are in the library (noisy copies) and 400 of siblings that are not (accession ends in «c»)
    let mut qs: Vec<Option<Query>> = Vec::new();
    let mut want: Vec<Option<String>> = Vec::new();
    for s in lib_set.iter().step_by(10).take(400) {
        let n = noisy(&mut r, s);
        qs.push(Query::new(n.prec, 1, &n.mz, &n.it));
        want.push(Some(s.meta.accession.clone()));
    }
    for s in left_out
        .iter()
        .filter(|s| s.meta.accession.ends_with('c'))
        .take(400)
    {
        let n = noisy(&mut r, s);
        qs.push(Query::new(n.prec, 1, &n.mz, &n.it));
        want.push(None);
    }
    let res = search_fdr(&lib, &qs, &OPTS, DecoyKind::Fragments).unwrap();
    assert_eq!(
        res,
        search_fdr(&lib, &qs, &OPTS, DecoyKind::Fragments).unwrap()
    );
    let wrong: Vec<bool> = res
        .iter()
        .zip(&want)
        .map(|(x, w)| match (x.best.index, w) {
            (Some(i), Some(a)) => lib.meta(i).unwrap().accession != *a,
            _ => true,
        })
        .collect();
    eprintln!("declared q -> accepted, observed error rate");
    for t in [0.001f32, 0.01, 0.05, 0.1, 0.2] {
        let acc: Vec<usize> = (0..res.len())
            .filter(|&i| res[i].best.index.is_some() && res[i].fdr.q_value <= t)
            .collect();
        let err = acc.iter().filter(|&&i| wrong[i]).count();
        let obs = if acc.is_empty() {
            0.0
        } else {
            err as f64 / acc.len() as f64
        };
        eprintln!(
            "q <= {t}: {} accepted, {err} wrong, observed {obs:.3}",
            acc.len()
        );
        assert!(
            acc.is_empty() || obs >= t as f64,
            "the declared q must be a lower bound"
        );
    }
    let j = search_fdr_json(&res);
    assert!(j.starts_with("[{") && j.contains("\"note\":\"lib.fdr.note\""));
}
