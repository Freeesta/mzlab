//! `gruppi/1`: shapes over time (rising, bell, decay, flat, single), the interpolated half-rise time, the relative rate and the decay of the
//! parent per condition, on three conditions x five times x two replicates (synthetic).

mod gruppi_comune;

use gruppi_comune::*;
use mzlab_feat::gruppi::{build, GroupOpts, GroupsOut};

const TIMES: [f64; 5] = [10.0, 20.0, 30.0, 40.0, 50.0];
const CONDS: [&str; 3] = ["A", "B", "C"];

/// Files: a shared time zero, then for each condition five times in duplicate. `course[c]` is the mean area of condition `c` at each time.
fn kinetic(rows: Vec<(&str, bool, [[f64; 5]; 3])>, opts: GroupOpts) -> GroupsOut {
    let mut v = vec![file("t0.mzML", "t0", "", Some(0.0), 1)];
    for c in CONDS {
        for t in TIMES {
            for r in 0..2 {
                let n = v.len() as u32 + 1;
                v.push(file(&format!("{c}_{t}_{r}.mzML"), "sample", c, Some(t), n));
            }
        }
    }
    let cs = files(v);
    let table_rows = rows
        .into_iter()
        .enumerate()
        .map(|(i, (name, parent, course))| {
            let mut m = meta(i as u32, name, 300.0 + i as f64);
            m.is_parent = parent;
            let mut cells = vec![if parent { d(1e7) } else { x() }];
            for c in course {
                for a in c {
                    cells.push(d(a));
                    cells.push(d(a));
                }
            }
            (m, cells)
        })
        .collect();
    build(&table(table_rows, cs.len()), &cs, &opts).unwrap()
}

fn cond<'a>(g: &'a GroupsOut, row: usize, c: &str) -> &'a mzlab_feat::gruppi::CondRow {
    g.rows[row].cond.iter().find(|x| x.id == c).unwrap()
}

fn opts() -> GroupOpts {
    GroupOpts {
        reference: Some("A".into()),
        ..GroupOpts::default()
    }
}

fn scale(v: [f64; 5], k: f64) -> [f64; 5] {
    v.map(|x| x * k * 1e5)
}

#[test]
fn shapes_and_half_rise() {
    let rising = [
        scale([1.0, 2.0, 3.0, 4.0, 5.0], 1.0),
        scale([3.0, 4.0, 5.0, 5.5, 6.0], 1.0),
        scale([0.2, 0.4, 0.6, 1.0, 5.0], 1.0),
    ];
    let bell = [scale([1.0, 3.0, 5.0, 3.4, 3.0], 1.0); 3];
    let decay = [scale([5.0, 4.0, 3.0, 2.0, 1.0], 1.0); 3];
    let flat = [scale([3.0, 3.2, 3.1, 3.3, 3.0], 1.0); 3];
    let g = kinetic(
        vec![
            ("rise", false, rising),
            ("bell", false, bell),
            ("decay", false, decay),
            ("flat", false, flat),
        ],
        opts(),
    );
    let shape = |r: usize, c: &str| cond(&g, r, c).shape;
    assert_eq!(shape(0, "A"), Some("rising"));
    assert_eq!(
        shape(1, "A"),
        Some("bell"),
        "fall of 40 % after the maximum"
    );
    assert_eq!(shape(2, "A"), Some("decay"));
    assert_eq!(shape(3, "A"), Some("flat"));
    // The times: the maximum, and the half of it interpolated between two points.
    assert_eq!(cond(&g, 0, "A").t_max, Some(50.0));
    assert_eq!(cond(&g, 1, "A").t_max, Some(30.0));
    assert_eq!(cond(&g, 0, "A").t_half_rise, Some(25.0));
    assert_eq!(cond(&g, 0, "B").t_half_rise, Some(10.0));
    assert!((cond(&g, 0, "C").t_half_rise.unwrap() - 43.75).abs() < 1e-9);
    assert_eq!(cond(&g, 2, "A").t_half_rise, None, "a decay does not rise");
    // Relative rate: > 1 for the faster condition, 1 for the reference, < 1 for the slower.
    assert_eq!(cond(&g, 0, "A").rate_rel, Some(1.0));
    assert_eq!(cond(&g, 0, "B").rate_rel, Some(2.5));
    assert!(cond(&g, 0, "C").rate_rel.unwrap() < 1.0);
    // The points: five times, two replicates each, the mean and the sd of the replicates (identical here).
    let k = &cond(&g, 0, "B").kin;
    assert_eq!((k.len(), k[0].n, k[0].t), (5, 2, 10.0));
    assert_eq!(k[2].area, 5e5);
    assert_eq!(k[2].sd, Some(0.0));
    // The ratio is taken at the highest common time (50 min).
    assert_eq!(cond(&g, 0, "B").ratio_ref, Some(1.2));
}

#[test]
fn a_plateau_after_the_maximum_is_not_a_bell() {
    let plateau = [scale([1.0, 3.0, 5.0, 4.5, 4.4], 1.0); 3];
    let g = kinetic(vec![("plateau", false, plateau)], opts());
    assert_eq!(cond(&g, 0, "A").shape, Some("rising"));
    let strict = GroupOpts {
        shape_drop: 0.1,
        ..opts()
    };
    let g = kinetic(vec![("plateau", false, plateau)], strict);
    assert_eq!(cond(&g, 0, "A").shape, Some("bell"));
}

