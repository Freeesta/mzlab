//! `lib_fdr` with linear pointers equals the binary-search version on random lists (ties, empties, non-finite scores), and a precursor decoy
//! never lands at or below 1 Da.

use mzlab_lib::decoy::{lib_fdr, make_decoy, qvalues, DecoyKind, XorShift, SHIFT_MIN};
use mzlab_lib::spec::Spectrum;

/// Reference of the previous implementation (binary searches for both counts).
fn lib_fdr_old(t: &[f32], d: &[f32]) -> Vec<(f32, u32, u32)> {
    let fin = |v: &[f32]| -> Vec<f32> { v.iter().cloned().filter(|x| x.is_finite()).collect() };
    let (mut ts, mut ds) = (fin(t), fin(d));
    ts.sort_by(|a, b| b.total_cmp(a));
    ds.sort_by(|a, b| b.total_cmp(a));
    let ge = |v: &[f32], s: f32| v.partition_point(|&x| x >= s);
    let mut q_of: Vec<(f32, (f32, u32, u32))> = Vec::new();
    let mut run = 1.0f32;
    let mut i = ts.len();
    while i > 0 {
        let s = ts[i - 1];
        let (nt, nd) = (ge(&ts, s), ge(&ds, s));
        run = run.min((nd as f32 / nt as f32).min(1.0));
        q_of.push((s, (run, nt as u32, nd as u32)));
        i = ts.partition_point(|&x| x > s);
    }
    t.iter()
        .map(|&s| {
            if !s.is_finite() {
                return (1.0, 0, 0);
            }
            q_of[q_of.partition_point(|(x, _)| *x < s)].1
        })
        .collect()
}

#[test]
fn lib_fdr_matches_the_binary_search_version_on_random_lists() {
    let mut r = XorShift::new(7);
    for _ in 0..10_000 {
        let nt = (r.next_u64() % 40) as usize;
        let nd = (r.next_u64() % 40) as usize;
        // coarse grid so that ties are frequent; some NaN / infinite scores
        let draw = |r: &mut XorShift| -> f32 {
            match r.next_u64() % 40 {
                0 => f32::NAN,
                1 => f32::INFINITY,
                _ => (r.next_u64() % 12) as f32 / 10.0,
            }
        };
        let t: Vec<f32> = (0..nt).map(|_| draw(&mut r)).collect();
        let d: Vec<f32> = (0..nd).map(|_| draw(&mut r)).collect();
        let new: Vec<(f32, u32, u32)> = lib_fdr(&t, &d, DecoyKind::Fragments)
            .iter()
            .map(|f| (f.q_value, f.n_target_ge, f.n_decoy_ge))
            .collect();
        assert_eq!(new, lib_fdr_old(&t, &d), "t={t:?} d={d:?}");
    }
    assert!(lib_fdr(&[], &[0.5], DecoyKind::Fragments).is_empty());
    assert_eq!(qvalues(&[0.5], &[]), vec![0.0]);
}

#[test]
fn precursor_decoy_never_goes_below_one_dalton() {
    for prec in [3.0, 6.0, 20.0, 51.0, 300.0] {
        for seed in 1..200u64 {
            let s = Spectrum {
                prec,
                pol: 1,
                mz: vec![1.5],
                it: vec![1.0],
                ..Default::default()
            };
            let p = make_decoy(&s, DecoyKind::Precursor, seed);
            assert!(p.prec > 1.0, "prec {prec} seed {seed} -> {}", p.prec);
            assert!((p.prec - prec).abs() >= SHIFT_MIN);
        }
    }
}
