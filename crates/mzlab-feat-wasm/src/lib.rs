//! wasm-bindgen bridge of `mzlab-feat`. It runs only inside the Web Worker of mzFinder (`TP_Mine/files/feat-worker.js`), as a `.wasm` of its own that is
//! imported only when mzFinder is on. The `.mzidx` files are never held whole: the worker gives `read(k, offset, length) -> Uint8Array` for file `k` (an OPFS
//! sync handle). Outputs are byte arrays (`.mzfeatf`, `.mzfeat`); errors are JSON `{"error_key": ..., "params": {...}}` strings (the page translates the key).
//! The steps themselves are in `run`, shared with the native example, so the browser and the desktop write the same bytes.
#![forbid(unsafe_code)]
use js_sys::{Array, Function, Uint8Array};
use mzlab_core::budget::{Hashing, MemBudget, WriteAt};
use mzlab_core::error::{Error, Result};
use mzlab_core::idxread::{Index, ReadAt};
use mzlab_core::json::Out;
use mzlab_feat::slabs::Stored;
use wasm_bindgen::prelude::*;

pub mod run;
#[cfg(feature = "proofs")]
pub mod state;

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

/// Bytes of the linear memory of this WASM instance (0 off wasm32): the CI guards its peak per phase (`tests/test_guardie_memoria.py`).
#[wasm_bindgen]
pub fn memory_bytes() -> f64 {
    #[cfg(target_arch = "wasm32")]
    {
        (core::arch::wasm32::memory_size(0) * 65536) as f64
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        0.0
    }
}

