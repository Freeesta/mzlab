//! AU-7: the blank subtraction at unit mass: on heights, and with the blanks matched by batch or matrix; every cell is height (or area) minus the fold
//! times the blank of its own group, not below 0, and the counts say why every row was dropped.

use mzlab_feat::filters::Rule;
use mzlab_feat::filters_blank::{BlankOpts, Labels, Matching, Quantity};
use mzlab_feat::norm::Method;
use mzlab_feat::samples::{propose, Sample};
use mzlab_feat::table::{Aligned, CELL_ABSENT, CELL_DETECTED};
use mzlab_feat::view;

/// Files: blank A, sample A1, sample A2, blank B, sample B1, dark. Batches A, A, A, B, B, A.
const KINDS: [&str; 6] = ["blank", "sample", "sample", "blank", "sample", "dark"];
const BATCH: [&str; 6] = ["A", "A", "A", "B", "B", "A"];

/// `cells[row][file]` = (area, height).
fn table(cells: &[[(f64, f32); 6]]) -> (Aligned, Vec<Sample>) {
    let names: Vec<(String, String)> = (0..6)
        .map(|k| (format!("f{k}.mzML"), "00".repeat(32)))
        .collect();
    let mut s = propose(&names);
    for (x, k) in s.iter_mut().zip(KINDS) {
        x.kind = k.into();
    }
    let mut t = Aligned::with_capacity(6, cells.len());
    for (row, r) in cells.iter().enumerate() {
        t.mz.push(100.0 + row as f64);
        t.rt.push(5.0);
        for &(a, h) in r {
            t.area.push(a);
            t.height.push(h);
            t.apex_rt.push(5.0);
            t.width.push(0.1);
            t.state
                .push(if a > 0.0 { CELL_DETECTED } else { CELL_ABSENT });
        }
    }
    (t, s)
}

fn labels() -> Labels {
    Labels {
        batch: BATCH.iter().map(|x| x.to_string()).collect(),
        ..Labels::default()
    }
}

fn rule(q: Quantity, m: Matching) -> Rule {
    Rule::BlankWith(
        2.0,
        BlankOpts {
            quantity: q,
            matching: m,
            subtract: true,
            ..BlankOpts::default()
        },
    )
}

const ROW0: [(f64, f32); 6] = [
    (10.0, 100.0),
    (500.0, 900.0),
    (400.0, 700.0),
    (30.0, 300.0),
    (600.0, 1000.0),
    (50.0, 50.0),
];

#[test]
fn heights_are_subtracted_by_batch_and_floored_at_zero() {
    let (t, s) = table(&[ROW0]);
    let v = view::build_with(
        &t,
        &s,
        &Method::None,
        &[rule(Quantity::Height, Matching::Batch)],
        labels(),
    )
    .unwrap();
    assert_eq!(v.rows, vec![0]);
    // batch A blank height 100, batch B blank height 300, fold 2: cut 200 and 600
    assert_eq!(v.height_at(&t, 0, 1), Some(700.0));
    assert_eq!(v.height_at(&t, 0, 2), Some(500.0));
    assert_eq!(v.height_at(&t, 0, 4), Some(400.0));
    // the dark file takes all blanks (mean 200, cut 400): 50 - 400 -> 0
    assert_eq!(v.height_at(&t, 0, 5), Some(0.0));
    // the blanks stay; the areas are not touched by a height subtraction
    assert_eq!(v.height_at(&t, 0, 0), Some(100.0));
    assert_eq!(v.area_at(&t, 0, 1), Some(500.0));
}

#[test]
fn with_all_blanks_one_level_serves_every_sample() {
    let (t, s) = table(&[ROW0]);
    let v = view::build(
        &t,
        &s,
        &Method::None,
        &[rule(Quantity::Height, Matching::All)],
    )
    .unwrap();
    // mean of the blank heights 200, cut 400
    assert_eq!(v.height_at(&t, 0, 1), Some(500.0));
    assert_eq!(v.height_at(&t, 0, 4), Some(600.0));
}

#[test]
fn area_subtraction_by_matrix_uses_the_matrix_blank() {
    let (t, s) = table(&[ROW0]);
    let l = Labels {
        matrix: BATCH.iter().map(|x| x.to_string()).collect(),
        ..Labels::default()
    };
    let v = view::build_with(
        &t,
        &s,
        &Method::None,
        &[rule(Quantity::Area, Matching::Matrix)],
        l,
    )
    .unwrap();
    assert_eq!(v.area_at(&t, 0, 1), Some(480.0)); // 500 - 2 * 10
    assert_eq!(v.area_at(&t, 0, 4), Some(540.0)); // 600 - 2 * 30
    assert_eq!(
        v.height_at(&t, 0, 1),
        Some(900.0),
        "an area subtraction leaves the heights"
    );
}

#[test]
fn the_counts_give_the_evidence_of_every_dropped_row() {
    let mut weak = ROW0;
    weak[1] = (100.0, 150.0); // best height 150 < 2 * 100 in batch A and 300 in B
    weak[2] = (90.0, 120.0);
    weak[4] = (100.0, 500.0);
    let (t, s) = table(&[ROW0, weak]);
    let v = view::build_with(
        &t,
        &s,
        &Method::None,
        &[rule(Quantity::Height, Matching::Batch)],
        labels(),
    )
    .unwrap();
    assert_eq!(v.rows, vec![0]);
    let counts = v.counts_json();
    // row id 2 (ids start at 1): the best fold is 500 / 300 = 1.666...
    assert!(counts.contains("\"blank\":{\"quantity\":\"height\",\"fold\":2,\"groups\":2,\"dropped\":[[2,500,300,1.6666"), "{counts}");
    assert!(counts.contains("\"discarded\":[1]"), "{counts}");
}

