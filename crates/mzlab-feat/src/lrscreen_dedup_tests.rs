//! Tests of `lrscreen_dedup`.

use super::*;

/// A table of `rows` = (m/z, RT, areas over 4 files), height equal to the area.
fn table(rows: &[(f64, f64, [f64; 4])]) -> Aligned {
    let mut t = Aligned {
        n_files: 4,
        ..Aligned::default()
    };
    for (mz, rt, a) in rows {
        t.mz.push(*mz);
        t.rt.push(*rt);
        for v in a {
            t.area.push(*v);
            t.height.push(*v as f32);
        }
        t.apex_rt.extend([*rt as f32; 4]);
        t.width.extend([0.1f32; 4]);
        t.state.extend([crate::aligned::CELL_DETECTED; 4]);
    }
    t
}

fn recipe() -> Recipe {
    let mut r = Recipe::lr(0.0, 3e3, 7e3);
    r.detect.polarity = Some(1);
    r.screen.evidence = Some(crate::lrscreen::Evidence::new());
    r
}

#[test]
fn a_row_that_moves_with_a_taller_one_within_the_window_is_dropped_and_logged() {
    let r = recipe();
    let mut t = table(&[
        (200.0, 5.0, [1e4, 2e4, 4e4, 8e4]),
        (200.4, 5.02, [5e3, 1.1e4, 2e4, 3.9e4]),
    ]);
    merge(&mut t, &r);
    assert_eq!(t.mz, vec![200.0]);
    let log = r.screen.evidence.as_ref().unwrap().take();
    assert_eq!(log.merged.len(), 1);
    let m = &log.merged[0];
    assert_eq!((m.kept.0, m.dropped.0), (200.0, 200.4));
    assert!((m.dmz_da - 0.4).abs() < 1e-9 && m.r > 0.99);
}

#[test]
fn neighbours_one_dalton_apart_uncorrelated_rows_and_distant_rt_stay() {
    let r = recipe();
    let same = [1e4, 2e4, 4e4, 8e4];
    let mut t = table(&[
        (200.0, 5.0, same),
        (201.0, 5.0, same),
        (210.0, 5.0, same),
        (210.3, 5.0, [8e4, 4e4, 2e4, 1e4]),
        (220.0, 5.0, same),
        (220.3, 6.0, same),
    ]);
    merge(&mut t, &r);
    assert_eq!(t.len(), 6, "{:?}", t.mz);
}

#[test]
fn off_and_the_high_resolution_axis_keep_the_table() {
    let rows = [
        (200.0, 5.0, [1e4, 2e4, 4e4, 8e4]),
        (200.4, 5.0, [1e4, 2e4, 4e4, 8e4]),
    ];
    let mut r = recipe();
    r.screen.dedup = Switch::Off;
    let mut t = table(&rows);
    merge(&mut t, &r);
    assert_eq!(t.len(), 2);
    let mut t = table(&rows);
    merge(&mut t, &Recipe::default());
    assert_eq!(t.len(), 2);
}
