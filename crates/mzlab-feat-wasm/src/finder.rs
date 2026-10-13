//! The functions of mzFinder that its Pyodide served and Rust now serves (`mzlab_feat::legacy`): `classify`, `formula_info`, `identity_graph`,
//! `polarity_pairs` and `origin_evidence` with the same JSON, and the Dig of a high-resolution series already through the pipeline (`FinderDig`: `summary`, `detail`,
//! `inclusion_csv`, `frag_tree`, `frag_tree_compare`). Built only into the `.wasm`
//! of the feature `finder`, loaded by the worker of mzFinder when the page asks for one of them. Errors are JavaScript `Error`s with the English text
//! that the Python raised.

use js_sys::{Array, Function};
use mzlab_core::idxread::Index;
use mzlab_feat::legacy::{self, dig::Dig};
use wasm_bindgen::prelude::*;

use crate::JsReader;

fn user(e: String) -> JsValue {
    js_sys::Error::new(&e).into()
}

/// The index of file `k` read through `read(k, offset, length)`; `size` NaN or negative = no index (the error text of the item).
fn open(read: &Function, k: usize, size: f64) -> Result<Index<JsReader>, String> {
    if size.is_nan() || size < 0.0 {
        return Err("no index of the file: open it again".into());
    }
    Index::open(
        JsReader {
            read: read.clone(),
            k: k as u32,
        },
        size as u64,
    )
    .map_err(|e| e.to_string())
}

/// `api.classify(names)`: `read(k, offset, length)` gives the `.mzidx` of `names[k]`, `sizes[k]` its size.
#[wasm_bindgen]
pub fn finder_classify(names: Array, read: Function, sizes: Vec<f64>) -> String {
    let names: Vec<String> = names
        .iter()
        .map(|v| v.as_string().unwrap_or_default())
        .collect();
    let mut k = 0;
    legacy::classify(&names, |_| {
        let r = open(&read, k, sizes.get(k).copied().unwrap_or(f64::NAN));
        k += 1;
        r
    })
}

/// `api.formula_info(text, adduct)`.
#[wasm_bindgen]
pub fn finder_formula_info(text: &str, adduct: &str) -> String {
    legacy::formula_info(text, adduct)
}

/// The Dig of the last `run`, kept by the worker for `detail` and the inclusion list.
#[wasm_bindgen]
pub struct FinderDig {
    inner: Dig,
}

#[wasm_bindgen]
impl FinderDig {
    /// The JSON of `api.run`; `settings` and `timing` are JSON objects copied into it.
    pub fn summary(&self, settings: &str, timing: &str) -> String {
        legacy::out::summary(&self.inner, settings, timing)
    }
    /// `api.detail(id)`.
    pub fn detail(&self, id: u32) -> Result<String, JsValue> {
        legacy::out::detail(&self.inner, id as usize).map_err(user)
    }
    /// `api.inclusion_csv(n)` (0 = the default length).
    pub fn inclusion_csv(&self, n: u32) -> String {
        legacy::out::inclusion_csv(&self.inner, n as usize)
    }
    /// `api.frag_tree()`: the tree of the MS2 of the parent.
    pub fn frag_tree(&self) -> String {
        legacy::fragtree::of_dig(&self.inner)
    }
    /// `api.frag_tree_compare(id)`: no localisation of the products, the empty answer.
    pub fn frag_tree_compare(&self, _id: u32) -> String {
        legacy::fragtree::COMPARE_NONE.into()
    }
}

/// `api.identity_graph(payload)`.
#[wasm_bindgen]
pub fn finder_identity_graph(payload: &str) -> Result<String, JsValue> {
    legacy::identity_graph(payload).map_err(user)
}

/// `api.polarity_pairs(payload)`.
#[wasm_bindgen]
pub fn finder_polarity_pairs(payload: &str) -> Result<String, JsValue> {
    legacy::polarity_pairs(payload).map_err(user)
}

/// `api.origin_evidence(payload)`: the evidence of the report «where does this ion come from?».
#[wasm_bindgen]
pub fn finder_origin_evidence(payload: &str) -> Result<String, JsValue> {
    legacy::edge::origin_evidence(payload).map_err(user)
}

/// `api.run` on high-resolution files: `table` = the `.mzfeat` with the proofs of the LC files (in the order of the files of `files_json` that are
/// samples, blanks or controls), `registry` = `ipotesi/1`, `read`/`sizes` = their `.mzidx` in the same order (the MS2 of the candidates).
#[wasm_bindgen]
pub fn finder_dig(
    files_json: &str,
    parent_json: &str,
    table: &[u8],
    registry: &str,
    read: Function,
    sizes: Vec<f64>,
) -> Result<FinderDig, JsValue> {
    let inner = legacy::dig(files_json, parent_json, table, registry, |k| {
        open(&read, k, sizes.get(k).copied().unwrap_or(f64::NAN))
    })
    .map_err(user)?;
    Ok(FinderDig { inner })
}

/// The peak of the parent in the aligned `table` (`.mzfeat` of the LC files): `{"mz", "rt"}` of its row, `{"mz", "rt": null}` when there is none.
#[wasm_bindgen]
pub fn finder_parent(files_json: &str, parent_json: &str, table: &[u8]) -> Result<String, JsValue> {
    let p = legacy::series::parent(parent_json).map_err(user)?;
    let peak = legacy::parent_peak(files_json, parent_json, table).map_err(user)?;
    let mut o = mzlab_core::json::Out::default();
    o.raw("{\"mz\":").num(p.mz).raw(",\"rt\":").opt(peak.map(|x| x.1)).raw("}");
    Ok(o.0)
}

/// What the worker needs to run the pipeline before `finder_dig`: `{lc: [names of the LC files, in the order of the table], minutes: [time | null],
/// formula: the neutral formula of the parent, structure: the grafo/1 JSON of its SMILES ("" without one), mz: the ion of the parent, rt: the RT the user gave | null}`.
#[wasm_bindgen]
pub fn finder_plan(files_json: &str, parent_json: &str) -> Result<String, JsValue> {
    let lc = legacy::series::lc_files(files_json).map_err(user)?;
    let p = legacy::series::parent(parent_json).map_err(user)?;
    let structure = p
        .smiles
        .as_deref()
        .and_then(|s| mzlab_feat::chem::grafo::analyse_json(s).ok())
        .unwrap_or_default();
    let mut o = mzlab_core::json::Out::default();
    o.raw("{\"lc\":[");
    for (i, f) in lc.iter().enumerate() {
        o.raw(if i > 0 { "," } else { "" }).str(&f.name);
    }
    o.raw("],\"minutes\":[");
    for (i, f) in lc.iter().enumerate() {
        o.raw(if i > 0 { "," } else { "" }).opt(f.time);
    }
    o.raw("],\"formula\":")
        .str(&p.formula())
        .raw(",\"structure\":")
        .str(&structure)
        .raw(",\"mz\":")
        .num(p.mz)
        .raw(",\"rt\":")
        .opt(p.rt)
        .raw("}");
    Ok(o.0)
}

/// The list of transformations of the page (`name;change;comment` per line) added to the `regole_tp` file `rules` as extra rules of the hypotheses.
#[wasm_bindgen]
pub fn finder_transformations(rules: &[u8], text: &str) -> Result<Vec<u8>, JsValue> {
    legacy::transf::merge(rules, text).map_err(user)
}
