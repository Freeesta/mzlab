//! The windowed `hybrid_pairs` / `gnps_match_prepared` and the allocation-free `common_losses` give the same output as the brute-force versions they
//! replaced (copied here), on random spectrum pairs with ties and peaks sitting on the tolerance edge.

use mzlab_lib::clean::weighted;
use mzlab_lib::scores::{
    common_losses, gnps_match_prepared, hybrid_pairs, match_peaks, pair_term, GnpsMatch, Spec,
};

struct Rng(u64);
impl Rng {
    fn next(&mut self) -> u64 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        self.0
    }
}

/// Peaks on a 0.005 grid (so many differences land exactly on the 0.01 tolerance), sorted, with repeated intensities for ties.
fn spec(r: &mut Rng, grid: u64) -> Spec {
    let n = (r.next() % 25) as usize;
    let mut mz: Vec<f64> = (0..n)
        .map(|_| 50.0 + (r.next() % grid) as f64 * 0.005)
        .collect();
    mz.sort_by(|a, b| a.total_cmp(b));
    let it = (0..n).map(|_| 1.0 + (r.next() % 6) as f64).collect();
    Spec {
        mz,
        it,
        prec: 200.0 + (r.next() % 60) as f64 * 0.5,
    }
}

fn hybrid_old(a: &Spec, b: &Spec, tol: f64) -> Vec<(usize, usize)> {
    let (pa, pb) = (weighted(&a.it), weighted(&b.it));
    let delta = a.prec - b.prec;
    let mut c: Vec<(usize, usize, f64)> = Vec::new();
    for (i, (&ma, &wa)) in a.mz.iter().zip(&pa).enumerate() {
        for (j, (&mb, &wb)) in b.mz.iter().zip(&pb).enumerate() {
            let d = ma - mb;
            if d.abs() <= tol || (d - delta).abs() <= tol {
                c.push((i, j, pair_term(wa, wb)));
            }
        }
    }
    c.sort_by(|x, y| y.2.total_cmp(&x.2));
    let (mut ua, mut ub) = (vec![false; a.mz.len()], vec![false; b.mz.len()]);
    let mut out = Vec::new();
    for (i, j, _) in c {
        if !ua[i] && !ub[j] {
            ua[i] = true;
            ub[j] = true;
            out.push((i, j));
        }
    }
    out
}

fn losses_old(a: &Spec, b: &Spec, tol: f64) -> usize {
    if a.prec <= 0.0 || b.prec <= 0.0 {
        return 0;
    }
    let la: Vec<f64> = a.mz.iter().rev().map(|m| a.prec - m).collect();
    let lb: Vec<f64> = b.mz.iter().rev().map(|m| b.prec - m).collect();
    let ia: Vec<f64> = a.it.iter().rev().copied().collect();
    let ib: Vec<f64> = b.it.iter().rev().copied().collect();
    match_peaks(&la, &ia, &lb, &ib, tol).len()
}

