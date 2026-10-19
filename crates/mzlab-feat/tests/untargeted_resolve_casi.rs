//! Hand-made cases of `untargeted::resolve` (the rules of the local-minimum resolver of mzmine 3.9.0, one at a time). The runs are long and sparse on purpose:
//! with more than 85 % of zeros the chromatographic threshold is 0 and does not touch the peaks (a trace with more points than that loses its lowest ones:
//! `dense_trace_keeps_only_what_stands_out`).
#![cfg(feature = "untargeted")]

use mzlab_feat::untargeted::resolve::{quantile, resolve, Chrom, MinParams};

const STEP: f64 = 0.01;
const N: usize = 1200;

fn axis(n: usize) -> Vec<f64> {
    (0..n).map(|i| 0.2 + i as f64 * STEP).collect()
}

fn chrom(y: Vec<f64>) -> Chrom {
    let mz = y
        .iter()
        .map(|&v| if v > 0.0 { 300.0 } else { 0.0 })
        .collect();
    Chrom { mz, y }
}

/// Run of N scans with a polyline of `(scan, intensity)` knots (linear in between, 0 outside the first and the last).
fn polyline(knots: &[(usize, f64)]) -> Vec<f64> {
    let mut y = vec![0.0; N];
    for w in knots.windows(2) {
        let ((a, ya), (b, yb)) = (w[0], w[1]);
        for (i, v) in y.iter_mut().enumerate().take(b + 1).skip(a) {
            *v = ya + (yb - ya) * (i - a) as f64 / (b - a) as f64;
        }
    }
    y
}

/// Two peaks of apex 3e5 with the valley at `valley` times the apex; edges at 5 % of the apex.
fn pair(valley: f64) -> Vec<f64> {
    polyline(&[
        (100, 1.5e4),
        (115, 3e5),
        (130, 3e5 * valley),
        (145, 3e5),
        (160, 1.5e4),
    ])
}

#[test]
fn valley_at_40_percent_splits_and_at_70_does_not() {
    let rt = axis(N);
    let p = MinParams::default();
    let split = resolve(&chrom(pair(0.4)), &rt, &p);
    assert_eq!(split.len(), 2, "apex/valley 2.5 >= 1.7");
    assert_eq!(
        (split[0].scans, split[1].scans),
        ((100, 130), (130, 160)),
        "the valley belongs to both regions"
    );
    let one = resolve(&chrom(pair(0.7)), &rt, &p);
    assert_eq!(one.len(), 1, "apex/valley 1.43 < 1.7");
    assert_eq!(one[0].scans, (100, 160));
    let loose = MinParams {
        min_ratio: 1.3,
        ..p
    };
    assert_eq!(
        resolve(&chrom(pair(0.7)), &rt, &loose).len(),
        2,
        "with min_ratio 1.3 the 70 % valley splits"
    );
}

#[test]
fn single_zero_closes_a_region() {
    let rt = axis(N);
    let p = MinParams::default();
    // a missing scan on the low rising flank: the points before it are too few, the rest is a peak whose left edge is the point after the zero
    let mut y = polyline(&[(100, 3e4), (115, 3e5), (130, 3e4)]);
    y[103] = 0.0;
    let f = resolve(&chrom(y), &rt, &p);
    assert_eq!(
        f.iter().map(|f| f.scans).collect::<Vec<_>>(),
        vec![(104, 130)]
    );
    // a missing scan at the apex: both halves end at a high point (apex < 1.7 x edge), so nothing is left
    let mut y = polyline(&[(100, 3e4), (115, 3e5), (130, 3e4)]);
    y[115] = 0.0;
    assert!(resolve(&chrom(y), &rt, &p).is_empty());
}

#[test]
fn peak_longer_than_one_minute_is_dropped() {
    let rt = axis(N);
    let p = MinParams::default();
    let long = polyline(&[(100, 3e4), (170, 3e5), (240, 3e4)]); // 1.40 min
    assert!(resolve(&chrom(long), &rt, &p).is_empty());
    let ok = polyline(&[(100, 3e4), (145, 3e5), (190, 3e4)]); // 0.90 min
    assert_eq!(resolve(&chrom(ok), &rt, &p).len(), 1);
}

#[test]
fn shape_filters() {
    let rt = axis(N);
    let p = MinParams::default();
    let few = polyline(&[(100, 3e4), (102, 3e5), (103, 3e4)]); // 4 points
    assert!(resolve(&chrom(few), &rt, &p).is_empty(), "under min_points");
    let low = polyline(&[(100, 5e3), (110, 2.9e4), (120, 5e3)]);
    assert!(
        resolve(&chrom(low), &rt, &p).is_empty(),
        "under the absolute height"
    );
    let high_edge = polyline(&[(100, 2e5), (110, 3e5), (120, 2e5)]);
    assert!(
        resolve(&chrom(high_edge), &rt, &p).is_empty(),
        "apex under 1.7 times the edge"
    );
    let rel = MinParams {
        min_rel_height: 0.5,
        ..p.clone()
    };
    let two = polyline(&[(100, 3e4), (110, 4e5), (120, 3e4)]);
    let mut y = two;
    for (i, v) in polyline(&[(300, 3e4), (310, 1.5e5), (320, 3e4)])
        .into_iter()
        .enumerate()
    {
        y[i] += v;
    }
    assert_eq!(resolve(&chrom(y.clone()), &rt, &p).len(), 2);
    assert_eq!(
        resolve(&chrom(y), &rt, &rel).len(),
        1,
        "the small peak is under half of the highest point"
    );
}

