//! The merge of split rows on small hand-made tables: a kinetic cut in two is joined, two resolved peaks stay apart, 0 switches it off.

use super::*;
use crate::aligned::CELL_FILLED;

/// m/z, RT and per file Some((area, apex)) for a detected peak or None for a forced cell of area 10.
type RowIn = (f64, f64, Vec<Option<(f64, f32)>>);

/// A table of `rows`.
fn table(rows: &[RowIn]) -> Aligned {
    let n = rows[0].2.len();
    let mut t = Aligned {
        n_files: n,
        ..Aligned::default()
    };
    for (mz, rt, cells) in rows {
        t.mz.push(*mz);
        t.rt.push(*rt);
        for c in cells {
            let (a, apex, s) = match c {
                Some((a, x)) => (*a, *x, CELL_DETECTED),
                None => (10.0, f32::NAN, CELL_FILLED),
            };
            t.area.push(a);
            t.height.push(a as f32 / 10.0);
            t.apex_rt.push(apex);
            t.width.push(if apex.is_finite() { 0.3 } else { f32::NAN });
            t.state.push(s);
        }
    }
    t
}

#[test]
fn a_kinetic_split_in_two_rows_is_joined() {
    let mut t = table(&[
        (
            250.1,
            5.00,
            vec![
                Some((100.0, 5.0)),
                Some((900.0, 5.0)),
                Some((800.0, 5.0)),
                None,
                None,
            ],
        ),
        (
            250.3,
            5.22,
            vec![None, None, None, Some((700.0, 5.2)), Some((600.0, 5.2))],
        ),
    ]);
    let m = merge_split(&mut t, 0.3, 0.3);
    assert_eq!(m.len(), 1);
    assert_eq!((m[0].kept.row, m[0].absorbed.row), (0, 1));
    assert_eq!(m[0].taken, vec![3, 4]);
    assert_eq!(m[0].both, 0);
    assert!((0..5).all(|k| t.state[t.cell(0, k)] == CELL_DETECTED));
    assert_eq!(t.area[t.cell(0, 4)], 600.0);
    assert!((0..5).all(|k| t.state[t.cell(1, k)] == CELL_ABSENT));
    // the rows as they were stay in the log
    assert_eq!(m[0].absorbed.cells[3], (700.0, CELL_DETECTED));
    assert_eq!(m[0].kept.cells[3], (10.0, CELL_FILLED));
}

#[test]
fn the_same_peak_in_two_rows_keeps_the_larger_cell() {
    let mut t = table(&[
        (364.1, 8.0, vec![Some((1000.0, 8.0)), Some((500.0, 8.0))]),
        (364.37, 8.02, vec![Some((300.0, 8.01)), Some((800.0, 8.01))]),
    ]);
    let m = merge_split(&mut t, 0.3, 0.3);
    assert_eq!(m.len(), 1);
    assert_eq!(m[0].both, 2);
    assert_eq!(m[0].taken, vec![1]);
    assert_eq!(
        (t.area[t.cell(0, 0)], t.area[t.cell(0, 1)]),
        (1000.0, 800.0)
    );
}

#[test]
fn two_resolved_peaks_stay_apart() {
    let mut t = table(&[
        (300.1, 6.0, vec![Some((500.0, 6.0)), Some((600.0, 6.0))]),
        (300.2, 6.25, vec![Some((400.0, 6.25)), Some((450.0, 6.25))]),
    ]);
    assert!(merge_split(&mut t, 0.3, 0.3).is_empty());
    assert_eq!(t.state[t.cell(1, 0)], CELL_DETECTED);
}

#[test]
fn zero_switches_the_merge_off_and_far_rows_never_merge() {
    let rows = [
        (250.1, 5.0, vec![Some((100.0, 5.0)), None]),
        (250.3, 5.2, vec![None, Some((100.0, 5.2))]),
    ];
    let mut t = table(&rows);
    assert!(merge_split(&mut t, 0.0, 0.3).is_empty());
    assert!(merge_split(&mut t, 0.3, 0.0).is_empty());
    assert!(merge_split(&mut t, 0.1, 0.3).is_empty());
    assert!(merge_split(&mut t, 0.3, 0.1).is_empty());
    assert_eq!(merge_split(&mut t, 0.3, 0.3).len(), 1);
}

