//! The recipe a lineage means (`rebuild`) and the one an alignment step runs with (`align_recipe`): the panel's recipe on the first index, the gap-filling floor
//! each file estimated on its own, then what the plan writes. Split from `flow_base.rs`.

use std::fs::File;

use mzlab_core::error::{Error, Result};
use mzlab_core::flow::Job;
use mzlab_core::idxread::Index;
use mzlab_feat::recipe::Recipe;
use mzlab_ops::run::feat::run;
use mzlab_ops::run::feat::run::meta::{recipe, Estimate};

use super::flow_indexes::Indexes;
use super::open;
use crate::flow::Lineage;

/// The recipe a lineage means: the one of the panel on its first index, then the parameters of each feature step.
pub(crate) fn rebuild(lin: &Lineage) -> Result<Recipe> {
    let first = lin
        .first
        .as_ref()
        .ok_or_else(|| Error::new("err.flow.lineage"))?;
    let mut r = run::recipe_of_layers(&mut open(first)?, &lin.layers)?;
    run::with_file_floors(
        &mut r,
        lin.index.len(),
        |k| open(&lin.index[k]),
        &lin.layers,
    )?;
    Ok(r)
}

/// The recipe and the estimates of a step of the alignment on `lin`: the one of `first`, the first index of its series, and, when `series` holds the
/// indexes of all its files, the gap-filling floor each file estimated on its own (`run::with_file_floors`), as its detection did.
pub(super) fn align_recipe(
    job: &Job,
    lin: &Lineage,
    first: &mut Index<File>,
    series: Option<&Indexes>,
) -> Result<(Recipe, Vec<Estimate>)> {
    let step = |e: Error| e.with("step", job.step.id);
    let mut base = run::recipe_of_layers(first, &lin.layers).map_err(step)?;
    if let Some(ix) = series {
        run::with_file_floors(&mut base, lin.index.len(), |k| ix.get(k), &lin.layers)
            .map_err(step)?;
    }
    let (mut recipe, estimates) = recipe(job, base)?;
    recipe.fill_gaps = job.step.id == "features.gapfill";
    Ok((recipe, estimates))
}
