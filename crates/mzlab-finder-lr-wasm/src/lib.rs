//! The low-resolution Dig of mzFinder in the browser (`mzlab_feat::legacy::lr`): the plan of the pipeline, the summary of `api.run` and
//! `api.detail` of a series of unit-resolution full-scan files. A `.wasm` of its own (`wasm-feat/finder-lr/`), loaded by the worker of mzFinder
//! only for a low-resolution series. Errors are JavaScript `Error`s with the English text of the Python engine.

use js_sys::{Function, Uint8Array};
use mzlab_core::error::{Error, Result};
use mzlab_core::idxread::{Index, ReadAt};
use mzlab_core::mzmlhead::Mzml;
use mzlab_feat::legacy::lr::mrm::{MrmFile, Trace};
use mzlab_feat::legacy::lr::{self, LrDig};
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;

fn user(e: String) -> JsValue {
    js_sys::Error::new(&e).into()
}

/// `read(k, offset, length)` of the worker: the bytes of the `.mzidx` (or the mzML) of file `k`.
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

fn open(read: &Function, k: usize, size: f64) -> std::result::Result<Index<JsReader>, String> {
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

/// The integration table of the MRM files of the series (`d.mrm` of `api.run`): `read(k, offset, length)` gives the bytes of the mzML of the `k`-th
/// file with kind `mrm` (the order of `mrm` in `finder_lr_plan`), `sizes` its size and `ends` where its last scan ends (0 without scans).
#[wasm_bindgen]
pub fn finder_lr_mrm(
    files_json: &str,
    read: Function,
    sizes: Vec<f64>,
    ends: Vec<f64>,
    rt_tol: f64,
) -> std::result::Result<String, JsValue> {
    let fs = lr::plan::files(files_json).map_err(user)?;
    let mut out = Vec::new();
    for (k, f) in fs.iter().filter(|f| f.kind == "mrm").enumerate() {
        let size = sizes.get(k).copied().unwrap_or(f64::NAN);
        if size.is_nan() || size < 0.0 {
            return Err(user("no mzML of the file: open it again".into()));
        }
        let r = JsReader {
            read: read.clone(),
            k: k as u32,
        };
        let mut m = Mzml::new(r, size as u64);
        let end = ends.get(k).copied().unwrap_or(0.0) as u64;
        let chroms = m.chromatograms(end).map_err(|e| user(e.to_string()))?;
        let mut traces = Vec::new();
        for c in chroms.iter().filter(|c| c.kind == "srm") {
            let (Some(q1), Some(q3)) = (c.q1, c.q3) else {
                continue;
            };
            let (rt, y) = m.chromatogram(c).map_err(|e| user(e.to_string()))?;
            traces.push(Trace {
                q1,
                q3,
                ce: c.ce,
                name: c.name.clone(),
                rt,
                y,
            });
        }
        out.push(MrmFile {
            name: f.name.clone(),
            label: f.label.clone(),
            kind_type: f.kind_type.clone(),
            time: f.time,
            traces,
        });
    }
    Ok(lr::mrm::analyse(&out, rt_tol).unwrap_or_else(|| "null".into()))
}

/// What the worker needs before the pipeline: `{rust, lc: [names of the full-scan columns], minutes, formula}`.
#[wasm_bindgen]
pub fn finder_lr_plan(files_json: &str, parent_json: &str) -> std::result::Result<String, JsValue> {
    lr::plan::plan_json(files_json, parent_json).map_err(user)
}

/// The low-resolution Dig of the last `run`, kept by the worker for `detail`.
#[wasm_bindgen]
pub struct FinderLrDig {
    inner: LrDig,
}

#[wasm_bindgen]
impl FinderLrDig {
    /// The JSON of `api.run`; `timing` is a JSON object copied into it.
    pub fn summary(&self, timing: &str) -> String {
        lr::summary::summary(&self.inner, timing)
    }
    /// The candidates that pass the criteria, for the live feed of the page.
    pub fn live(&self) -> String {
        lr::summary::live(&self.inner)
    }
    /// `api.detail(id)`: `read`/`sizes` give the `.mzidx` of the columns (the XIC of every file).
    pub fn detail(
        &self,
        id: f64,
        read: Function,
        sizes: Vec<f64>,
    ) -> std::result::Result<String, JsValue> {
        lr::detail::detail(&self.inner, id as i64, |k| {
            open(&read, k, sizes.get(k).copied().unwrap_or(f64::NAN))
        })
        .map_err(user)
    }
    /// `api.frag_tree`: the tree of the MS2 of the parent (a note while the series has none).
    pub fn frag_tree(&self) -> String {
        lr::detail::frag_tree(&self.inner)
    }
    /// `api.frag_tree_compare(id)`: the nodes of the tree found in the MS2 of a product.
    pub fn frag_tree_compare(&self, id: f64) -> std::result::Result<String, JsValue> {
        lr::detail::frag_tree_compare(&self.inner, id as i64).map_err(user)
    }
}

/// `api.run` on a low-resolution series: `table` = the `.mzfeat` of the columns of `finder_lr_plan`, `registry` = `ipotesi/1` of the parent.
#[wasm_bindgen]
pub fn finder_lr_dig(
    files_json: &str,
    parent_json: &str,
    settings_json: &str,
    table: &[u8],
    registry: &str,
) -> std::result::Result<FinderLrDig, JsValue> {
    let inner = lr::dig(files_json, parent_json, settings_json, table, registry).map_err(user)?;
    Ok(FinderLrDig { inner })
}
