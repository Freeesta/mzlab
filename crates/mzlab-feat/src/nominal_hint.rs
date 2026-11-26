//! The signature of nominal-resolution data that the `nominal` profile addresses, measured on one unit-axis file without applying anything: the share of
//! candidate rows between the nominal height and the height in force (low-mass products the recipe would drop) and the share of ions seen on both grids
//! whose centroids lie farther apart than the grid reach in force but within the reach of the profile (one ion in two rows). The numbers and the
//! cuts they are judged by go to `estimates/1` and to the report; the profile stays the user's choice.

use mzlab_core::error::Result;
use mzlab_core::idxread::{Index, ReadAt};

use crate::post::merge_grids;
use crate::recipe::Recipe;
use crate::trace::grid_rows;

/// Default cuts: the hint is given when this share of the candidate rows lies between the two heights, or this share of them is one ion split in two.
pub const CUT_BETWEEN: f64 = 0.1;
pub const CUT_SPLIT: f64 = 0.05;

/// The cuts a hint is judged by (`hint_between`, `hint_split` of `features.estimate`).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Cuts {
    pub between: f64,
    pub split: f64,
}

impl Default for Cuts {
    fn default() -> Cuts {
        Cuts {
            between: CUT_BETWEEN,
            split: CUT_SPLIT,
        }
    }
}

/// What the file shows, with the values it was counted against.
#[derive(Debug, Clone, PartialEq)]
pub struct Signature {
    /// Candidate rows: those the nominal profile keeps after its grid merge.
    pub rows: usize,
    /// Of them, rows under the height in force (`height_now`) and over the nominal one (`height_nominal`).
    pub between: usize,
    /// Ions merged by the reach of the profile (`reach_nominal`) that the reach in force (`reach_now`) leaves in two rows.
    pub split: usize,
    pub height_now: f64,
    pub height_nominal: f64,
    pub reach_now: f64,
    pub reach_nominal: f64,
    pub cuts: Cuts,
}

impl Signature {
    pub fn between_share(&self) -> f64 {
        share(self.between, self.rows)
    }
    pub fn split_share(&self) -> f64 {
        share(self.split, self.rows)
    }
    /// `true` when a share reaches its cut: the file looks like the data the `nominal` profile is for.
    pub fn hint(&self) -> bool {
        self.rows > 0
            && (self.between_share() >= self.cuts.between || self.split_share() >= self.cuts.split)
    }
}

fn share(n: usize, of: usize) -> f64 {
    if of == 0 {
        0.0
    } else {
        n as f64 / of as f64
    }
}

/// The signature of the file `index` under `recipe`: one more detection with the values of the profile, its rows counted before the grids are merged.
/// `None` on a high-resolution recipe, when the profile is already in force or when the file has no MS1 scans of the recipe's polarity.
pub fn signature<R: ReadAt>(
    index: &mut Index<R>,
    recipe: &Recipe,
    cuts: Cuts,
) -> Result<Option<Signature>> {
    if !recipe.detect.axis.is_da() || recipe.preset == Some("nominal") {
        return Ok(None);
    }
    let mut nominal = recipe.clone();
    nominal.nominal();
    let q = &nominal.detect;
    let Some((rows, _)) = grid_rows(index, q, &mut |_, _| {})? else {
        return Ok(None);
    };
    let all: Vec<_> = rows.into_iter().flatten().collect();
    let now = &recipe.detect;
    let keep_nominal = merge_grids(&all, q.axis, q.merge_overlap, q.grid_touch)?;
    let keep_now = merge_grids(&all, q.axis, q.merge_overlap, now.grid_touch)?;
    let kept = keep_nominal.iter().filter(|k| **k).count();
    let split = keep_now.iter().filter(|k| **k).count().saturating_sub(kept);
    let between = all
        .iter()
        .zip(&keep_nominal)
        .filter(|(r, k)| **k && r.height < now.min_height)
        .count();
    Ok(Some(Signature {
        rows: kept,
        between,
        split,
        height_now: now.min_height,
        height_nominal: q.min_height,
        reach_now: now.grid_touch,
        reach_nominal: q.grid_touch,
        cuts,
    }))
}
