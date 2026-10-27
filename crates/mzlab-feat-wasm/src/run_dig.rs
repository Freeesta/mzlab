//! `tp.dig_lr` up to `lr::dig`, shared by the native executor (`mzlab-cli`, `flow_digl.rs`) and the browser (the export `tp_dig_inputs` below): the polarity, the hypotheses searched in
//! the stored peaks of the series and the JSON of the files and of the parent that the Dig reads.

use mzlab_core::error::Result;
use mzlab_core::idxread::{Index, ReadAt};
use mzlab_core::json::Out;
use mzlab_feat::recipe::Recipe;
use mzlab_feat::slabs::Stored;

use crate::run;
use crate::run_tp::{parent_formula, read_parent, LinkParent};

/// What the Dig of a unit-resolution series (`lr::dig`) is given besides the table: the files and samples, the parent and the `ipotesi/1` the search in the series annotated.
pub struct DigInputs {
    pub files: String,
    pub who: String,
    pub registry: String,
}

/// The polarity the Dig looks in: the recipe's, else the sign of the parent adduct, else what the MS1 scans of the files say (a mix or no scans read as positive).
fn dig_polarity<I: ReadAt>(adduct: &str, recipe: &Recipe, indexes: &[Index<I>]) -> i8 {
    let sum: i64 = indexes
        .iter()
        .map(|ix| i64::from(mzlab_feat::facts::file_facts(ix).polarity))
        .sum();
    recipe
        .detect
        .polarity
        .filter(|&p| p != 0)
        .or_else(|| adduct.ends_with('-').then_some(-1))
        .or_else(|| (sum != 0).then(|| sum.signum() as i8))
        .unwrap_or(1)
}

/// `tp.dig_lr` up to `lr::dig`, shared by the native executor (`flow_digl.rs`) and the browser (`tp_dig_inputs`): the polarity, the hypotheses searched in the stored peaks
/// of the series, and the JSON of the files (`names`, in the order of the table) and of the parent that the Dig reads. `[M+H]+` meeting a negative series is read as `[M-H]-`.
pub fn dig_inputs<I: ReadAt>(
    rules: &[u8],
    p: &LinkParent,
    adduct: &str,
    names: &[String],
    samples: &str,
    recipe: &mut Recipe,
    indexes: &mut [Index<I>],
    peaks: &mut dyn Stored,
) -> Result<DigInputs> {
    let polarity = dig_polarity(adduct, recipe, indexes);
    recipe.detect.polarity = Some(polarity);
    let hypo = run::hypotheses_stored(indexes, peaks, rules, p.structure, p.formula, None, recipe)?;
    let mut files = Out::default();
    files.raw("{\"files\":[");
    for (i, n) in names.iter().enumerate() {
        files
            .raw(if i == 0 { "" } else { "," })
            .raw("{\"name\":")
            .str(n)
            .raw(",\"kind\":\"full\"}");
    }
    files.raw("],\"campioni\":").raw(samples.trim()).raw("}");
    let adduct = if adduct == "[M+H]+" && polarity < 0 {
        "[M-H]-"
    } else {
        adduct
    };
    let mut who = Out::default();
    who.raw("{\"name\":\"parent\",\"neutral\":")
        .str(p.formula)
        .raw(",\"adduct\":")
        .str(adduct)
        .raw("}");
    Ok(DigInputs {
        files: files.0,
        who: who.0,
        registry: hypo.registry.to_json(),
    })
}

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
        let mut indexes = (0..index_sizes.len())
            .map(|k| crate::open_one(&index_read, k, index_sizes[k]))
            .collect::<Result<Vec<_>>>()?;
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
