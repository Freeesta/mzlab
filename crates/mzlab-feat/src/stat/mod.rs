//! Statistics of the feature table as steps (`table.stats`, `table.pca`, `table.kmd`): Welch/ANOVA with BH, the exploratory PCA and the Kendrick mass defect.
//! The PCA is `mzlab_core::pca` (the route `api/stat/pca`, already in every .wasm); `pca_json` is its entry for the step.
//! Plain matrices in, `stat/1` text out; nothing here changes the table. The p-values and q-values live only in this output (the admin/HR side): no view
//! of the page shows them. A condition with fewer than `MIN_REPLICHE` files refuses the comparison with a reason and no number. Reference of the PCA:
//! `mzlab/statistica.py`; of the KMD: `mzlab/web/kmd-calc.js`. Docs: `docs/agenti/stat.md`.

mod compare;
mod json;
mod kmd;
mod special;

pub use compare::{
    benjamini_hochberg, compare, pseudocount, Anova, AnovaRow, Comparison, Row, StatsOpts,
    StatsOut, Warning, FLOOR, NOISE_FACTOR,
};
pub use json::{kmd_json, pca_json, stats_json};
pub use kmd::{
    analyse, kendrick, spearman, unit_by_id, units, Ion, KmdOpts, KmdOut, KmdRow, Series, Unit,
};

use crate::samples::Sample;
use mzlab_core::disegno::{Confronto, Disegno};
use mzlab_core::error::{Error, Result};

pub const SCHEMA: &str = "stat/1";

/// Features x files; a missing or non-positive area counts as not detected.
pub type Matrix = [Vec<f64>];

/// Which condition each file belongs to (`None` = not a sample of any condition), which files are blanks and which condition is the reference.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Layout {
    pub names: Vec<String>,
    pub group_of: Vec<Option<usize>>,
    pub blank: Vec<bool>,
    pub reference: Option<usize>,
}

impl Layout {
    /// The files of a condition, in column order.
    pub fn files_of(&self, group: usize) -> Vec<usize> {
        (0..self.group_of.len())
            .filter(|&i| self.group_of[i] == Some(group))
            .collect()
    }
}

/// The layout of a design: conditions by name, sample files by their group, blanks by their type; the reference of the design.
pub fn layout_from_design(samples: &[Sample], d: &Disegno) -> Layout {
    let names: Vec<String> = d.conditions.iter().map(|c| c.name.clone()).collect();
    Layout {
        group_of: samples
            .iter()
            .map(|s| {
                (s.kind == "sample")
                    .then(|| names.iter().position(|n| *n == s.group))
                    .flatten()
            })
            .collect(),
        blank: samples.iter().map(|s| s.kind == "blank").collect(),
        reference: d
            .reference
            .as_ref()
            .and_then(|r| names.iter().position(|n| n == r)),
        names,
    }
}

/// The comparisons of a design as pairs `(a, b)` of conditions (`b` against `a`); "every condition against the reference" needs a reference.
pub fn comparisons_from_design(d: &Disegno, lay: &Layout) -> Result<Vec<(usize, usize)>> {
    let find = |n: &str| {
        lay.names
            .iter()
            .position(|x| x == n)
            .ok_or_else(|| Error::new("error.stats.condition").with("name", n))
    };
    let mut out = Vec::new();
    for c in &d.comparisons {
        match c {
            Confronto::Pair(a, b) => out.push((find(a)?, find(b)?)),
            Confronto::VsReference => {
                let r = lay
                    .reference
                    .ok_or_else(|| Error::new("error.stats.noReference"))?;
                out.extend((0..lay.names.len()).filter(|&g| g != r).map(|g| (r, g)));
            }
        }
    }
    Ok(out)
}