#[test]
fn the_evidence_words_separate_a_rejected_peak_from_the_gap_filling() {
    use crate::aligned::{CELL_NA, CELL_TARGETED};
    assert_eq!(evidence_word(CELL_DETECTED, false), "detected");
    assert_eq!(evidence_word(CELL_FILLED, true), "forced");
    assert_eq!(evidence_word(CELL_TARGETED, false), "forced");
    assert_eq!(evidence_word(CELL_FILLED, false), "filled");
    assert_eq!(evidence_word(CELL_ABSENT, false), "absent");
    assert_eq!(evidence_word(CELL_NA, true), "absent");
}

/// The pairwise loop `merge_split` had before its m/z index, kept independent of it: every later row of the order compared with every row,
/// resolved peaks and the move of the cells written out again. The only change is the comparison as "within" (`<=`), so that a non-finite m/z or
/// RT never merges, as in `merge_split`.
fn merge_pairwise(t: &mut Aligned, da: f64, rt: f64) -> Vec<Merge> {
    let mut ord: Vec<usize> = (0..t.len()).collect();
    let tot: Vec<f64> = ord.iter().map(|&r| detected_area(t, r)).collect();
    mzlab_core::sortc::by(&mut ord, |&a, &b| tot[b].total_cmp(&tot[a]).then(a.cmp(&b)));
    let mut gone = vec![false; t.len()];
    let mut out = Vec::new();
    for (i, &a) in ord.iter().enumerate() {
        if gone[a] || tot[a] <= 0.0 {
            continue;
        }
        for &b in &ord[i + 1..] {
            let near = (t.mz[a] - t.mz[b]).abs() <= da && (t.rt[a] - t.rt[b]).abs() <= rt;
            if gone[b] || tot[b] <= 0.0 || !near {
                continue;
            }
            let (mut both, mut apart) = (0, false);
            for k in 0..t.n_files {
                let (x, y) = (t.cell(a, k), t.cell(b, k));
                if t.state[x] == CELL_DETECTED && t.state[y] == CELL_DETECTED {
                    both += 1;
                    let d = (t.apex_rt[x] - t.apex_rt[y]).abs();
                    let w = t.width[x].max(t.width[y]);
                    if d.is_finite() && w.is_finite() && d > 0.5 * w {
                        apart = true;
                        break;
                    }
                }
            }
            if apart {
                continue;
            }
            let (ka, kb) = (was(t, a), was(t, b));
            let mut taken = Vec::new();
            for k in 0..t.n_files {
                let (x, y) = (t.cell(a, k), t.cell(b, k));
                let dx = t.state[x] == CELL_DETECTED;
                if t.state[y] == CELL_DETECTED && (!dx || t.area[y] > t.area[x]) {
                    t.area[x] = t.area[y];
                    t.height[x] = t.height[y];
                    t.apex_rt[x] = t.apex_rt[y];
                    t.width[x] = t.width[y];
                    t.state[x] = t.state[y];
                    taken.push(k);
                }
                t.area[y] = 0.0;
                t.height[y] = 0.0;
                t.apex_rt[y] = f32::NAN;
                t.width[y] = f32::NAN;
                t.state[y] = CELL_ABSENT;
            }
            out.push(Merge {
                kept: ka,
                absorbed: kb,
                taken,
                both,
            });
            gone[b] = true;
        }
    }
    out
}

/// A table of `n` rows crowded in m/z and RT (many partners per row), random states, areas and apexes.
fn crowded(n: usize, files: usize, seed: u64) -> Aligned {
    let mut s = seed;
    let mut next = move || {
        s = s
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        (s >> 11) as f64 / (1u64 << 53) as f64
    };
    let mut t = Aligned {
        n_files: files,
        ..Aligned::default()
    };
    for _ in 0..n {
        t.mz.push(200.0 + (next() * 40.0).floor() * 0.1 + next() * 0.05);
        t.rt.push(5.0 + next() * 2.0);
        for _ in 0..files {
            let det = next() < 0.4;
            t.area.push(if det {
                1.0 + (next() * 1e3).floor()
            } else {
                5.0
            });
            t.height.push(1.0);
            t.apex_rt.push(if det {
                (5.0 + next() * 2.0) as f32
            } else {
                f32::NAN
            });
            t.width.push(if det {
                (0.05 + next() * 0.3) as f32
            } else {
                f32::NAN
            });
            t.state.push(if det { CELL_DETECTED } else { CELL_FILLED });
        }
    }
    t
}

