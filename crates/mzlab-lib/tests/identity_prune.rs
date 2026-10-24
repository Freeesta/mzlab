//! The identity search keeps only the best `top` hits while it runs and scores against a prepared query: both must give the bits of the plain way.
mod common;
use mzlab_lib::build::{build_in_memory, BuildOpts};
use mzlab_lib::pair_score::{pair_scores_prepared, Prepared};
use mzlab_lib::read::{Library, CACHE_BYTES};
use mzlab_lib::scores::{pair_scores, Spec};
use mzlab_lib::search::{identity, IdentityOpts, Query, Tol};
use mzlab_lib::spec::{Meta, Spectrum};

fn next(s: &mut u64) -> f64 {
    *s = s
        .wrapping_mul(6364136223846793005)
        .wrapping_add(1442695040888963407);
    (*s >> 11) as f64 / (1u64 << 53) as f64
}

fn synthetic(n: usize) -> Vec<Spectrum> {
    let mut s = 7u64;
    (0..n)
        .map(|k| {
            let prec = 200.0 + (k % 40) as f64 * 0.01;
            let np = 6 + (next(&mut s) * 20.0) as usize;
            // few distinct m/z values so that many candidates share peaks and ties are common
            let mz: Vec<f64> = (0..np)
                .map(|_| 60.0 + (next(&mut s) * 60.0).floor() * 2.0)
                .collect();
            let mut mz = mz;
            mz.sort_by(|a, b| a.total_cmp(b));
            mz.dedup();
            let it: Vec<f64> = mz
                .iter()
                .map(|_| 1.0 + (next(&mut s) * 9.0).floor())
                .collect();
            Spectrum {
                prec,
                pol: 1,
                mz,
                it,
                meta: Meta {
                    name: format!("s{k}"),
                    ..Default::default()
                },
                rt: None,
            }
        })
        .collect()
}

#[test]
fn pruned_top_equals_the_head_of_the_full_list() {
    let set = synthetic(1500);
    let (b, _) = build_in_memory(&set, BuildOpts::default()).unwrap();
    let lib = Library::open(&b[..], CACHE_BYTES).unwrap();
    for k in [0usize, 17, 400, 999] {
        let q = Query::new(set[k].prec, 1, &set[k].mz, &set[k].it).unwrap();
        let run = |top: usize| {
            let o = IdentityOpts {
                prec_tol: Tol::Da(1.0),
                frag_da: 0.01,
                pol: 1,
                top,
            };
            identity(&lib, &q, &o).unwrap()
        };
        let full = run(100_000);
        assert!(full.len() > 300);
        for top in [1usize, 5, 50] {
            let few = run(top);
            assert_eq!(few.len(), top);
            for (a, b) in few.iter().zip(&full) {
                assert_eq!(a.index, b.index);
                assert_eq!(a.entropy.to_bits(), b.entropy.to_bits());
                assert_eq!(a.pairs, b.pairs);
                assert_eq!(a.nl_shared, b.nl_shared);
            }
        }
    }
}

#[test]
fn prepared_scores_have_the_bits_of_pair_scores() {
    let set = synthetic(60);
    let a = Spec {
        mz: set[0].mz.clone(),
        it: set[0].it.clone(),
        prec: set[0].prec,
    };
    let prep = Prepared::new(&a.it);
    for s in &set {
        let b = Spec {
            mz: s.mz.clone(),
            it: s.it.clone(),
            prec: s.prec,
        };
        assert_eq!(
            pair_scores(&a, &b, 0.01),
            pair_scores_prepared(&prep, &a, &b, 0.01)
        );
    }
}
