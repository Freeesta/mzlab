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

/// The pairwise loop `merge_split` had before its m/z index: every later row of the order compared with every row.
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
            if gone[b] || tot[b] <= 0.0 {
                continue;
            }
            if (t.mz[a] - t.mz[b]).abs() > da || (t.rt[a] - t.rt[b]).abs() > rt {
                continue;
            }
            let (apart, both) = resolved(t, a, b);
            if !apart {
                out.push(absorb(t, a, b, both));
                gone[b] = true;
            }
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

fn bits(t: &Aligned) -> (Vec<u64>, Vec<u32>, Vec<u32>, Vec<u32>, Vec<u8>) {
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
