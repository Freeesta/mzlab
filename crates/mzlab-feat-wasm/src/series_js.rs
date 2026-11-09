//! The recipe the alignment of a series runs with in the browser (`align_series_to`, `align_series` of `lib.rs`): first the check of `flow run` that the
//! series holds one polarity and one resolution (`mzlab_feat::nature::one_kind_of`), then the recipe of the first index; and the weak candidates of a
//! file kept by the page beside its `.mzfeatf`, as the app keeps its `.weak` files.
#![cfg(feature = "detect")]

use js_sys::{Array, Function, Uint8Array};
use mzlab_core::budget::{Hashing, MemBudget, WriteAt};
use mzlab_core::error::{Error, Result};
use mzlab_feat::facts::{file_facts, FileFacts};
use mzlab_feat::recipe::Recipe;
use mzlab_feat::slabs::Stored;
use wasm_bindgen::prelude::*;

use crate::run::recipe_for;
use crate::{
    blocks_of, js_err, open_all, open_one, progress_js, run, spill_js, to_sink, JsSink, JsStored,
};

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

/// L4-L6: alignment and gap filling of the `.mzfeatf` of a series (an `Array` of `Uint8Array`, in the order of the indexes) written to
/// `write(offset, bytes)`: `{"bytes", "sha256"}` of the `.mzfeat`. The same bytes whatever `budget_mb` (0/NaN = 192): in memory, in slabs of m/z, or
/// the error `error.memoryBudget` with `{fase, stima_mb, budget_mb}`. The optional `spill` keeps the weak tables outside the module (`spill_js`); the
/// optional `weak` holds the weak candidates of each file from its detection (`series_js::detect_weak_file_to`), the same bytes with or without them;
/// the optional `progress(done, total)` is called at most once per 1 % (slabs, weak tier, gap filling), the same bytes with or without it.
#[wasm_bindgen]
#[allow(clippy::too_many_arguments)]
pub fn align_series_to(
    files: Array,
    read: Function,
    sizes: Vec<f64>,
    recipe: &str,
    write: Function,
    budget_mb: f64,
    spill: Option<js_sys::Object>,
    weak: Option<Array>,
    progress: Option<Function>,
) -> std::result::Result<String, JsValue> {
    let spill = spill_js::of(spill.as_ref())?;
    to_sink(write, |sink| {
        series_into(
            sink, files, &read, &sizes, recipe, budget_mb, spill, weak, progress,
        )
    })
}

/// The steps of `align_series_to` into `sink` (one copy of the alignment in the module for both exports).
#[allow(clippy::too_many_arguments)]
fn series_into(
    sink: &mut Hashing<JsSink>,
    files: Array,
    read: &Function,
    sizes: &[f64],
    recipe: &str,
    budget_mb: f64,
    mut spill: Option<Box<dyn mzlab_feat::spill::Spill>>,
    weak: Option<Array>,
    progress: Option<Function>,
) -> Result<()> {
    let (r, facts) = series_recipe(read, sizes, recipe)?;
    let budget = MemBudget::from_request(budget_mb);
    let mut src = WithWeak {
        files: JsStored { files },
        weak,
    };
    let open = |k| open_one(read, k, sizes[k]);
    let mut tell = progress_js::calling(progress);
    let mut pace = mzlab_core::pace::Pace::new(&mut tell);
    // the gap filling reads no scan table (`series_recipe` read them): where there are threads its opener must be `Sync`, so the native build of
    // this crate (its tests) opens the files whole
    #[cfg(target_family = "wasm")]
    let lean = |k: usize| {
        mzlab_core::idxread::Index::open_lean(blocks_of(read, k, sizes[k], 1), sizes[k] as u64)
    };
    #[cfg(target_family = "wasm")]
    let fill: Option<&mzlab_feat::series::FillOpen<_>> = Some(&lean);
    #[cfg(not(target_family = "wasm"))]
    let fill = None;
    run::align_to_spilling(
        &mut src,
        &budget,
        &r,
        Some(facts),
        open,
        sink,
        spill_js::borrow(&mut spill),
        fill,
        &mut |d, t| pace.at(d, t),
    )
}

/// L4-L6 with the bytes of the `.mzfeat` returned (they are in memory as a whole: for small series and tests; the page uses `align_series_to`).
#[wasm_bindgen]
pub fn align_series(
    files: Array,
    read: Function,
    sizes: Vec<f64>,
    recipe: &str,
) -> std::result::Result<Vec<u8>, JsValue> {
    let mut sink = Hashing::new(JsSink {
        write: None,
        held: Vec::new(),
    });
    series_into(
        &mut sink,
        files,
        &read,
        &sizes,
        recipe,
        f64::NAN,
        None,
        None,
        None,
    )
    .map_err(js_err)?;
    Ok(sink.finish().0.held)
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
    fn part(
        &mut self,
        k: usize,
    ) -> Result<Option<(Box<dyn mzlab_core::idxread::ReadAt + '_>, u64)>> {
        self.files.part(k)
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
