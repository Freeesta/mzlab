//! `dedup` of `features.gapfill` at unit resolution: two rows are one ion when they are within 0.6 Da (never the neighbour nominal mass) and the RT
//! tolerance and either (rule `correlation`) their areas move together across the files (Pearson r of at least 0.9 over at least 3 files where both
//! are present) or (rule `overlap`) in at least 2 files where both were detected (1 for a one-file series) their peaks cover at least `dedup_overlap`
//! (default 0.5) of the narrower one and at least half of that of the wider one: a shoulder or a broad top that the detector cut in two rows is two
//! rows in the same file, which no correlation across files can show. The row with the higher maximum height stays; areas are never summed. Every
//! merge is logged with both rules' numbers; `dedup_overlap=0` turns the second rule off.

use crate::aligned::{Aligned, CELL_DETECTED};
use crate::lrscreen::{choose, Merge, Near, Switch};
use crate::recipe::Recipe;

/// Largest m/z distance of two rows that are one ion.
pub const MAX_DMZ_DA: f64 = 0.6;
/// Smallest correlation of the area vectors.
pub const MIN_R: f64 = 0.9;
/// Files in which both rows must have an area.
pub const MIN_COMMON: usize = 3;
/// Areas that move against each other (r at or below this) show two ions: the `overlap` rule never merges them.
pub const APART_R: f64 = -0.5;
/// Default of `dedup_overlap`: share of the narrower peak the two rows cover together in a file where both were detected.
pub const DEFAULT_OVERLAP: f64 = 0.5;
/// Files where both rows were detected that the `overlap` rule needs (one file only when the series has one): a single file is a coincidence.
pub const MIN_SHARED: usize = 2;
/// The wider peak must be covered by at least this fraction of `dedup_overlap`: a narrow peak inside a very broad one is another ion.
pub const WIDE_FRACTION: f64 = 0.5;

/// What the peaks of two rows share, as medians over the files where both were detected.
#[derive(Clone, Copy)]
struct Cover {
    /// Share of the narrower peak covered by the other.
    narrow: f64,
    /// Share of the wider peak covered by the other.
    wide: f64,
    files: usize,
}

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

/// Over the files where both rows have a detected peak (peak = apex +- width / 2): the median share of the narrower and of the wider peak that the two
/// cover, and the number of those files; `None` when there is none.
fn overlap(t: &Aligned, a: usize, b: usize) -> Option<Cover> {
    let nf = t.n_files;
    let peak = |row: usize, k: usize| {
        let i = row * nf + k;
        let (apex, w) = (f64::from(t.apex_rt[i]), f64::from(t.width[i]));
        (t.state[i] == CELL_DETECTED && apex.is_finite() && w.is_finite() && w > 0.0)
            .then(|| (apex - w / 2.0, apex + w / 2.0))
    };
    let (mut narrow, mut wide): (Vec<f64>, Vec<f64>) = (0..nf)
        .filter_map(|k| {
            let ((l1, h1), (l2, h2)) = (peak(a, k)?, peak(b, k)?);
            let cover = (h1.min(h2) - l1.max(l2)).max(0.0);
            Some((
                cover / (h1 - l1).min(h2 - l2),
                cover / (h1 - l1).max(h2 - l2),
            ))
        })
        .unzip();
    let files = narrow.len();
    (files > 0).then(|| Cover {
        narrow: median(&mut narrow),
        wide: median(&mut wide),
        files,
    })
}

fn median(v: &mut [f64]) -> f64 {
    v.sort_by(f64::total_cmp);
    let n = v.len();
    if n % 2 == 1 {
        v[n / 2]
    } else {
        (v[n / 2 - 1] + v[n / 2]) / 2.0
    }
}

fn tallest(t: &Aligned, row: usize) -> f64 {
    let nf = t.n_files;
    (0..nf)
        .map(|k| f64::from(t.height[row * nf + k]))
        .fold(0.0, f64::max)
}

