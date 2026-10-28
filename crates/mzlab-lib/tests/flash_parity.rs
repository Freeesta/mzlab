//! The flat accumulators of `flash_top` give the same scores and pair counts as the pair-collecting search they replace (kept here as the reference),
//! also when pairs compete (two library peaks in the window of one query peak, or the reverse), and the `keep`/`accept` shortcut equals filtering the full result.
mod common;
use common::flash_pairs::reference;
use common::Rng;
use mzlab_lib::build::{build_in_memory, BuildOpts};
use mzlab_lib::flash::{flash_top, FlashAcc};
use mzlab_lib::read::{Library, CACHE_BYTES};
use mzlab_lib::search::{flash_scores_with, FlashMode, Query};
use mzlab_lib::spec::{Meta, Spectrum};

fn dense(r: &mut Rng, i: usize) -> Spectrum {
    let prec = 150.0 + (r.unit() * 35000.0).round() / 100.0;
    let n = 5 + (r.next() % 26) as usize;
    let mut mz: Vec<f64> = (0..n)
        .map(|_| 50.0 + (r.next() % 120) as f64 * 2.0 + r.range(-0.1, 0.1))
        .collect();
    mz.sort_by(|a, b| a.total_cmp(b));
    mz.dedup_by(|a, b| (*a - *b).abs() < 0.02);
    let it = mz.iter().map(|_| 1.0 + r.unit() * 999.0).collect();
    Spectrum {
        prec,
        pol: 1,
        mz,
        it,
        meta: Meta {
            name: format!("D{i}"),
            ..Meta::default()
        },
        rt: None,
        ri: None,
        ei: false,
    }
}

fn queries(set: &[Spectrum], r: &mut Rng, n: usize) -> Vec<Query> {
    (0..n)
        .filter_map(|j| {
            let s = &set[(j * 7 + 3) % set.len()];
            let keep: Vec<usize> = (0..s.mz.len()).filter(|_| r.unit() > 0.2).collect();
            let mz: Vec<f64> = keep
                .iter()
                .map(|&i| s.mz[i] + r.range(-0.08, 0.08))
                .collect();
            let it: Vec<f64> = keep.iter().map(|&i| s.it[i] * r.range(0.6, 1.4)).collect();
            // every third query is an analogue: the precursor is shifted and the fragments above the shift move with it
            let shift = if j % 3 == 0 { r.range(5.0, 80.0) } else { 0.0 };
            let mz: Vec<f64> = mz
                .iter()
                .map(|m| {
                    if shift > 0.0 && *m > 120.0 {
                        m + shift
                    } else {
                        *m
                    }
                })
                .collect();
            Query::new(s.prec + shift, 1, &mz, &it)
        })
        .collect()
}

#[test]
fn flat_accumulators_equal_the_pair_search_on_2000_queries() {
    let mut r = Rng(99);
    let set: Vec<Spectrum> = (0..500).map(|i| dense(&mut r, i)).collect();
    let (b, _) = build_in_memory(&set, BuildOpts::default()).unwrap();
    let lib = Library::open(&b[..], CACHE_BYTES).unwrap();
    let all = lib.ranges(1, f64::NEG_INFINITY, f64::INFINITY).unwrap();
    let qs = queries(&set, &mut r, 700);
    let mut acc = FlashAcc::new();
    let (mut searches, mut competing) = (0, 0);
    for q in &qs {
        for mode in [FlashMode::Open, FlashMode::NeutralLoss, FlashMode::Hybrid] {
            let (want, c) = reference(&lib, q, mode, 0.3, &all);
            let got = flash_scores_with(&lib, q, mode, 0.3, &all).unwrap();
            let shared = flash_top(&lib, q, mode, 0.3, &all, &mut acc, None, |_| true).unwrap();
            for g in [&got, &shared] {
                assert_eq!(g.len(), want.len(), "{mode:?}");
                for (a, b) in g.iter().zip(&want) {
                    assert_eq!((a.index, a.pairs), (b.index, b.pairs), "{mode:?}");
                    assert!(
                        (a.score - b.score).abs() < 1e-9,
                        "{mode:?} {} vs {}",
                        a.score,
                        b.score
                    );
                }
            }
            // keep + accept = filter + truncate of the full result
            let pick = flash_top(&lib, q, mode, 0.3, &all, &mut acc, Some(25), |s| {
                s.pairs >= 2
            })
            .unwrap();
            let expect: Vec<_> = want.iter().filter(|s| s.pairs >= 2).take(25).collect();
            assert_eq!(pick.len(), expect.len());
            for (a, b) in pick.iter().zip(expect) {
                assert_eq!((a.index, a.pairs), (b.index, b.pairs));
            }
            searches += 1;
            competing += c;
        }
    }
    assert!(searches >= 2000, "{searches} searches");
    assert!(
        competing > 100,
        "the test must exercise competing pairs ({competing})"
    );
}

#[test]
fn restricted_ranges_leave_the_accumulators_clean() {
    let mut r = Rng(5);
    let set: Vec<Spectrum> = (0..200).map(|i| dense(&mut r, i)).collect();
    let (b, _) = build_in_memory(&set, BuildOpts::default()).unwrap();
    let lib = Library::open(&b[..], CACHE_BYTES).unwrap();
    let q = &queries(&set, &mut r, 3)[1];
    #[allow(clippy::single_range_in_vec_init)]
    let half = [0..100u32];
    let mut acc = FlashAcc::new();
    let a = flash_top(
        &lib,
        q,
        FlashMode::Hybrid,
        0.3,
        &half,
        &mut acc,
        None,
        |_| true,
    )
    .unwrap();
    assert!(a.iter().all(|s| s.index < 100));
    // the same accumulators, another query and range, equal fresh ones
    let all = lib.ranges(0, f64::NEG_INFINITY, f64::INFINITY).unwrap();
    let again = flash_top(&lib, q, FlashMode::Open, 0.3, &all, &mut acc, None, |_| {
        true
    })
    .unwrap();
    let fresh = flash_scores_with(&lib, q, FlashMode::Open, 0.3, &all).unwrap();
    assert_eq!(again.len(), fresh.len());
    for (x, y) in again.iter().zip(&fresh) {
        assert_eq!(
            (x.index, x.pairs, x.score.to_bits()),
            (y.index, y.pairs, y.score.to_bits())
        );
    }
}
