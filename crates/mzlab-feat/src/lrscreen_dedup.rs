//! `dedup` of `features.gapfill` at unit resolution: two rows are one ion when they are within 0.6 Da (never the neighbour nominal mass) and the RT
//! tolerance and their areas move together across the files (Pearson r of at least 0.9 over at least 3 files where both are present). The row with
//! the higher maximum height stays; areas are never summed.

use crate::aligned::Aligned;
use crate::lrscreen::{choose, Merge, Switch};
use crate::recipe::Recipe;

/// Largest m/z distance of two rows that are one ion.
pub const MAX_DMZ_DA: f64 = 0.6;
/// Smallest correlation of the area vectors.
pub const MIN_R: f64 = 0.9;
/// Files in which both rows must have an area.
pub const MIN_COMMON: usize = 3;

/// Pearson r of the cells of two rows over the files where both have a positive area; `None` below `MIN_COMMON` files or without variance.
fn correlation(t: &Aligned, a: usize, b: usize) -> Option<f64> {
    let nf = t.n_files;
    let ok = |x: f64| x.is_finite() && x > 0.0;
    let pairs: Vec<(f64, f64)> = (0..nf)
        .map(|k| (t.area[a * nf + k], t.area[b * nf + k]))
        .filter(|&(x, y)| ok(x) && ok(y))
        .collect();
    if pairs.len() < MIN_COMMON {
        return None;
    }
    let n = pairs.len() as f64;
    let (mx, my) = (
        pairs.iter().map(|p| p.0).sum::<f64>() / n,
        pairs.iter().map(|p| p.1).sum::<f64>() / n,
    );
    let (mut sxy, mut sxx, mut syy) = (0.0, 0.0, 0.0);
    for (x, y) in pairs {
        sxy += (x - mx) * (y - my);
        sxx += (x - mx).powi(2);
        syy += (y - my).powi(2);
    }
    (sxx > 0.0 && syy > 0.0).then(|| sxy / (sxx * syy).sqrt())
}

fn tallest(t: &Aligned, row: usize) -> f64 {
    let nf = t.n_files;
    (0..nf).map(|k| f64::from(t.height[row * nf + k])).fold(0.0, f64::max)
}

/// Drops the rows that are the same ion as a taller one, and logs each merge. Only for the unit axis with one polarity and no transitions.
pub(crate) fn merge(t: &mut Aligned, r: &Recipe) {
    if !r.is_lr() || r.screen.dedup == Switch::Off || t.is_empty() {
        return;
    }
    if r.detect.polarity.is_none() || !t.q3.is_empty() {
        choose(r, "dedup", "off", "rows of different polarity or transitions are never merged, and the table has them");
        return;
    }
    let tol = r.fill.rt_tol;
    let mut order: Vec<usize> = (0..t.len()).collect();
    mzlab_core::sortc::by(&mut order, |&a, &b| t.mz[a].total_cmp(&t.mz[b]).then(a.cmp(&b)));
    let mut alive = vec![true; t.len()];
    let mut log = Vec::new();
    for (p, &a) in order.iter().enumerate() {
        for &b in &order[p + 1..] {
            let dmz = t.mz[b] - t.mz[a];
            if dmz > MAX_DMZ_DA {
                break;
            }
            let drt = (t.rt[b] - t.rt[a]).abs();
            if !alive[a] || !alive[b] || drt > tol {
                continue;
            }
            let Some(c) = correlation(t, a, b).filter(|&c| c >= MIN_R) else {
                continue;
            };
            let (keep, drop) = if tallest(t, b) > tallest(t, a) { (b, a) } else { (a, b) };
            alive[drop] = false;
            log.push(Merge {
                kept: (t.mz[keep], t.rt[keep]),
                dropped: (t.mz[drop], t.rt[drop]),
                dmz_da: dmz,
                drt_min: drt,
                r: c,
            });
            if drop == a {
                break;
            }
        }
    }
    choose(r, "dedup", if log.is_empty() { "off" } else { "on" }, format!("{} rows merged into taller rows", log.len()));
    if !log.is_empty() {
        t.retain_rows(|_, i| alive[i]);
    }
    if let Some(e) = &r.screen.evidence {
        e.with(|l| l.merged.extend(log));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A table of `rows` = (m/z, RT, areas over 4 files), height equal to the area.
    fn table(rows: &[(f64, f64, [f64; 4])]) -> Aligned {
        let mut t = Aligned { n_files: 4, ..Aligned::default() };
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
        let mut t = table(&[(200.0, 5.0, [1e4, 2e4, 4e4, 8e4]), (200.4, 5.02, [5e3, 1.1e4, 2e4, 3.9e4])]);
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
        let rows = [(200.0, 5.0, [1e4, 2e4, 4e4, 8e4]), (200.4, 5.0, [1e4, 2e4, 4e4, 8e4])];
        let mut r = recipe();
        r.screen.dedup = Switch::Off;
        let mut t = table(&rows);
        merge(&mut t, &r);
        assert_eq!(t.len(), 2);
        let mut t = table(&rows);
        merge(&mut t, &Recipe::default());
        assert_eq!(t.len(), 2);
    }
}
