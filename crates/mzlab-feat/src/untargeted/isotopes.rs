//! Isotope grouping of a list of ions, per ion and without removing rows. Rewritten from scratch after the algorithm of mzmine 3.9.0
//! `modules/dataprocessing/filter_isotopegrouper/IsotopeGrouperTask.java` (MIT, (c) 2004-2022 The mzmine Development Team); no code copied.
//! Paper: MZmine 3, Schmid et al., Nat. Biotechnol. 41:447 (2023), doi:10.1038/s41587-023-01690-2.
//!
//! Starting from the most intense free row, each charge 1..`max_charge` is tried: the rows at n x step / z above (and, unless `monotonic`, below) the
//! start, within the m/z and RT tolerances, are the isotopologues; the charge that explains the most rows wins, the lowest on a tie; the rows taken leave the
//! free ones. mzmine removes the isotopologues from the list; here the rows stay and the groups only mark them (`IsoGroups`). Differences kept on purpose:
//! `step` is 1.0033 as in the Java constant (not 1.003355); `monotonic` is the mzmine option of the same name (it only drops the search below the start
//! row; it does not test that heights fall); the mobility and "never remove with MS2" options do not exist.
//!
//! `group_items` works on a plain list (the rows of one file, which is what mzmine groups); `group` takes the rows of an aligned table, whose height is
//! the mean over the files where the ion was detected.

use crate::table::{Aligned, CELL_ABSENT};

/// Distance between isotopologues of mzmine (`isotopeDistance`), Da.
pub const ISOTOPE_STEP: f64 = 1.0033;

/// Which row of a group is its representative.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Representative {
    MostIntense,
    LowestMz,
}

/// Tolerances of the grouping; `Default` has the values of `isotopi.xml` of the golden data (0.0015 Da / 5 ppm, 0.05 min, charge 2, most intense).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct IsoParams {
    pub tol_da: f64,
    pub tol_ppm: f64,
    /// Largest RT distance to the start row, minutes.
    pub rt_tol: f64,
    pub max_charge: u8,
    pub monotonic: bool,
    pub representative: Representative,
    pub step: f64,
}

impl Default for IsoParams {
    fn default() -> Self {
        IsoParams {
            tol_da: 0.0015,
            tol_ppm: 5.0,
            rt_tol: 0.05,
            max_charge: 2,
            monotonic: false,
            representative: Representative::MostIntense,
            step: ISOTOPE_STEP,
        }
    }
}

/// One ion as the grouping sees it.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Item {
    pub mz: f64,
    pub rt: f64,
    pub height: f64,
}

/// A group of at least two rows: `members` includes the representative and is ordered by m/z; `charge` is the one that explained the most rows.
#[derive(Debug, Clone, PartialEq)]
pub struct IsoGroup {
    pub representative: usize,
    pub members: Vec<usize>,
    pub charge: u8,
}

/// The groups of a list, in the order mzmine finds them (most intense start first); rows in no group are alone.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct IsoGroups {
    pub groups: Vec<IsoGroup>,
}

impl IsoGroups {
    /// `(group id, charge)` of each of `n` rows: ids start at 1 in the order of `groups`; (0, 0) for a row alone.
    pub fn per_row(&self, n: usize) -> (Vec<u32>, Vec<i8>) {
        let (mut id, mut z) = (vec![0u32; n], vec![0i8; n]);
        for (g, grp) in self.groups.iter().enumerate() {
            for &m in &grp.members {
                id[m] = g as u32 + 1;
                z[m] = grp.charge as i8;
            }
        }
        (id, z)
    }
}

/// `RTTolerance.checkWithinTolerance`: the range around `a` is built in `float`.
fn rt_within(a: f64, b: f64, tol: f64) -> bool {
    let (a, b, t) = (a as f32, b as f32, tol as f32);
    a - t <= b && b <= a + t
}

