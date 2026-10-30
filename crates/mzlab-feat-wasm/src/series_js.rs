//! The recipe the alignment of a series runs with in the browser (`align_series_to`, `align_series` of `lib.rs`): first the check of `flow run` that the
//! series holds one polarity and one resolution (`mzlab_feat::nature::one_kind_of`), then the recipe of the first index.
#![cfg(feature = "detect")]

use js_sys::Function;
use mzlab_core::error::Result;
use mzlab_feat::recipe::Recipe;

use crate::open_one;
use crate::run::recipe_for;

/// The recipe `text` on the first of the indexes `sizes` (read with `read(k, offset, length)`), or the refusal of a series that mixes polarities or resolutions.
pub fn series_recipe(read: &Function, sizes: &[f64], text: &str) -> Result<Recipe> {
    let indexes = (0..sizes.len())
        .map(|k| open_one(read, k, sizes[k]))
        .collect::<Result<Vec<_>>>()?;
    mzlab_feat::nature::one_kind_of(&indexes)?;
    recipe_for(text, &mut open_one(read, 0, sizes[0])?)
}
