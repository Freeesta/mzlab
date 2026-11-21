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

/// Which rows stay (`alive`) and the merges that dropped the others: the rows in m/z order, each against the ones within `MAX_DMZ_DA` above it.
fn same_ions(t: &Aligned, tol: f64) -> (Vec<bool>, Vec<Merge>) {
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
    (alive, log)
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
    let (alive, log) = same_ions(t, r.fill.rt_tol);
    choose(r, "dedup", if log.is_empty() { "off" } else { "on" }, format!("{} rows merged into taller rows", log.len()));
    if !log.is_empty() {
        t.retain_rows(|_, i| alive[i]);
    }
    if let Some(e) = &r.screen.evidence {
        e.with(|l| l.merged.extend(log));
    }
}

#[cfg(test)]
#[path = "lrscreen_dedup_tests.rs"]
mod tests;
