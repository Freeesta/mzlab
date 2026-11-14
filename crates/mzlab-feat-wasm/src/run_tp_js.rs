//! JS exports of `run_tp`; the plain functions are `mzlab_ops::run::feat::run_tp`.

use crate::run::{self};
use crate::run_tp::{
    generate_text, hypo_params, link_evidence, parent_formula, read_parent, LinkParent, LinkRun,
    Series,
};
use mzlab_core::budget::MemBudget;
use mzlab_core::error::Result;
use mzlab_core::json::parse;
use mzlab_feat::tp::TpRoles;

/// A SMILES or a formula in the parent file, as `{"formula","smiles","rt"}`: the page asks the `grafo` .wasm for the structure of the SMILES before `tp_generate`.
#[cfg(feature = "proofs")]
#[wasm_bindgen::prelude::wasm_bindgen]
pub fn tp_parent(parent: &str) -> String {
    let bytes = parent.as_bytes();
    let p = read_parent(bytes);
    let mut o = mzlab_core::json::Out::default();
    o.raw("{\"formula\":")
        .str(&p.formula)
        .raw(",\"smiles\":")
        .str(&p.smiles);
    o.raw(",\"rt\":");
    match p.rt {
        Some(t) => o.num(t),
        None => o.raw("null"),
    };
    o.raw("}");
    o.0
}

/// `tp.generate` in the browser: the registry of the hypotheses of the parent file `parent` (text) with the structure text of its SMILES (empty for a formula)
/// under the parameters `params` (JSON object of the step, `{}` for the defaults) and the rules.
#[cfg(feature = "proofs")]
#[wasm_bindgen::prelude::wasm_bindgen]
pub fn tp_generate(
    rules: &[u8],
    parent: &str,
    structure: &str,
    params: &str,
) -> std::result::Result<String, wasm_bindgen::JsValue> {
    let run = || -> Result<String> {
        let p = read_parent(parent.as_bytes());
        let formula = parent_formula(&p, structure)?;
        let set = parse(params.as_bytes())?;
        generate_text(
            rules,
            &formula,
            structure,
            &hypo_params(&|n| set.get(n).cloned()),
        )
    };
    run().map_err(crate::js_err)
}

/// The parameters of `tp.generate` the files decide when the plan does not write them (`mzlab_feat::nature`, the rule of `flow run`): the names the plan writes
/// and the `.mzidx` of the series read as `index_read(k, offset, length)`; `[{"name","value","proof"}]`. The name `?facets` asks the facets of the
/// files for the check of the Flow page instead: `{"resolution":[...],"polarity":[...]}` (one export for both keeps the hr `.wasm` under its cap).
#[cfg(feature = "proofs")]
#[wasm_bindgen::prelude::wasm_bindgen]
pub fn series_estimates(
    written: Vec<String>,
    index_read: js_sys::Function,
    index_sizes: Vec<f64>,
) -> std::result::Result<String, wasm_bindgen::JsValue> {
    let run = || -> Result<String> {
        let indexes = crate::open_all(&index_read, &index_sizes)?;
        // the rule `flow run` puts on the scan inputs before the check
        if written.iter().any(|w| w == "?facets") {
            let files = indexes.iter().map(mzlab_feat::nature::index_facts);
            return Ok(mzlab_feat::nature::facets_json(
                &mzlab_feat::nature::facets(files),
            ));
        }
        let nature = mzlab_feat::nature::of_indexes(&indexes);
        let list = mzlab_feat::nature::estimates(&|n| written.iter().any(|w| w == n), &nature);
        Ok(mzlab_feat::nature::estimates_json(&list))
    };
    run().map_err(crate::js_err)
}

/// What `tp_link_evidence` gives the page: the table with `tp/1` and what `tp_link_interp` (the .wasm `interp`) needs next.
#[cfg(feature = "proofs")]
#[wasm_bindgen::prelude::wasm_bindgen]
pub struct LinkEvidence {
    inner: LinkRun,
}

#[cfg(feature = "proofs")]
#[wasm_bindgen::prelude::wasm_bindgen]
impl LinkEvidence {
    /// The `.mzfeat` bytes with the block `tp/1`.
    pub fn table(&self) -> Vec<u8> {
        self.inner.with_tp.clone()
    }
    /// The registry `ipotesi/1` the search annotated.
    pub fn registry(&self) -> String {
        self.inner.registry.clone()
    }
    /// The parent as the interpretation reads it.
    pub fn parent(&self) -> String {
        self.inner.parent.clone()
    }
    pub fn polarity(&self) -> i32 {
        self.inner.polarity as i32
    }
    /// Position of the infusion file in the series, -1 without one.
    pub fn infusion(&self) -> i32 {
        self.inner.infusion.map_or(-1, |k| k as i32)
    }
}

/// `tp.link@2` up to the proofs, in the browser: the parent file text and the structure text of its SMILES, the `ipotesi/1` of `tp.generate`, the `.mzfeat` of
/// the series (`table`), the `campioni/2` `samples`, the recipe text of the run, the polarity the plan sets (NaN = not set), the memory budget (MB; 0/NaN = the default),
/// the stored peaks of each file (Uint8Arrays in the order of the indexes) and the `.mzidx` read as `index_read(k, offset, length)` with `index_sizes[k]` bytes.
#[cfg(feature = "proofs")]
#[wasm_bindgen::prelude::wasm_bindgen]
#[allow(clippy::too_many_arguments)]
pub fn tp_link_evidence(
    rules: &[u8],
    parent: &str,
    structure: &str,
    hypotheses: &str,
    table: &[u8],
    samples: &str,
    recipe: &str,
    polarity: f64,
    budget_mb: f64,
    peaks: js_sys::Array,
    index_read: js_sys::Function,
    index_sizes: Vec<f64>,
) -> std::result::Result<LinkEvidence, wasm_bindgen::JsValue> {
    let run = || -> Result<LinkRun> {
        let file = read_parent(parent.as_bytes());
        let formula = parent_formula(&file, structure)?;
        let p = LinkParent {
            formula: &formula,
            smiles: &file.smiles,
            structure,
            rt: file.rt,
        };
        let roles = TpRoles::for_table(samples, table)?;
        let mut indexes = crate::open_all(&index_read, &index_sizes)?;
        let mut recipe = match indexes.first_mut() {
            Some(first) => run::recipe_for(recipe, first)?,
            None => run::recipe_from_json(recipe)?,
        };
        let mut hyp_src = crate::JsStored {
            files: peaks.clone(),
        };
        let mut row_src = crate::JsStored { files: peaks };
        let mut open = |k: usize| crate::open_one(&index_read, k, index_sizes[k]);
        link_evidence(
            rules,
            &p,
            hypotheses,
            table,
            &roles,
            &mut recipe,
            polarity.is_finite().then_some(polarity),
            Series {
                indexes: &mut indexes,
                peaks: &mut hyp_src,
                peaks_again: &mut row_src,
                open: &mut open,
            },
            &MemBudget::from_request(budget_mb),
            &mut |_| {},
        )
    };
    run()
        .map(|inner| LinkEvidence { inner })
        .map_err(crate::js_err)
}
