//! wasm-bindgen bridge of `mzlab-feat`. It runs only inside the Web Worker of mzFinder (`TP_Mine/files/feat-worker.js`), as a `.wasm` of its own that is
//! imported only when mzFinder is on. The `.mzidx` files are never held whole: the worker gives `read(k, offset, length) -> Uint8Array` for file `k` (an OPFS
//! sync handle). Outputs are byte arrays (`.mzfeatf`, `.mzfeat`); errors are JSON `{"error_key": ..., "params": {...}}` strings (the page translates the key).
//! The steps themselves are in `run`, shared with the native example, so the browser and the desktop write the same bytes.
use js_sys::{Array, Function, Uint8Array};
use mzlab_core::error::{Error, Result};
use mzlab_core::idxread::{Index, ReadAt};
use mzlab_core::json::Out;
#[cfg(feature = "proofs")]
use mzlab_feat::store::read_file;
use wasm_bindgen::prelude::*;

pub mod run;

/// Stable version of the bridge: the page checks that the loaded `.wasm` is the one it was built with.
const BRIDGE: &str = "mzfeat-wasm/1";

fn js_err(e: Error) -> JsValue {
    let mut o = Out::default();
    o.raw("{\"error_key\":").str(e.key).raw(",\"params\":{");
    for (i, (k, v)) in e.params.iter().enumerate() {
        o.raw(if i > 0 { "," } else { "" }).str(k).raw(":").str(v);
    }
    o.raw("}}");
    JsValue::from_str(&o.0)
}

/// Version of the bridge and of the feature engine: `mzfeat-wasm/1 feat=3`.
#[wasm_bindgen]
pub fn version() -> String {
    format!("{BRIDGE} feat={}", mzlab_feat::FEAT_VERSION)
}

/// Bytes of file `k` held by JavaScript: `read(k, offset, length) -> Uint8Array`.
struct JsReader {
    read: Function,
    k: u32,
}

impl ReadAt for JsReader {
    fn read_at(&mut self, off: u64, buf: &mut [u8]) -> Result<()> {
        let io = || Error::new("error.io");
        let v = self
            .read
            .call3(
                &JsValue::NULL,
                &self.k.into(),
                &(off as f64).into(),
                &(buf.len() as f64).into(),
            )
            .map_err(|_| io())?;
        let a: Uint8Array = v.dyn_into().map_err(|_| io())?;
        if a.length() as usize != buf.len() {
            return Err(Error::new("error.badIndex"));
        }
        a.copy_to(buf);
        Ok(())
    }
}

fn open_one(read: &Function, k: usize, size: f64) -> Result<Index<JsReader>> {
    let r = JsReader {
        read: read.clone(),
        k: k as u32,
    };
    Index::open(r, size as u64)
}

#[cfg(feature = "proofs")]
fn open_all(read: &Function, sizes: &[f64]) -> Result<Vec<Index<JsReader>>> {
    (0..sizes.len())
        .map(|k| open_one(read, k, sizes[k]))
        .collect()
}

/// The byte arrays of a JS list, copied one at a time: the caller parses and drops each before the next is copied (peak memory = the largest, not the sum).
fn arrays(list: &Array) -> impl Iterator<Item = Vec<u8>> + '_ {
    list.iter().map(|v| Uint8Array::new(&v).to_vec())
}

#[cfg(feature = "detect")]
/// L1-L3 of one file (`.mzidx` of `size` bytes read with `read`): the bytes of its `.mzfeatf`. `recipe` is `{"polarity": 1|-1|null, "mode": "ion"|"family"}`.
#[wasm_bindgen]
pub fn detect_file(
    read: Function,
    size: f64,
    name: &str,
    recipe: &str,
) -> std::result::Result<Vec<u8>, JsValue> {
    let r = run::recipe_from_json(recipe).map_err(js_err)?;
    let mut ix = open_one(&read, 0, size).map_err(js_err)?;
    run::detect(&mut ix, name, &r).map_err(js_err)
}

#[cfg(feature = "detect")]
/// L4-L6: alignment and gap filling of the `.mzfeatf` of a series (an `Array` of `Uint8Array`, in the order of the indexes): the bytes of the `.mzfeat`.
#[wasm_bindgen]
pub fn align_series(
    files: Array,
    read: Function,
    sizes: Vec<f64>,
    recipe: &str,
) -> std::result::Result<Vec<u8>, JsValue> {
    let r = run::recipe_from_json(recipe).map_err(js_err)?;
    let (feats, origin) = run::read_files(arrays(&files)).map_err(js_err)?;
    run::align(&feats, &origin, &r, |k| open_one(&read, k, sizes[k])).map_err(js_err)
}

#[cfg(feature = "proofs")]
/// The hypotheses of a parent searched in a series: kept in the memory of the worker for `build_evidence`.
#[wasm_bindgen]
pub struct Hypotheses {
    inner: run::HypoRun,
}

#[cfg(feature = "proofs")]
#[wasm_bindgen]
impl Hypotheses {
    /// The registry `ipotesi/1` as JSON (hypotheses with sources, formula, m/z and outcome).
    pub fn registry_json(&self) -> String {
        self.inner.registry.to_json()
    }
    /// Hypotheses searched.
    pub fn tried(&self) -> u32 {
        self.inner.result.tried
    }
    /// Hypotheses with a peak in at least one file.
    pub fn found(&self) -> u32 {
        self.inner.result.found
    }
}

#[cfg(feature = "proofs")]
/// L7: hypotheses of `parent` (formula) from the rules (`regole_tp.json` bytes), searched in every file. `parent_rt` NaN = the whole run.
#[wasm_bindgen]
pub fn run_hypotheses(
    files: Array,
    read: Function,
    sizes: Vec<f64>,
    rules: &[u8],
    parent: &str,
    parent_rt: f64,
    recipe: &str,
) -> std::result::Result<Hypotheses, JsValue> {
    let r = run::recipe_from_json(recipe).map_err(js_err)?;
    let feats = arrays(&files)
        .map(|b| read_file(&b).map(|x| x.0))
        .collect::<Result<Vec<_>>>()
        .map_err(js_err)?;
    let refs: Vec<&_> = feats.iter().collect();
    let mut ix = open_all(&read, &sizes).map_err(js_err)?;
    let rt = parent_rt.is_finite().then_some(parent_rt);
    let inner = run::hypotheses(&mut ix, &refs, rules, parent, rt, &r).map_err(js_err)?;
    Ok(Hypotheses { inner })
}

#[cfg(feature = "proofs")]
/// L8-L9: the graph of the proofs on the `.mzfeat` of the series; `minutes[k]` is the treatment time of file `k` (NaN = a control); `parent_mz` NaN =
/// no parent in the table. Returns the `.mzfeat` bytes with the `evid` section.
#[wasm_bindgen]
pub fn build_evidence(
    table: &[u8],
    hypotheses: &Hypotheses,
    minutes: Vec<f64>,
    parent_mz: f64,
    parent_rt: f64,
    recipe: &str,
) -> std::result::Result<Vec<u8>, JsValue> {
    let r = run::recipe_from_json(recipe).map_err(js_err)?;
    let parent = (parent_mz.is_finite() && parent_rt.is_finite()).then_some((parent_mz, parent_rt));
    run::evidence(table, &hypotheses.inner, &minutes, parent, &r).map_err(js_err)
}
