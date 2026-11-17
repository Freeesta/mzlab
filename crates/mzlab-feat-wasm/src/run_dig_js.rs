//! JS exports of `run_dig`; the plain functions are `mzlab_ops::run::feat::run_dig`.

use crate::run;
use crate::run_tp::dig::dig_inputs;
use crate::run_tp::dig::DigInputs;
use crate::run_tp::{parent_formula, read_parent, LinkParent};
use mzlab_core::error::Result;

/// What `tp_dig_inputs` gives the page, to hand to `finder_lr_dig` (the .wasm `finder-lr`).
#[cfg(feature = "proofs")]
#[wasm_bindgen::prelude::wasm_bindgen]
pub struct DigPrepared {
    inner: DigInputs,
}

#[cfg(feature = "proofs")]
#[wasm_bindgen::prelude::wasm_bindgen]
impl DigPrepared {
    pub fn files(&self) -> String {
        self.inner.files.clone()
    }
    pub fn who(&self) -> String {
        self.inner.who.clone()
    }
    pub fn registry(&self) -> String {
        self.inner.registry.clone()
    }
}

/// `tp.dig_lr` in the browser, up to the Dig: the parent file text and the structure text of its SMILES, the `campioni/2` `samples`, the names of the files in the order of the
/// table, the recipe text of the run, the stored peaks of each file (Uint8Arrays in the order of the indexes) and the `.mzidx` read as `index_read(k, offset, length)`.
#[cfg(feature = "proofs")]
#[wasm_bindgen::prelude::wasm_bindgen]
#[allow(clippy::too_many_arguments)]
pub fn tp_dig_inputs(
    rules: &[u8],
    parent: &str,
    structure: &str,
    samples: &str,
    names: Vec<String>,
    recipe: &str,
    peaks: js_sys::Array,
    index_read: js_sys::Function,
    index_sizes: Vec<f64>,
) -> std::result::Result<DigPrepared, wasm_bindgen::JsValue> {
    let run = || -> Result<DigInputs> {
        let file = read_parent(parent.as_bytes());
        let formula = parent_formula(&file, structure)?;
        let p = LinkParent {
            formula: &formula,
            smiles: &file.smiles,
            structure,
            rt: file.rt,
        };
        let mut indexes = crate::open_all(&index_read, &index_sizes)?;
        let mut recipe = match indexes.first_mut() {
            Some(first) => run::recipe_for(recipe, first)?,
            None => run::recipe_from_json(recipe)?,
        };
        let mut src = crate::JsStored { files: peaks };
        dig_inputs(
            rules,
            &p,
            &file.adduct,
            &names,
            samples,
            &mut recipe,
            &mut indexes,
            &mut src,
        )
    };
    run()
        .map(|inner| DigPrepared { inner })
        .map_err(crate::js_err)
}
