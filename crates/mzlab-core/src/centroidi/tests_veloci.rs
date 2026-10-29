//! The faster stages (running valley, windowed centroid) against the plain ones, on made-up scans of 3000-9000 points.

use super::tests::{params, scan, Rng, STEP, W};
use super::*;

/// Runs the stages up to the groups, as `centroids` does.
fn staged(mz: &[f64], y: &[f64], p: &Params) -> (Buf, f64) {
    let mut buf = Buf::default();
    grid::fill(mz, y, p, &mut buf);
    smooth::smooth(&mut buf, p.sg_len());
    smooth::background(&mut buf, p.wb(), p.mode == Mode::Counts);
    let sigma = if p.mode == Mode::Continuous {
        smooth::noise(&mut buf, p.wb())
    } else {
        0.0
    };
    (buf, sigma)
}

#[test]
fn the_faster_stages_give_the_same_groups_and_masses_as_the_plain_ones() {
    let mut rng = Rng(0x9E3779B97F4A7C15);
    for (bins, bg, mode) in [
        (9000, 0.4, Mode::Counts),
        (3000, 3.0, Mode::Counts),
        (9000, 0.05, Mode::Counts),
        (4000, 2.0, Mode::Continuous),
    ] {
        let peaks: Vec<(f64, f64, f64)> = (0..bins / 40)
            .map(|i| {
                // narrow peaks, pairs closer than the width (shoulders) and a few tall ones
                let c = 100.0 + rng.next() * bins as f64 * STEP;
                (
                    c + if i % 3 == 0 { 0.5 } else { 0.0 },
                    5.0 + 300.0 * rng.next().powi(3),
                    if i % 5 == 0 { 0.3 } else { W },
                )
            })
            .collect();
        let (mz, y) = scan(&mut rng, 100.0, bins, &peaks, bg, 1.0);
        let mut p = params(1.0);
        p.mode = mode;
        let (mut buf, sigma) = staged(&mz, &y, &p);
        assert!(buf.sorted);
        let (a, b) = (
            detect::groups(&buf, &p, sigma),
            detect::groups_reference(&buf, &p, sigma),
        );
        assert_eq!(a.len(), b.len());
        for (g, h) in a.iter().zip(&b) {
            assert_eq!(
                (g.apex, g.lo, g.hi, g.members),
                (h.apex, h.lo, h.hi, h.members)
            );
        }
        let windowed: Vec<_> = a
            .iter()
            .map(|g| emit::centroid_at(&buf, &p, buf.x[g.apex], None, 0.5 * p.w).map(f64::to_bits))
            .collect();
        buf.sorted = false;
        let plain: Vec<_> = a
            .iter()
            .map(|g| emit::centroid_at(&buf, &p, buf.x[g.apex], None, 0.5 * p.w).map(f64::to_bits))
            .collect();
        assert_eq!(windowed, plain);
    }
}