/// The columns of a table as bits: area, height, apex RT, width and state.
type Bits = (Vec<u64>, Vec<u32>, Vec<u32>, Vec<u32>, Vec<u8>);

fn bits(t: &Aligned) -> Bits {
    (
        t.area.iter().map(|x| x.to_bits()).collect(),
        t.height.iter().map(|x| x.to_bits()).collect(),
        t.apex_rt.iter().map(|x| x.to_bits()).collect(),
        t.width.iter().map(|x| x.to_bits()).collect(),
        t.state.clone(),
    )
}

#[test]
fn the_mz_index_merges_exactly_as_the_pairwise_loop() {
    for (seed, n) in [(1, 50), (2, 300), (3, 800)] {
        for (da, rt) in [(0.3, 0.3), (0.1, 0.5), (1.0, 0.2)] {
            let (mut a, mut b) = (crowded(n, 6, seed), crowded(n, 6, seed));
            let (ma, mb) = (merge_split(&mut a, da, rt), merge_pairwise(&mut b, da, rt));
            assert!(!ma.is_empty(), "the table has merges to compare");
            assert_eq!(ma.len(), mb.len(), "{seed} {da} {rt}");
            for (x, y) in ma.iter().zip(&mb) {
                assert_eq!(
                    (x.kept.row, x.absorbed.row, &x.taken, x.both),
                    (y.kept.row, y.absorbed.row, &y.taken, y.both)
                );
            }
            assert!(bits(&a) == bits(&b), "{seed} {da} {rt}");
        }
    }
}

/// The merges of both paths are the same, in the same order, and leave the same bits.
fn same_as_pairwise(t: &Aligned, da: f64, rt: f64, what: &str) -> Vec<Merge> {
    let (mut a, mut b) = (t.clone(), t.clone());
    let (ma, mb) = (merge_split(&mut a, da, rt), merge_pairwise(&mut b, da, rt));
    assert_eq!(ma.len(), mb.len(), "{what}");
    for (x, y) in ma.iter().zip(&mb) {
        assert_eq!(
            (x.kept.row, x.absorbed.row, &x.taken, x.both),
            (y.kept.row, y.absorbed.row, &y.taken, y.both),
            "{what}"
        );
    }
    assert!(bits(&a) == bits(&b), "{what}");
    ma
}

/// Row `r` of `t` copied over row `to` with m/z `mz` and RT `rt`: the same cells (a tied detected total) at a chosen place.
fn place(t: &mut Aligned, r: usize, to: usize, mz: f64, rt: f64) {
    for k in 0..t.n_files {
        let (x, y) = (t.cell(r, k), t.cell(to, k));
        t.area[y] = t.area[x];
        t.height[y] = t.height[x];
        t.apex_rt[y] = t.apex_rt[x];
        t.width[y] = t.width[x];
        t.state[y] = t.state[x];
    }
    t.mz[to] = mz;
    t.rt[to] = rt;
}

