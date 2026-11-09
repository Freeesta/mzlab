//! The recipe the alignment of a series runs with in the browser (`align_series_to`, `align_series` of `lib.rs`): first the check of `flow run` that the
//! series holds one polarity and one resolution (`mzlab_feat::nature::one_kind_of`), then the recipe of the first index; and the weak candidates of a
//! file kept by the page beside its `.mzfeatf`, as the app keeps its `.weak` files.
#![cfg(feature = "detect")]

use js_sys::{Array, Function, Uint8Array};
use mzlab_core::budget::WriteAt;
use mzlab_core::error::{Error, Result};
use mzlab_feat::facts::{file_facts, FileFacts};
use mzlab_feat::recipe::Recipe;
use mzlab_feat::slabs::Stored;
use wasm_bindgen::prelude::*;

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

/// The `.mzfeatf` of a series with the weak candidates the page kept from their detection (`detect_weak_file_to`; `null` or absent for a file
/// without them): the alignment takes a stored table whose recipe key agrees and detects the weak tier again only for the others.
pub(crate) struct WithWeak {
    pub(crate) files: crate::JsStored,
    pub(crate) weak: Option<Array>,
}

impl Stored for WithWeak {
    fn count(&self) -> usize {
        self.files.count()
    }
    fn bytes(&mut self, k: usize) -> Result<Vec<u8>> {
        self.files.bytes(k)
    }
    fn weak(&mut self, k: usize) -> Result<Option<Vec<u8>>> {
        let v = self.weak.as_ref().map(|w| w.get(k as u32));
        Ok(v.and_then(|v| v.dyn_into::<Uint8Array>().ok())
            .map(|b| b.to_vec()))
    }
}

/// The weak candidates of one file (`.mzidx` of `size` bytes read with `read`) with the numbers of `recipe`, written to `write(offset, bytes)` as the
/// app writes them beside the `.mzfeatf` (`mzlab_feat::weakfile`): `{"bytes", "sha256"}`. The alignment of the series then does not detect the file again.
#[wasm_bindgen]
pub fn detect_weak_file_to(
    read: Function,
    size: f64,
    recipe: &str,
    write: Function,
) -> std::result::Result<String, JsValue> {
    let mut ix = crate::open_one(&read, 0, size).map_err(crate::js_err)?;
    let r = recipe_for(recipe, &mut ix).map_err(crate::js_err)?;
    let weak = crate::run::detect_weak(&mut ix, &r).map_err(crate::js_err)?;
    crate::to_sink(write, |sink| sink.write_at(0, &weak))
}
