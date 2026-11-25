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
    merge(&mut t, &r, true);
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
    merge(&mut t, &r, true);
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
    merge(&mut t, &r, true);
    assert_eq!(t.len(), 2);
    let mut t = table(&rows);
    merge(&mut t, &Recipe::default(), true);
    assert_eq!(t.len(), 2);
}

/// A cell of a row: (area, apex RT, width), or none where the file has no detected peak.
type Cell = Option<(f64, f32, f32)>;

fn peaks(rows: &[(f64, f64, [Cell; 4])]) -> Aligned {
    let mut t = Aligned {
        n_files: 4,
        ..Aligned::default()
    };
    for (mz, rt, cells) in rows {
        t.mz.push(*mz);
        t.rt.push(*rt);
        for c in cells {
            let (a, apex, w) = c.unwrap_or((0.0, f32::NAN, f32::NAN));
            t.area.push(a);
            t.height.push(a as f32);
            t.apex_rt.push(apex);
            t.width.push(w);
            t.state.push(if c.is_some() {
                crate::aligned::CELL_DETECTED
            } else {
                crate::aligned::CELL_ABSENT
            });
        }
    }
    t
}

/// A strong peak with a broad top (0.30 min wide) and the shoulder the detector cut from it in two of the four files: the shoulder row has areas in
/// two files, so no correlation across files can be computed.
fn shoulder() -> Aligned {
    let top = |a: f64| Some((a, 5.0, 0.3));
    let cut = |a: f64| Some((a, 5.08, 0.1));
    peaks(&[
        (250.2, 5.0, [top(9e4), top(8e4), top(1e5), top(7e4)]),
        (250.25, 5.08, [None, cut(2e4), None, cut(3e4)]),
    ])
}

#[test]
fn a_shoulder_cut_from_a_broad_top_merges_by_overlap_and_logs_every_number() {
    let r = recipe();
    let mut t = shoulder();
    merge(&mut t, &r, true);
    assert_eq!(t.mz, vec![250.2], "the broad row stays");
    let log = r.screen.evidence.as_ref().unwrap().take();
    let m = &log.merged[0];
    assert_eq!(m.rule, "overlap");
    assert_eq!((m.shared, m.threshold), (2, DEFAULT_OVERLAP));
    assert!((m.overlap - 1.0).abs() < 1e-9, "{}", m.overlap);
    assert!(m.r.is_nan(), "fewer than 3 common files: no r");
    assert_eq!(m.height, (1e5, 3e4));
    assert!((m.drt_min - 0.08).abs() < 1e-9 && (m.dmz_da - 0.05).abs() < 1e-9);
    assert!(log.near.is_empty());
}

#[test]
fn dedup_overlap_zero_keeps_both_rows_and_reports_them_as_near() {
    let mut r = recipe();
    r.screen.dedup_overlap = Some(0.0);
    let mut t = shoulder();
    merge(&mut t, &r, true);
    assert_eq!(t.len(), 2);
    let log = r.screen.evidence.as_ref().unwrap().take();
    assert!(log.merged.is_empty());
    assert_eq!(log.near.len(), 1);
    let n = &log.near[0];
    assert_eq!(n.shared, 2);
    assert!((n.overlap - 1.0).abs() < 1e-9 && n.r.is_nan());
}

#[test]
fn peaks_that_do_not_touch_stay_and_a_series_of_both_polarities_is_not_merged() {
    let mut r = recipe();
    r.fill.rt_tol = 0.5;
    let mut t = peaks(&[
        (250.2, 5.0, [Some((9e4, 5.0, 0.1)); 4]),
        (250.2, 5.3, [Some((2e4, 5.3, 0.1)); 4]),
    ]);
    merge(&mut t, &r, true);
    assert_eq!(
        t.len(),
        2,
        "0.3 min apart with 0.1 min wide peaks do not overlap"
    );
    let log = r.screen.evidence.as_ref().unwrap().take();
    assert_eq!(log.near.len(), 1);
    let mut t = shoulder();
    merge(&mut t, &r, false);
    assert_eq!(t.len(), 2);
}