#[test]
fn the_mz_index_matches_the_pairwise_loop_on_the_edge_cases() {
    let da = 0.3;
    let mut t = crowded(400, 6, 7);
    let (m, r) = (t.mz[0], t.rt[0]);
    // the same m/z, exactly ±da, one ulp inside and outside of ±da, an RT exactly at the limit: each a copy of row 0 (tied totals)
    let at = [
        (m, r),
        (m + da, r),
        (m - da, r),
        ((m + da).next_down(), r),
        ((m + da).next_up(), r),
        ((m - da).next_up(), r),
        ((m - da).next_down(), r),
        (m, r + 0.3),
        (m, (r + 0.3).next_up()),
    ];
    for (i, &(x, y)) in at.iter().enumerate() {
        place(&mut t, 0, 1 + i, x, y);
    }
    // non-finite and negative m/z and RT among the crowd
    let odd = [
        (f64::NAN, r),
        (-f64::NAN, r),
        (f64::INFINITY, r),
        (f64::NEG_INFINITY, r),
        (f64::INFINITY, r),
        (m, f64::NAN),
        (m, f64::INFINITY),
        (-m, r),
        (-m + 0.1, r),
        (m, -r),
        (m, -r + 0.1),
    ];
    for (i, &(x, y)) in odd.iter().enumerate() {
        place(&mut t, 20 + i, 20 + i, x, y);
    }
    for rt in [0.3, 0.5] {
        let ms = same_as_pairwise(&t, da, rt, &format!("edges {rt}"));
        assert!(!ms.is_empty());
        for g in ms.iter().flat_map(|x| [&x.kept, &x.absorbed]) {
            assert!(g.mz.is_finite() && g.rt.is_finite(), "{g:?}");
        }
    }
    // the copies that fall within ±da of row 0 (or of a neighbour) are merged; a NaN or an infinite m/z never
    let ms = same_as_pairwise(&t, da, 0.3, "edges");
    assert!(ms.iter().any(|x| x.kept.row == 0 || x.absorbed.row == 0));
}

#[test]
fn a_non_finite_mz_or_rt_is_never_merged() {
    for (mz, rt) in [
        (f64::NAN, 5.0),
        (-f64::NAN, 5.0),
        (f64::INFINITY, 5.0),
        (200.0, f64::NAN),
        (200.0, f64::NEG_INFINITY),
    ] {
        let mut t = table(&[
            (200.0, 5.0, vec![Some((100.0, 5.0)), None]),
            (mz, rt, vec![None, Some((90.0, 5.1))]),
            (mz, rt, vec![None, Some((80.0, 5.1))]),
        ]);
        assert!(merge_split(&mut t, 0.3, 0.3).is_empty(), "{mz} {rt}");
    }
}

#[test]
fn the_merges_follow_the_rows_and_not_their_order_in_the_table() {
    for seed in [11, 12, 13] {
        let mut t = crowded(300, 6, seed);
        // distinct detected totals: the order of the rows then depends on the areas alone
        for (c, a) in t.area.iter_mut().enumerate() {
            *a += c as f64 * 1e-7;
        }
        let tot: Vec<f64> = (0..t.len()).map(|r| detected_area(&t, r)).collect();
        let mut sorted = tot.clone();
        sorted.sort_by(f64::total_cmp);
        assert!(sorted.windows(2).all(|w| w[0] != w[1] || w[0] == 0.0));
        // a permutation of the rows: row j of `p` is row perm[j] of `t`
        let mut perm: Vec<usize> = (0..t.len()).collect();
        let mut s = seed;
        for j in (1..perm.len()).rev() {
            s = s.wrapping_mul(6364136223846793005).wrapping_add(1);
            perm.swap(j, (s >> 33) as usize % (j + 1));
        }
        let mut p = Aligned {
            n_files: t.n_files,
            ..Aligned::default()
        };
        for &r in &perm {
            p.mz.push(t.mz[r]);
            p.rt.push(t.rt[r]);
            for k in 0..t.n_files {
                let c = t.cell(r, k);
                p.area.push(t.area[c]);
                p.height.push(t.height[c]);
                p.apex_rt.push(t.apex_rt[c]);
                p.width.push(t.width[c]);
                p.state.push(t.state[c]);
            }
        }
        let a = same_as_pairwise(&t, 0.3, 0.3, "table");
        let b = same_as_pairwise(&p, 0.3, 0.3, "permuted");
        let back: Vec<(usize, usize, Vec<usize>)> = b
            .iter()
            .map(|x| (perm[x.kept.row], perm[x.absorbed.row], x.taken.clone()))
            .collect();
        let here: Vec<(usize, usize, Vec<usize>)> = a
            .iter()
            .map(|x| (x.kept.row, x.absorbed.row, x.taken.clone()))
            .collect();
        assert!(!here.is_empty());
        assert_eq!(here, back, "{seed}");
    }
}
