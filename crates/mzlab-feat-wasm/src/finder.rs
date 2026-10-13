//! The functions of mzFinder that its Pyodide served and Rust now serves (`mzlab_feat::legacy`): `classify` and `formula_info` with the same JSON, and
//! the Dig of a high-resolution series already through the pipeline (`FinderDig`: `summary`, `detail`, `inclusion_csv`). Built only into the `.wasm`
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