/// Which rows stay (`alive`), the merges that dropped the others, and the pairs of surviving rows that are still within the window: the rows in m/z
/// order, each against the ones within `MAX_DMZ_DA` above it. `min_overlap` 0 leaves the `overlap` rule off.
fn same_ions(t: &Aligned, tol: f64, min_overlap: f64) -> (Vec<bool>, Vec<Merge>, Vec<Near>) {
    let mut order: Vec<usize> = (0..t.len()).collect();
    mzlab_core::sortc::by(&mut order, |&a, &b| {
        t.mz[a].total_cmp(&t.mz[b]).then(a.cmp(&b))
    });
    let mut alive = vec![true; t.len()];
    let mut log = Vec::new();
    let mut near = Vec::new();
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
            let r = correlation(t, a, b);
            let ov = overlap(t, a, b);
            let by_r = r.is_some_and(|c| c >= MIN_R);
            let by_overlap = min_overlap > 0.0
                && r.is_none_or(|c| c > APART_R)
                && ov.is_some_and(|c| {
                    c.files >= MIN_SHARED.min(t.n_files)
                        && c.narrow >= min_overlap
                        && c.wide >= min_overlap * WIDE_FRACTION
                });
            if !by_r && !by_overlap {
                near.push(Near {
                    a: (t.mz[a], t.rt[a]),
                    b: (t.mz[b], t.rt[b]),
                    dmz_da: dmz,
                    drt_min: drt,
                    r: r.unwrap_or(f64::NAN),
                    overlap: ov.map_or(f64::NAN, |c| c.narrow),
                    overlap_wide: ov.map_or(f64::NAN, |c| c.wide),
                    shared: ov.map_or(0, |c| c.files),
                });
                continue;
            }
            let (keep, drop) = if tallest(t, b) > tallest(t, a) {
                (b, a)
            } else {
                (a, b)
            };
            alive[drop] = false;
            log.push(Merge {
                kept: (t.mz[keep], t.rt[keep]),
                dropped: (t.mz[drop], t.rt[drop]),
                dmz_da: dmz,
                drt_min: drt,
                r: r.unwrap_or(f64::NAN),
                rule: if by_r { "correlation" } else { "overlap" },
                overlap: ov.map_or(f64::NAN, |c| c.narrow),
                overlap_wide: ov.map_or(f64::NAN, |c| c.wide),
                shared: if by_r {
                    common_areas(t, a, b)
                } else {
                    ov.map_or(0, |c| c.files)
                },
                threshold: if by_r { MIN_R } else { min_overlap },
                height: (tallest(t, keep), tallest(t, drop)),
            });
            if drop == a {
                break;
            }
        }
    }
    // A pair noted as near may have lost a member to a later merge: only pairs of rows that stay are left.
    near.retain(|n| {
        let keep =
            |x: (f64, f64)| (0..t.len()).any(|i| alive[i] && t.mz[i] == x.0 && t.rt[i] == x.1);
        keep(n.a) && keep(n.b)
    });
    (alive, log, near)
}

/// Files where both rows have a positive area (what the `correlation` rule counts).
fn common_areas(t: &Aligned, a: usize, b: usize) -> usize {
    let nf = t.n_files;
    let ok = |x: f64| x.is_finite() && x > 0.0;
    (0..nf)
        .filter(|&k| ok(t.area[a * nf + k]) && ok(t.area[b * nf + k]))
        .count()
}

/// Whether the rows of the series can be of one polarity only: the recipe names one, or every file has MS1 scans of the same single sign (the `auto`
/// preset of a positive-only series is that case, and without this check `dedup` would never run on it). A file that cannot be opened says no.
pub(crate) fn one_polarity<R: mzlab_core::idxread::ReadAt>(
    n: usize,
    r: &Recipe,
    open: &mut impl FnMut(usize) -> mzlab_core::error::Result<mzlab_core::idxread::Index<R>>,
) -> bool {
    if r.detect.polarity.is_some() {
        return true;
    }
    if !r.is_lr() || r.screen.dedup == Switch::Off || n == 0 {
        return false;
    }
    let mut seen: Option<i8> = None;
    for k in 0..n {
        let Ok(index) = open(k) else {
            return false;
        };
        let scans = |pol| {
            index
                .section_of(1, Some(pol))
                .map_or(0, |i| index.sections[i].n_scans)
        };
        let sign = match (scans(1), scans(-1)) {
            (n, 0) if n > 0 => 1,
            (0, n) if n > 0 => -1,
            _ => return false,
        };
        if *seen.get_or_insert(sign) != sign {
            return false;
        }
    }
    true
}

/// Drops the rows that are the same ion as a taller one, and logs each merge. Only for the unit axis with one polarity (`one_pol`) and no transitions.
pub(crate) fn merge(t: &mut Aligned, r: &Recipe, one_pol: bool) {
    if !r.is_lr() || r.screen.dedup == Switch::Off || t.is_empty() {
        return;
    }
    if !one_pol || !t.q3.is_empty() {
        choose(
            r,
            "dedup",
            "off",
            "rows of different polarity or transitions are never merged, and the files have them",
        );
        return;
    }
    let min_overlap = r.screen.dedup_overlap.unwrap_or(DEFAULT_OVERLAP);
    let (alive, log, near) = same_ions(t, r.fill.rt_tol, min_overlap);
    choose(
        r,
        "dedup",
        if log.is_empty() { "off" } else { "on" },
        format!(
            "{} rows merged into taller rows (rules: correlation r >= {MIN_R}, or overlap >= {min_overlap} of the narrower and {} of the wider peak in at least {MIN_SHARED} files, unless r <= {APART_R}); {} pairs still within {MAX_DMZ_DA} Da and the RT tolerance",
            log.len(),
            min_overlap * WIDE_FRACTION,
            near.len()
        ),
    );
    if !log.is_empty() {
        t.retain_rows(|_, i| alive[i]);
    }
    if let Some(e) = &r.screen.evidence {
        e.with(|l| {
            l.merged.extend(log);
            l.near.extend(near);
        });
    }
}

#[cfg(test)]
#[path = "lrscreen_dedup_tests.rs"]
mod tests;
