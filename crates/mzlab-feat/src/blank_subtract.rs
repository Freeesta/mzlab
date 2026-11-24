//! The blank subtraction of a view (`blank_subtract`): the level of every row per comparison group (all blanks, or the blanks of the batch or matrix
//! of each sample), the cell values and heights the view shows, and the evidence behind a row the blank rule dropped.

use mzlab_core::error::Result;

use crate::aligned::Aligned;
use crate::filters_blank::{
    blank_groups, blank_level, value, BlankOpts, Group, Labels, Quantity, Scope,
};
use crate::norm::measured;

/// The blank subtraction of a view: the level of every row per comparison group, and the group of every file (`None` for the blanks, left as they are).
#[derive(Debug, Clone, PartialEq)]
pub struct Subtraction {
    pub opts: BlankOpts,
    pub fold: f64,
    /// `row * n_groups + group`.
    pub level: Vec<f64>,
    pub n_groups: usize,
    pub group: Vec<Option<usize>>,
}

impl Subtraction {
    fn cut(&self, row: usize, file: usize) -> Option<f64> {
        let g = self.group[file]?;
        Some(self.fold * self.level[row * self.n_groups + g])
    }

    /// The cell as the view shows it: the area (normalised) minus `fold` times the blank level, not below 0. With `Height` the area is not touched
    /// (the heights are: `height_cell`).
    pub fn cell(&self, t: &Aligned, factors: &[f64], row: usize, file: usize) -> Option<f64> {
        let a = measured(t, row, file)?;
        let f = factors[file];
        let Some(cut) = self.cut(row, file) else {
            return Some(a * f);
        };
        Some(match self.opts.quantity {
            Quantity::Height => a * f,
            Quantity::NormArea => (a * f - cut).max(0.0),
            Quantity::Area => (a - cut).max(0.0) * f,
        })
    }

    /// The height of the cell as the view shows it: with `Height` the measured height minus `fold` times the blank height of its group, not below 0;
    /// else the measured height.
    pub fn height_cell(&self, t: &Aligned, row: usize, file: usize) -> Option<f64> {
        measured(t, row, file)?;
        let h = f64::from(t.height[row * t.n_files + file]);
        Some(match (self.opts.quantity, self.cut(row, file)) {
            (Quantity::Height, Some(cut)) => (h - cut).max(0.0),
            _ => h,
        })
    }
}

/// The subtraction a list of rules asks for, if any (the first `BlankWith` with `subtract`). The blanks are matched as the rule says (all, batch or
/// matrix); the files that are neither samples nor blanks (dark) use all the blanks.
pub fn subtraction(
    t: &Aligned,
    factors: &[f64],
    rules: &[crate::filters::Rule],
    (samples, labels): (&[crate::samples::Sample], &Labels),
) -> Result<Option<Subtraction>> {
    let Some((fold, o)) = rules.iter().find_map(|r| match r {
        crate::filters::Rule::BlankWith(f, o) if o.subtract => Some((*f, *o)),
        _ => None,
    }) else {
        return Ok(None);
    };
    let (blank, sample) = crate::filters_nts::blank_and_samples(samples);
    let (mut groups, _) = blank_groups(o.matching, labels, &sample, &blank, t.n_files);
    let mut group: Vec<Option<usize>> = vec![None; t.n_files];
    for (i, (s, _)) in groups.iter().enumerate() {
        for &k in s {
            group[k] = Some(i);
        }
    }
    let rest: Vec<usize> = (0..t.n_files)
        .filter(|&k| group[k].is_none() && !blank.contains(&k))
        .collect();
    if !rest.is_empty() {
        let i = groups.len();
        rest.iter().for_each(|&k| group[k] = Some(i));
        groups.push((rest, blank.clone()));
    }
    let n_groups = groups.len().max(1);
    let mut level = Vec::with_capacity(t.len() * n_groups);
    for row in 0..t.len() {
        for (_, b) in &groups {
            level.push(blank_level(t, Some(factors), &o, row, b)?);
        }
        if groups.is_empty() {
            level.push(0.0);
        }
    }
    Ok(Some(Subtraction {
        opts: o,
        fold: fold.max(1.0),
        level,
        n_groups,
        group,
    }))
}

/// Why a row was judged as it was: the cell with the deciding fold (the largest over the cells of the samples, the smallest with `Scope::EachFeature`),
/// its value and the blank level of its group. `None` when no sample cell has a blank level above 0 (then no fold exists).
pub fn evidence(
    t: &Aligned,
    factors: Option<&[f64]>,
    o: &BlankOpts,
    row: usize,
    groups: &[Group],
) -> Result<Option<(f64, f64, f64)>> {
    let mut best: Option<(f64, f64, f64)> = None;
    for (s, b) in groups {
        let level = blank_level(t, factors, o, row, b)?;
        if level <= 0.0 {
            continue;
        }
        for &k in s {
            if let Some(x) = value(t, factors, o.quantity, row, k)? {
                let fold = x / level;
                let better = best.is_none_or(|(_, _, f)| match o.scope {
                    Scope::MaxFeature => fold > f,
                    Scope::EachFeature => fold < f,
                });
                if better {
                    best = Some((x, level, fold));
                }
            }
        }
    }
    Ok(best)
}