#[test]
fn dense_trace_keeps_only_what_stands_out() {
    // 80 scans at 6e4 and 20 scans of a peak over them: the quantile 0.85 falls inside the peak, so the baseline and its foot go to 0
    let mut y = vec![6e4; 100];
    for (k, v) in y[40..60].iter_mut().enumerate() {
        *v = 1e5 + 5e5 * (1.0 - ((k as f64 - 9.5) / 10.0).abs());
    }
    let f = resolve(&chrom(y), &axis(100), &MinParams::default());
    assert_eq!(f.len(), 1);
    assert!(f[0].scans.0 > 40 && f[0].scans.1 < 59, "{:?}", f[0].scans);
    assert!(f[0].height > 5.4e5);
}

#[test]
fn height_area_mz_and_apex() {
    // rt every 0.05 min (inside the search range, so a falling flank has lower points to its right); the apex is two equal points: the first is the apex
    let rt: Vec<f64> = (0..100).map(|i| i as f64 * 0.05).collect();
    let mut y = vec![0.0; 100];
    y[3..9].copy_from_slice(&[10.0, 40.0, 40.0, 30.0, 20.0, 10.0]);
    let mut mz = vec![0.0; 100];
    mz[3..9].copy_from_slice(&[100.0, 100.2, 100.4, 100.6, 100.8, 101.0]);
    let p = MinParams {
        min_abs_height: 0.0,
        min_points: 3,
        duration: (0.0, 10.0),
        ..MinParams::default()
    };
    let f = resolve(
        &Chrom {
            mz: mz.clone(),
            y: y.clone(),
        },
        &rt,
        &p,
    );
    assert_eq!(f.len(), 1);
    let f = &f[0];
    assert_eq!(
        (f.scans, f.rt_start, f.rt_end, f.rt_apex, f.height),
        ((3, 8), rt[3], rt[8], rt[4], 40.0)
    );
    let area = 0.05
        * ((10.0 + 40.0) / 2.0
            + 40.0
            + (40.0 + 30.0) / 2.0
            + (30.0 + 20.0) / 2.0
            + (20.0 + 10.0) / 2.0);
    assert!((f.area - area).abs() < 1e-12, "{} vs {area}", f.area);
    assert_eq!(f.area_s, f.area * 60.0);
    let want =
        (100.0 * 10.0 + 100.2 * 40.0 + 100.4 * 40.0 + 100.6 * 30.0 + 100.8 * 20.0 + 101.0 * 10.0)
            / 150.0;
    assert!(
        (f.mz - want).abs() < 1e-12,
        "m/z weighted by intensity: {} vs {want}",
        f.mz
    );
}

#[test]
fn quantile_like_calc_quantile() {
    assert_eq!(quantile(&[], 0.85), 0.0);
    assert_eq!(quantile(&[7.0], 0.3), 7.0);
    assert_eq!(quantile(&[4.0, 1.0, 3.0, 2.0], 0.5), 2.5); // positions 1 and 2 of the sorted vector
    assert_eq!(quantile(&[0.0, 0.0, 0.0, 0.0, 10.0], 0.85), 5.0); // 3.4 -> positions 3 and 4, mean of 0 and 10
    assert_eq!(quantile(&[3.0, 1.0, 2.0, 9.0, 0.0], 0.25), 1.0); // exact position 1
    assert_eq!(quantile(&[3.0, 1.0, 2.0], 1.5), 3.0); // q clamped to 1
}

#[test]
fn bad_input_gives_nothing() {
    let p = MinParams::default();
    assert!(resolve(&chrom(vec![0.0; N]), &axis(N), &p).is_empty());
    assert!(
        resolve(&chrom(vec![1e5; 10]), &axis(9), &p).is_empty(),
        "length mismatch"
    );
    assert!(resolve(&Chrom::default(), &[], &p).is_empty());
}

#[test]
fn adjust_edges_takes_the_zero_next_to_an_edge_in() {
    // mzmine 4.10 `adjustStartAndEnd`: an edge point that is not 0 with a 0 next to it takes the 0 in, one scan per side
    let y = polyline(&[(100, 1.5e4), (115, 3e5), (130, 1.5e4)]);
    let off = resolve(&chrom(y.clone()), &axis(N), &MinParams::default());
    let on = resolve(
        &chrom(y),
        &axis(N),
        &MinParams {
            adjust_edges: true,
            ..MinParams::default()
        },
    );
    assert_eq!((off.len(), on.len()), (1, 1));
    assert_eq!(off[0].scans, (100, 130));
    assert_eq!(on[0].scans, (99, 131));
    assert!((on[0].rt_start - (off[0].rt_start - STEP)).abs() < 1e-12);
    // the zeros add no height and no area (the trapezoids to them are of the triangles 0 -> edge)
    assert_eq!(on[0].height, off[0].height);
    assert!(on[0].area > off[0].area);
}

#[test]
fn adjust_edges_stops_at_the_ends_of_the_run() {
    // a peak that starts at the first scan (or ends at the last one) has no zero to take in on that side
    let mut y = vec![0.0; 100];
    for (k, v) in y.iter_mut().enumerate().take(12) {
        *v = 3e4 + 4e4 * (6 - (k as i32 - 6).abs()) as f64;
    }
    let on = MinParams {
        adjust_edges: true,
        ..MinParams::default()
    };
    let first = resolve(&chrom(y.clone()), &axis(100), &on);
    assert_eq!(first.len(), 1);
    assert_eq!(first[0].scans, (0, 12));
    y.reverse();
    let last = resolve(&chrom(y), &axis(100), &on);
    assert_eq!(last.len(), 1);
    assert_eq!(last[0].scans, (87, 99));
}