#[test]
fn single_time_and_two_times() {
    let cs = files(vec![
        file("t0.mzML", "t0", "", Some(0.0), 1),
        file("A.mzML", "sample", "A", Some(60.0), 2),
        file("B_1.mzML", "sample", "B", Some(30.0), 3),
        file("B_2.mzML", "sample", "B", Some(60.0), 4),
    ]);
    let rows = vec![(meta(0, "TP", 300.0), vec![x(), d(1e5), d(1e5), d(2e5)])];
    let g = build(&table(rows, 4), &cs, &GroupOpts::default()).unwrap();
    assert_eq!(cond(&g, 0, "A").shape, Some("single"));
    assert_eq!(cond(&g, 0, "A").t_half_rise, None);
    assert_eq!(
        cond(&g, 0, "B").shape,
        Some("rising"),
        "two times: rising, decay or flat, never bell"
    );
    assert_eq!(
        g.warnings.iter().filter(|w| w.code == "mixedTimes").count(),
        1
    );
    assert!(g
        .warnings
        .iter()
        .any(|w| w.code == "condOnePoint" && w.condition == "A"));
    // A has 60, B has 30 and 60: they share 60.
    let o = GroupOpts {
        reference: Some("A".into()),
        ..GroupOpts::default()
    };
    let rows = vec![(meta(0, "TP", 300.0), vec![x(), d(1e5), d(1e5), d(2e5)])];
    let g = build(&table(rows, 4), &cs, &o).unwrap();
    assert_eq!(cond(&g, 0, "B").ratio_ref, Some(2.0));
}

#[test]
fn no_common_time_gives_no_ratio() {
    let cs = files(vec![
        file("A.mzML", "sample", "A", Some(60.0), 1),
        file("B.mzML", "sample", "B", Some(30.0), 2),
    ]);
    let rows = vec![(meta(0, "TP", 300.0), vec![d(1e5), d(2e5)])];
    let o = GroupOpts {
        reference: Some("A".into()),
        ..GroupOpts::default()
    };
    let g = build(&table(rows, 2), &cs, &o).unwrap();
    assert_eq!(cond(&g, 0, "B").ratio_ref, None);
    assert!(g
        .warnings
        .iter()
        .any(|w| w.code == "mixedTimes" && w.condition == "B"));
    assert!(g
        .warnings
        .iter()
        .any(|w| w.code == "condNoControls" && w.condition == "A"));
}

#[test]
fn parent_decay_per_condition() {
    let decay = |k: f64| TIMES.map(|t| 1e7 * (-k * t).exp());
    let g = kinetic(
        vec![("Parent", true, [decay(0.03), decay(0.06), decay(0.015)])],
        opts(),
    );
    let p = |c: &str| {
        g.conditions
            .iter()
            .find(|x| x.id == c)
            .unwrap()
            .parent
            .clone()
    };
    assert!((p("A").k.unwrap() - 0.03).abs() < 1e-5);
    assert!((p("B").k.unwrap() - 0.06).abs() < 1e-5);
    assert!((p("B").half_life.unwrap() - 2f64.ln() / 0.06).abs() < 1e-4);
    // The fraction left at 50 minutes against the time zero of the condition (1e7).
    assert!((p("A").remaining.unwrap() - (-1.5f64).exp()).abs() < 1e-5);
    assert!(p("C").k.unwrap() < p("A").k.unwrap());
}

#[test]
fn two_times_are_rising_decay_or_flat() {
    let cs = files(vec![
        file("t0.mzML", "t0", "", Some(0.0), 1),
        file("A_1.mzML", "sample", "A", Some(30.0), 2),
        file("A_2.mzML", "sample", "A", Some(60.0), 3),
    ]);
    let shape = |a: f64, b: f64| {
        let rows = vec![(meta(0, "TP", 300.0), vec![x(), d(a), d(b)])];
        let g = build(&table(rows, 3), &cs, &GroupOpts::default()).unwrap();
        cond(&g, 0, "A").shape
    };
    assert_eq!(shape(1e5, 2e5), Some("rising"));
    assert_eq!(shape(2e5, 1e5), Some("decay"));
    assert_eq!(shape(1e5, 1.4e5), Some("flat"));
    assert_eq!(shape(1e5, 1.5e5), Some("flat"));
}

#[test]
fn ratio_is_null_when_the_reference_area_is_zero() {
    let cs = files(vec![
        file("t0.mzML", "t0", "", Some(0.0), 1),
        file("R.mzML", "sample", "R", Some(60.0), 2),
        file("A.mzML", "sample", "A", Some(60.0), 3),
    ]);
    let rows = vec![(meta(0, "TP", 300.0), vec![x(), d(0.0), d(2e5)])];
    let o = GroupOpts {
        reference: Some("R".into()),
        ..GroupOpts::default()
    };
    let g = build(&table(rows, 3), &cs, &o).unwrap();
    for c in ["R", "A"] {
        let r = cond(&g, 0, c).ratio_ref;
        assert!(r.is_none_or(f64::is_finite), "never Inf or NaN");
    }
    assert_eq!(cond(&g, 0, "A").ratio_ref, None);
}