fn gnps_old(
    am: &[f64],
    ai: &[f64],
    bm: &[f64],
    bi: &[f64],
    delta: f64,
    tol: f64,
    modified: bool,
) -> GnpsMatch {
    let mut c: Vec<(usize, usize, f64, bool)> = Vec::new();
    for i in 0..am.len() {
        for j in 0..bm.len() {
            if (am[i] - bm[j]).abs() <= tol {
                c.push((i, j, ai[i] * bi[j], false));
            } else if modified && delta.abs() > tol && (am[i] - (bm[j] + delta)).abs() <= tol {
                c.push((i, j, ai[i] * bi[j], true));
            }
        }
    }
    c.sort_by(|x, y| {
        y.2.total_cmp(&x.2)
            .then(x.3.cmp(&y.3))
            .then(y.0.cmp(&x.0))
            .then(y.1.cmp(&x.1))
    });
    let (mut ua, mut ub) = (vec![false; am.len()], vec![false; bm.len()]);
    let (mut dot, mut na_m, mut n_shifted) = (0.0, 0.0, 0usize);
    let mut pairs = Vec::new();
    for (i, j, w, sh) in c {
        if !ua[i] && !ub[j] {
            ua[i] = true;
            ub[j] = true;
            dot += w;
            na_m += ai[i] * ai[i];
            pairs.push((i, j, sh));
            if sh {
                n_shifted += 1;
            }
        }
    }
    let na: f64 = ai.iter().map(|v| v * v).sum::<f64>().sqrt();
    let nb: f64 = bi.iter().map(|v| v * v).sum::<f64>().sqrt();
    let ratio = |d: f64, x: f64, y: f64| if x > 0.0 && y > 0.0 { d / (x * y) } else { 0.0 };
    GnpsMatch {
        cosine: ratio(dot, na, nb),
        reverse: ratio(dot, na_m.sqrt(), nb),
        matched: pairs.len(),
        n_shifted,
        pairs,
    }
}

#[test]
fn hybrid_pairs_equal_the_brute_force_on_10k_pairs() {
    let mut r = Rng(0x1234_5678_9abc_def1);
    for _ in 0..10_000 {
        let (a, b) = (spec(&mut r, 400), spec(&mut r, 400));
        for tol in [0.01, 0.02] {
            assert_eq!(
                hybrid_pairs(&a, &b, tol),
                hybrid_old(&a, &b, tol),
                "{a:?} {b:?}"
            );
        }
    }
}

#[test]
fn gnps_prepared_equals_the_brute_force_on_10k_pairs() {
    let mut r = Rng(0x0fed_cba9_8765_4321);
    for _ in 0..10_000 {
        let (a, b) = (spec(&mut r, 400), spec(&mut r, 400));
        let ai: Vec<f64> = a.it.iter().map(|v| v.sqrt()).collect();
        let bi: Vec<f64> = b.it.iter().map(|v| v.sqrt()).collect();
        let delta = a.prec - b.prec;
        for modified in [false, true] {
            let new = gnps_match_prepared(&a.mz, &ai, &b.mz, &bi, delta, 0.01, modified);
            assert_eq!(
                new,
                gnps_old(&a.mz, &ai, &b.mz, &bi, delta, 0.01, modified),
                "{a:?} {b:?}"
            );
        }
    }
}

#[test]
fn unsorted_peaks_still_equal_the_brute_force() {
    let mut r = Rng(99);
    for _ in 0..2_000 {
        let (mut a, mut b) = (spec(&mut r, 300), spec(&mut r, 300));
        a.mz.reverse();
        a.it.reverse();
        let h = b.mz.len() / 2;
        b.mz.rotate_left(h);
        b.it.rotate_left(h);
        assert_eq!(hybrid_pairs(&a, &b, 0.01), hybrid_old(&a, &b, 0.01));
        let ai: Vec<f64> = a.it.iter().map(|v| v.sqrt()).collect();
        let bi: Vec<f64> = b.it.iter().map(|v| v.sqrt()).collect();
        assert_eq!(
            gnps_match_prepared(&a.mz, &ai, &b.mz, &bi, 3.0, 0.01, true),
            gnps_old(&a.mz, &ai, &b.mz, &bi, 3.0, 0.01, true)
        );
    }
}

#[test]
fn common_losses_equal_the_allocating_version_on_100k_cases() {
    let mut r = Rng(0xdead_beef_cafe_f00d);
    for _ in 0..100_000 {
        let (mut a, mut b) = (spec(&mut r, 400), spec(&mut r, 400));
        if r.next() % 50 == 0 {
            a.prec = 0.0;
        }
        if r.next() % 50 == 0 {
            b.prec = -1.0;
        }
        assert_eq!(
            common_losses(&a, &b, 0.01),
            losses_old(&a, &b, 0.01),
            "{a:?} {b:?}"
        );
    }
}
