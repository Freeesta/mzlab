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