/// Two rows 0.05 Da apart whose peaks (`wa`, `wb` min wide) sit at `rt_a` and `rt_b` in the files flagged in `have_b`; row a is in every file.
fn pair(wa: f32, wb: f32, rt_a: f64, rt_b: f64, areas_b: [Option<f64>; 4]) -> Aligned {
    let a = [9e4, 8e4, 1e5, 7e4].map(|x| Some((x, rt_a as f32, wa)));
    let b = areas_b.map(|x| x.map(|x| (x, rt_b as f32, wb)));
    peaks(&[(250.2, rt_a, a), (250.25, rt_b, b)])
}

fn merged_rows(mut t: Aligned) -> (usize, crate::lrscreen::Log) {
    let mut r = recipe();
    r.fill.rt_tol = 0.5;
    merge(&mut t, &r, true);
    (t.len(), r.screen.evidence.as_ref().unwrap().take())
}

#[test]
fn touching_peaks_of_two_ions_at_the_same_mass_are_not_merged() {
    let all = [Some(2e4); 4];
    let (n, log) = merged_rows(pair(0.1, 0.1, 5.0, 5.1, all));
    assert_eq!((n, log.near.len()), (2, 1));
    assert!(log.near[0].overlap < 1e-4);
}

#[test]
fn a_narrow_peak_inside_a_very_broad_one_is_another_ion() {
    let all = [Some(2e4); 4];
    let (n, log) = merged_rows(pair(0.8, 0.05, 5.0, 5.02, all));
    assert_eq!(n, 2, "narrower share 1.0 but wider share 0.0625");
    let g = &log.near[0];
    assert!((g.overlap - 1.0).abs() < 1e-6 && (g.overlap_wide - 0.0625).abs() < 1e-6);
    // the same pair as a cut shoulder (3x wider) is merged and both shares are logged
    let (n, log) = merged_rows(pair(0.3, 0.1, 5.0, 5.02, all));
    assert_eq!(n, 1);
    assert!((log.merged[0].overlap_wide - 1.0 / 3.0).abs() < 1e-6);
}

#[test]
fn rows_whose_areas_move_against_each_other_in_four_files_are_not_merged() {
    let t = table(&[
        (200.0, 5.0, [1e4, 2e4, 4e4, 8e4]),
        (200.3, 5.0, [8e4, 4e4, 2e4, 1e4]),
    ]);
    let (n, log) = merged_rows(t);
    assert_eq!(n, 2, "same peaks, but r <= -0.5 blocks the overlap rule");
    assert!(log.near[0].r <= APART_R && log.near[0].overlap == 1.0);
}

#[test]
fn a_partial_overlap_just_under_the_threshold_stays_and_just_over_merges() {
    let all = [Some(2e4); 4];
    let (n, log) = merged_rows(pair(0.1, 0.1, 5.0, 5.055, all));
    assert_eq!(n, 2);
    assert!((log.near[0].overlap - 0.45).abs() < 1e-3);
    let (n, log) = merged_rows(pair(0.1, 0.1, 5.0, 5.045, all));
    assert_eq!(n, 1);
    assert!((log.merged[0].overlap - 0.55).abs() < 1e-3);
}

#[test]
fn one_shared_file_is_not_enough_unless_the_series_is_one_file() {
    let one = [None, Some(2e4), None, None];
    let (n, log) = merged_rows(pair(0.3, 0.1, 5.0, 5.08, one));
    assert_eq!((n, log.near[0].shared), (2, 1));
    // a series of one file: the only file is the evidence there is
    let mut t = Aligned {
        n_files: 1,
        ..Aligned::default()
    };
    for (mz, w, a) in [(250.2, 0.3f32, 9e4), (250.25, 0.1, 2e4)] {
        t.mz.push(mz);
        t.rt.push(5.0);
        t.area.push(a);
        t.height.push(a as f32);
        t.apex_rt.push(5.0);
        t.width.push(w);
        t.state.push(crate::aligned::CELL_DETECTED);
    }
    let (n, log) = merged_rows(t);
    assert_eq!((n, log.merged[0].shared), (1, 1));
}
