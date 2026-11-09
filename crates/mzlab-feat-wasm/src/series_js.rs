//! The recipe the alignment of a series runs with in the browser (`align_series_to`, `align_series` of `lib.rs`): first the check of `flow run` that the
//! series holds one polarity and one resolution (`mzlab_feat::nature::one_kind_of`), then the recipe of the first index.
#![cfg(feature = "detect")]

use js_sys::Function;
use mzlab_core::error::{Error, Result};
use mzlab_feat::facts::{file_facts, FileFacts};
use mzlab_feat::recipe::Recipe;

use crate::run::recipe_for;
use crate::{blocks_of, open_all};

/// The recipe `text` on the first of the indexes `sizes` (read with `read(k, offset, length)`) and the facts of every file, or the refusal of a
/// series that mixes polarities or resolutions. Each scan table is parsed once: the first index is taken again over a reader of its own (the block
/// budget of a file open alone, as the recipe was always estimated) without parsing it a second time.
pub fn series_recipe(
    read: &Function,
    sizes: &[f64],
    text: &str,
) -> Result<(Recipe, Vec<FileFacts>)> {
    let indexes = open_all(read, sizes)?;
    mzlab_feat::nature::one_kind_of(&indexes)?;
    let facts = indexes.iter().map(file_facts).collect();
    let first = indexes
        .first()
        .ok_or_else(|| Error::new("error.badIndex"))?;
    let mut first = first.over(blocks_of(read, 0, sizes[0], 1));
    drop(indexes);
    Ok((recipe_for(text, &mut first)?, facts))
}