/// Version of the crate `mzlab-feat` inside this `.wasm` (`x.y.z`): the page compares it with the one the gate of quality was measured on.
#[wasm_bindgen]
pub fn crate_version() -> String {
    mzlab_feat::CRATE_VERSION.to_string()
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

/// Where an output goes: `write(offset, bytes: Uint8Array)` in JavaScript (an OPFS sync handle), called in order.
struct JsSink {
    write: Function,
}

impl WriteAt for JsSink {
    fn write_at(&mut self, off: u64, data: &[u8]) -> Result<()> {
        self.write
            .call2(&JsValue::NULL, &(off as f64).into(), &Uint8Array::from(data))
            .map(|_| ())
            .map_err(|_| Error::new("error.io"))
    }
}

/// `{"bytes": N, "sha256": "..."}`: what a step that wrote to a sink returns instead of the bytes.
fn written(bytes: u64, sha256: &str) -> String {
    let mut o = Out::default();
    o.raw("{\"bytes\":").int(bytes as i64);
    o.raw(",\"sha256\":").str(sha256).raw("}");
    o.0
}

/// Runs `step` on a hashing sink over `write` and gives back `{bytes, sha256}`.
fn to_sink(
    write: Function,
    step: impl FnOnce(&mut Hashing<JsSink>) -> Result<()>,
) -> std::result::Result<String, JsValue> {
    let mut sink = Hashing::new(JsSink { write });
    step(&mut sink).map_err(js_err)?;
    let (_, n, sha) = sink.finish();
    Ok(written(n, &sha))
}

/// The `.mzfeatf` of a series held in JavaScript (an `Array` of `Uint8Array`): a copy of one file is made when it is asked for and dropped after use.
struct JsStored {
    files: Array,
}

impl Stored for JsStored {
    fn count(&self) -> usize {
        self.files.length() as usize
    }
    fn bytes(&mut self, k: usize) -> Result<Vec<u8>> {
        Ok(Uint8Array::new(&self.files.get(k as u32)).to_vec())
    }
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
/// L1-L3 of one file written to `write(offset, bytes)`: `{"bytes", "sha256"}` of the `.mzfeatf`.
#[wasm_bindgen]
pub fn detect_file_to(
    read: Function,
    size: f64,
    name: &str,
    recipe: &str,
    write: Function,
) -> std::result::Result<String, JsValue> {
    let r = run::recipe_from_json(recipe).map_err(js_err)?;
    let mut ix = open_one(&read, 0, size).map_err(js_err)?;
    to_sink(write, |sink| run::detect_to(&mut ix, name, &r, sink))
}

#[cfg(feature = "detect")]
/// Preview of one file: detection with the numbers of `recipe`, the estimated noise floor, tolerance and peak width with their proofs, the histogram of
/// the heights and three example traces, as JSON (`mzlab_feat::params::preview_json`). Without `floor` in the recipe the floor is the estimated one.
#[wasm_bindgen]
pub fn preview_file(
    read: Function,
    size: f64,
    recipe: &str,
) -> std::result::Result<String, JsValue> {
    let r = run::recipe_from_json(recipe).map_err(js_err)?;
    let mut ix = open_one(&read, 0, size).map_err(js_err)?;
    run::preview(&mut ix, &r, run::floor_given(recipe)).map_err(js_err)
}

#[cfg(feature = "detect")]
/// L4-L6: alignment and gap filling of the `.mzfeatf` of a series (an `Array` of `Uint8Array`, in the order of the indexes) written to
/// `write(offset, bytes)`: `{"bytes", "sha256"}` of the `.mzfeat`. The same bytes whatever `budget_mb` (0/NaN = 192): in memory, in slabs of m/z, or
/// the error `error.memoryBudget` with `{fase, stima_mb, budget_mb}`.
#[wasm_bindgen]
pub fn align_series_to(
    files: Array,
    read: Function,
    sizes: Vec<f64>,
    recipe: &str,
    write: Function,
    budget_mb: f64,
) -> std::result::Result<String, JsValue> {
    let r = run::recipe_from_json(recipe).map_err(js_err)?;
    let budget = MemBudget::from_request(budget_mb);
    let mut src = JsStored { files };
    let open = |k| open_one(&read, k, sizes[k]);
    to_sink(write, |sink| run::align_to(&mut src, &budget, &r, open, sink))
}

#[cfg(feature = "detect")]
/// L4-L6 with the bytes of the `.mzfeat` returned (they are in memory as a whole: for small series and tests; the page uses `align_series_to`).
#[wasm_bindgen]
pub fn align_series(
    files: Array,
    read: Function,
    sizes: Vec<f64>,
    recipe: &str,
) -> std::result::Result<Vec<u8>, JsValue> {
    let r = run::recipe_from_json(recipe).map_err(js_err)?;
    let budget = MemBudget::from_request(f64::NAN);
    let mut out = Vec::new();
    let mut src = JsStored { files };
    let open = |k| open_one(&read, k, sizes[k]);
    run::align_to(&mut src, &budget, &r, open, &mut out).map_err(js_err)?;
    Ok(out)
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
    /// The state the proofs need, as bytes (`load_hypotheses` gives the object back in another worker).
    pub fn state(&self) -> Vec<u8> {
        state::dump(&self.inner)
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
/// The hypotheses of a run made in another worker, from `Hypotheses.state()`: all that `build_evidence_to` reads.
#[wasm_bindgen]
pub fn load_hypotheses(state: &[u8]) -> std::result::Result<Hypotheses, JsValue> {
    Ok(Hypotheses {
        inner: state::load(state).map_err(js_err)?,
    })
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
    let mut ix = open_all(&read, &sizes).map_err(js_err)?;
    let rt = parent_rt.is_finite().then_some(parent_rt);
    let mut src = JsStored { files };
    let inner = run::hypotheses_stored(&mut ix, &mut src, rules, parent, rt, &r).map_err(js_err)?;
    Ok(Hypotheses { inner })
}

#[cfg(feature = "proofs")]
/// L8-L9: the graph of the proofs on the `.mzfeat` of the series, read by offsets with `table_read(offset, length)`, written to `write(offset, bytes)`:
/// `{"bytes", "sha256"}` of the `.mzfeat` with the `evid` section. `minutes[k]` is the treatment time of file `k` (NaN = a control); `parent_mz` NaN = no
/// parent in the table. The same bytes whatever `budget_mb` (0/NaN = 192): the graph whole, or made identity by identity (`error.memoryBudget` if not even so).
#[wasm_bindgen]
pub fn build_evidence_to(
    table_read: Function,
    table_size: f64,
    hypotheses: &Hypotheses,
    minutes: Vec<f64>,
    parent_mz: f64,
    parent_rt: f64,
    recipe: &str,
    write: Function,
    budget_mb: f64,
) -> std::result::Result<String, JsValue> {
    let r = run::recipe_from_json(recipe).map_err(js_err)?;
    let parent = (parent_mz.is_finite() && parent_rt.is_finite()).then_some((parent_mz, parent_rt));
    let budget = MemBudget::from_request(budget_mb);
    let mut table = JsReader {
        read: table_read,
        k: 0,
    };
    to_sink(write, |sink| {
        let size = table_size as u64;
        run::evidence_to(&mut table, size, &hypotheses.inner, &minutes, parent, &r, &budget, sink)
    })
}

#[cfg(feature = "proofs")]
/// L8-L9 with the bytes in and out as wholes (small series and tests; the page uses `build_evidence_to`).
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
    let budget = MemBudget::from_request(f64::NAN);
    let mut out = Vec::new();
    let mut at = mzlab_feat::store::Slice(table);
    run::evidence_to(&mut at, table.len() as u64, &hypotheses.inner, &minutes, parent, &r, &budget, &mut out)
        .map_err(js_err)?;
    Ok(out)
}
