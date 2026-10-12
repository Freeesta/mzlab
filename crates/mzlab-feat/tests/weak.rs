//! The weak tier: which rows of a table of weak peaks are kept and how they join the table of the strong ones.

use mzlab_feat::table::{Aligned, CELL_ABSENT, CELL_DETECTED};

/// A table of 6 files with one row per pattern of detections (`1` detected, `0` absent), at m/z 100, 200, ...
fn table(patterns: &[&str]) -> Aligned {
    let mut a = Aligned::with_capacity(6, patterns.len());
    a.n_files = 6;
    for (i, p) in patterns.iter().enumerate() {
        a.mz.push(100.0 * (i + 1) as f64);
        a.rt.push(2.0);
        for c in p.chars() {
            let on = c == '1';
            a.area.push(if on { 10.0 } else { 0.0 });
            a.height.push(if on { 5.0 } else { 0.0 });
            a.apex_rt.push(2.0);
            a.width.push(0.1);
            a.state.push(if on { CELL_DETECTED } else { CELL_ABSENT });
        }
    }
    a
}

#[test]
fn only_an_unbroken_run_after_the_first_file_is_kept() {
    let mut a = table(&["011110", "111110", "010110", "000111", "001000"]);
    a.retain_recurring(3);
    assert_eq!(a.mz, vec![100.0, 400.0]);
}

#[test]
fn rows_already_held_are_not_appended() {
    let mut strong = table(&["111111"]);
    let weak = table(&["011110", "011110"]);
    // The first weak row has the m/z and RT of the strong one; the second is at m/z 200.
    assert_eq!(strong.append_unheld(&weak, 5.0, 0.1), 1);
    assert_eq!(strong.mz, vec![100.0, 200.0]);
    assert_eq!(strong.state.len(), 12);
}
