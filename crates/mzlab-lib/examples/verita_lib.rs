//! `cargo run --release -p mzlab-lib --example verita_lib -- LIB QUERIES [DECOY_SEED]` -> JSON on stdout (test 5 of the workflow study).
//! LIB and QUERIES are MassBank/MSP/MGF/JSON files of two different public libraries; the truth is the first block of the InChIKey. For each score
//! (entropy, cosine, modified cosine) it counts the rank-1 hits among the targets and the correct ones at q <= 1 % with fragment decoys. No clock.
use mzlab_lib::build::{build_in_memory, BuildOpts};
use mzlab_lib::decoy::{is_decoy, lib_fdr, with_decoys, DecoyKind};
use mzlab_lib::parse::{detect, parse_all};
use mzlab_lib::read::{Library, CACHE_BYTES};
use mzlab_lib::scores::modified_cosine;
use mzlab_lib::search::{identity, IdentityOpts, Query, Tol};
use mzlab_lib::spec::Spectrum;
use serde_json::json;

const SCORES: [&str; 3] = ["cosine", "modified_cosine", "spectral_entropy"];
const TOP: usize = 30;
const FRAG_DA: f64 = 0.01;

fn read(path: &str) -> Vec<Spectrum> {
    let b = std::fs::read(path).unwrap_or_else(|e| panic!("{path}: {e}"));
    let fmt = detect(path, &b[..b.len().min(4096)]).expect("format");
    parse_all(fmt, &b).0
}

fn key14(s: &str) -> String {
    s.chars().take(14).collect::<String>().to_uppercase()
}

fn main() {
    let a: Vec<String> = std::env::args().collect();
    let seed: u64 = a.get(3).map_or(1, |s| s.parse().expect("seed"));
    let targets = read(&a[1]);
    let known: std::collections::BTreeSet<String> = targets
        .iter()
        .map(|s| key14(&s.meta.inchikey))
        .filter(|k| !k.is_empty())
        .collect();
    let (bytes, _) = build_in_memory(
        &with_decoys(&targets, DecoyKind::Fragments, seed),
        BuildOpts::default(),
    )
    .expect("build");
    let lib = Library::open(&bytes[..], CACHE_BYTES).expect("open");
    let opts = IdentityOpts {
        prec_tol: Tol::Ppm(10.0),
        frag_da: FRAG_DA,
        pol: 1,
        top: TOP,
    };
    // per score: (target score, decoy score, correct) of every query that has a truth
    let mut rows: Vec<Vec<(f32, f32, bool)>> = vec![Vec::new(); SCORES.len()];
    let (mut n_queries, mut n_truth) = (0usize, 0usize);
    for s in read(&a[2]) {
        n_queries += 1;
        let truth = key14(&s.meta.inchikey);
        let Some(q) = Query::new(s.prec, s.pol, &s.mz, &s.it) else {
            continue;
        };
        if !known.contains(&truth) {
            continue;
        }
        n_truth += 1;
        let hits = identity(
            &lib,
            &q,
            &IdentityOpts {
                pol: s.pol,
                ..opts.clone()
            },
        )
        .expect("identity");
        for (k, row) in rows.iter_mut().enumerate() {
            let (mut tb, mut db, mut tkey) = (f32::NAN, 0.0f32, String::new());
            for h in &hits {
                let m = lib.meta(h.index).expect("meta");
                let v = match k {
                    0 => h.cosine,
                    1 => modified_cosine(&q.spec, &h.spec, FRAG_DA).cosine,
                    _ => h.entropy,
                } as f32;
                if is_decoy(&m.accession) {
                    db = db.max(v);
                } else if tb.is_nan() || v > tb {
                    (tb, tkey) = (v, key14(&m.inchikey));
                }
            }
            row.push((tb, db, !tb.is_nan() && tkey == truth));
        }
    }
    let mut out = json!({"queries": n_queries, "with_truth": n_truth, "targets": targets.len(), "decoy_seed": seed, "scores": {}});
    for (k, name) in SCORES.iter().enumerate() {
        let r = &rows[k];
        let (t, d): (Vec<f32>, Vec<f32>) = (
            r.iter().map(|x| x.0).collect(),
            r.iter().map(|x| x.1).collect(),
        );
        let q = lib_fdr(&t, &d, DecoyKind::Fragments);
        let ok = |i: usize| !t[i].is_nan();
        let rank1 = r.iter().filter(|x| x.2).count();
        let pass = (0..r.len())
            .filter(|&i| ok(i) && q[i].q_value <= 0.01)
            .collect::<Vec<_>>();
        let pass_ok = pass.iter().filter(|&&i| r[i].2).count();
        out["scores"][*name] = json!({"rank1": rank1, "rank1_rate": rank1 as f64 / n_truth.max(1) as f64, "q01": pass.len(), "q01_correct": pass_ok});
    }
    println!("{}", serde_json::to_string_pretty(&out).unwrap());
}