#[test]
fn the_plain_ratio_filter_gives_the_same_evidence() {
    // row 1 (id 1): best sample area 600 against a blank mean of 20; row 2: best 30 against 20 (fold 1.5 < 2); row 3: no peak in the blanks
    let mut weak = ROW0;
    weak[1] = (30.0, 0.0);
    weak[2] = (25.0, 0.0);
    weak[4] = (20.0, 0.0);
    let mut clean = ROW0;
    clean[0] = (0.0, 0.0);
    clean[3] = (0.0, 0.0);
    let (t, s) = table(&[ROW0, weak, clean]);
    let v = view::build(&t, &s, &Method::None, &[Rule::Blank(2.0)]).unwrap();
    assert_eq!(v.rows, vec![0, 2]);
    let counts = v.counts_json();
    assert!(counts.contains("\"discarded\":[1]"), "{counts}");
    assert!(
        counts.contains("\"blank\":{\"quantity\":\"area\",\"fold\":2,\"groups\":1,\"dropped\":[[2,30,20,1.5]],\"dropped_total\":1,\"truncated\":false,\"fallback_all\":{\"total\":0,\"files\":[]}}"),
        "{counts}"
    );
    // the plain rule and the same rule written with its default options judge and report alike
    let w = view::build(
        &t,
        &s,
        &Method::None,
        &[Rule::BlankWith(2.0, BlankOpts::default())],
    )
    .unwrap();
    assert_eq!(w.rows, v.rows);
    assert_eq!(w.counts_json(), counts);
}

#[test]
fn a_ratio_under_one_is_reported_as_given() {
    let (t, s) = table(&[ROW0]);
    let v = view::build(&t, &s, &Method::None, &[Rule::Blank(0.5)]).unwrap();
    assert!(
        v.counts_json().contains("\"fold\":0.5,"),
        "{}",
        v.counts_json()
    );
}

#[test]
fn the_csv_and_the_view_table_carry_the_subtracted_heights() {
    use mzlab_feat::export::{csv_full, Ctx};
    let (t, s) = table(&[ROW0]);
    let v = view::build_with(
        &t,
        &s,
        &Method::None,
        &[rule(Quantity::Height, Matching::Batch)],
        labels(),
    )
    .unwrap();
    let c = Ctx {
        table: &t,
        samples: &s,
        view: Some(&v),
        table_sha256: "",
        polarity: 1,
    };
    let csv = csv_full(&c, false);
    let line = csv.lines().nth(1).unwrap();
    // sample A1: area 500 untouched, height 900 - 2 * 100 = 700
    assert!(line.contains(",500,700,DETECTED,"), "{line}");
    let out = mzlab_feat::view_table::table_of(&t, &v);
    assert_eq!(out.height[1], 700.0);
    assert_eq!(out.area[1], 500.0);
}

#[test]
fn the_evidence_is_capped_and_says_how_many_rows_were_dropped() {
    let mut weak = ROW0;
    weak[1] = (100.0, 150.0);
    weak[2] = (90.0, 120.0);
    weak[4] = (100.0, 500.0);
    let rows = vec![weak; 250];
    let (t, s) = table(&rows);
    let v = view::build_with(
        &t,
        &s,
        &Method::None,
        &[rule(Quantity::Height, Matching::Batch)],
        labels(),
    )
    .unwrap();
    assert!(v.rows.is_empty());
    let counts = v.counts_json();
    assert!(
        counts.contains("\"dropped_total\":250,\"truncated\":true"),
        "{counts}"
    );
    // the first 200 rows by row order: ids 1..=200 and no more
    assert_eq!(counts.matches("],[").count(), 199, "{counts}");
    assert!(
        counts.contains("[200,500,300,") && !counts.contains("[201,500,300,"),
        "{counts}"
    );
    // under the cap nothing is cut
    let (t, s) = table(&[weak]);
    let v = view::build_with(
        &t,
        &s,
        &Method::None,
        &[rule(Quantity::Height, Matching::Batch)],
        labels(),
    )
    .unwrap();
    assert!(v
        .counts_json()
        .contains("\"dropped_total\":1,\"truncated\":false"));
}

#[test]
fn the_counts_name_the_files_that_used_all_blanks() {
    let (t, s) = table(&[ROW0]);
    // sample f2 is in batch C that has no blank: it falls back to all blanks; the dark file f5 is cut by all blanks too
    let mut l = labels();
    l.batch[2] = "C".into();
    let v = view::build_with(
        &t,
        &s,
        &Method::None,
        &[rule(Quantity::Height, Matching::Batch)],
        l,
    )
    .unwrap();
    let counts = v.counts_json();
    assert!(counts.contains("\"fallback_all\":{\"total\":1,\"files\":[\"f2.mzML\"]},\"ungrouped_all\":{\"total\":1,\"files\":[\"f5.mzML\"]}"), "{counts}");
    // with all blanks nobody falls back, and without the subtraction the dark file is not listed
    let v = view::build(
        &t,
        &s,
        &Method::None,
        &[rule(Quantity::Height, Matching::All)],
    )
    .unwrap();
    assert!(v
        .counts_json()
        .contains("\"fallback_all\":{\"total\":0,\"files\":[]},\"ungrouped_all\":{\"total\":1,"));
    let plain = Rule::BlankWith(
        2.0,
        BlankOpts {
            quantity: Quantity::Height,
            ..BlankOpts::default()
        },
    );
    let v = view::build(&t, &s, &Method::None, &[plain]).unwrap();
    assert!(!v.counts_json().contains("ungrouped_all"));
}