/// The free rows (indices into `items`) that sit `n` steps above (`dir` = 1) or below (`dir` = -1) the start row at charge `z`.
fn candidates(
    items: &[Item],
    free: &[bool],
    start: usize,
    z: u8,
    dir: f64,
    n: u32,
    p: &IsoParams,
) -> Vec<usize> {
    let main = items[start];
    let tol = p.tol_da.max(main.mz / 1e6 * p.tol_ppm);
    let shift = p.step * dir * f64::from(n) / f64::from(z);
    (0..items.len())
        .filter(|&i| free[i] && i != start)
        .filter(|&i| {
            (items[i].mz - shift - main.mz).abs() <= tol
                && rt_within(main.rt, items[i].rt, p.rt_tol)
        })
        .collect()
}

/// One half of the pattern: steps are taken one after the other and the search stops at the first step without a candidate. Every candidate of a step
/// is taken (mzmine 2.3 kept only the highest).
fn half(
    items: &[Item],
    free: &[bool],
    start: usize,
    z: u8,
    dir: f64,
    p: &IsoParams,
    fitted: &mut Vec<usize>,
) {
    let mut n = 1;
    loop {
        let found: Vec<usize> = candidates(items, free, start, z, dir, n, p)
            .into_iter()
            .filter(|i| !fitted.contains(i))
            .collect();
        if found.is_empty() {
            return;
        }
        fitted.extend(found);
        n += 1;
    }
}

/// The rows of the pattern around `start` at charge `z`, the start row first.
fn fit(items: &[Item], free: &[bool], start: usize, z: u8, p: &IsoParams) -> Vec<usize> {
    let mut fitted = vec![start];
    if !p.monotonic {
        half(items, free, start, z, -1.0, p, &mut fitted);
    }
    half(items, free, start, z, 1.0, p, &mut fitted);
    fitted
}

/// Groups the ions of a list as the isotope grouper of mzmine 3.9.0 does, without removing any row.
pub fn group_items(items: &[Item], p: &IsoParams) -> IsoGroups {
    let mut order: Vec<usize> = (0..items.len()).collect();
    order.sort_by(|&a, &b| items[b].height.total_cmp(&items[a].height));
    let mut free = vec![true; items.len()];
    let mut out = IsoGroups::default();
    for start in order {
        if !free[start] {
            continue;
        }
        free[start] = false;
        let mut best: Option<(u8, Vec<usize>)> = None;
        for z in 1..=p.max_charge.max(1) {
            let fitted = fit(items, &free, start, z, p);
            if best.as_ref().is_none_or(|(_, b)| fitted.len() > b.len()) {
                best = Some((z, fitted));
            }
        }
        let Some((charge, mut rows)) = best.filter(|(_, r)| r.len() > 1) else {
            continue;
        };
        for &r in &rows {
            free[r] = false;
        }
        let representative = match p.representative {
            Representative::MostIntense => {
                rows.sort_by(|&a, &b| items[b].height.total_cmp(&items[a].height));
                rows[0]
            }
            Representative::LowestMz => *rows
                .iter()
                .min_by(|&&a, &&b| items[a].mz.total_cmp(&items[b].mz))
                .expect("two rows"),
        };
        rows.sort_by(|&a, &b| items[a].mz.total_cmp(&items[b].mz));
        out.groups.push(IsoGroup {
            representative,
            members: rows,
            charge,
        });
    }
    out
}

/// The ions of an aligned table as items: m/z and RT of the row, height = mean over the files with the ion (0 when none).
pub fn items_of(t: &Aligned) -> Vec<Item> {
    (0..t.len())
        .map(|r| {
            let (mut sum, mut n) = (0.0, 0u32);
            for k in 0..t.n_files {
                let c = t.cell(r, k);
                if t.state[c] != CELL_ABSENT {
                    sum += f64::from(t.height[c]);
                    n += 1;
                }
            }
            Item {
                mz: t.mz[r],
                rt: t.rt[r],
                height: if n > 0 { sum / f64::from(n) } else { 0.0 },
            }
        })
        .collect()
}

/// `group_items` on the rows of an aligned table.
pub fn group(t: &Aligned, p: &IsoParams) -> IsoGroups {
    group_items(&items_of(t), p)
}
