//! wasm-bindgen bridge of `api/rawiso` (`mzlab_core::rawiso`, `docs/agenti/raw-access.md`): the net counts of M, M+1 and M+2 over the peak region of an m/z and a
//! time window, their observed ratios against the pattern of a formula, from the `.mzidx` of a sample (the members and `result` of `mzlab-cli raw isotopes`). A
//! `.wasm` of its own, so that `raw` and `rawxic` keep their size; fetched by the worker of the engine on the first request (`load("rawiso")` of
//! `mzlab/web/rust-raw.js`). Errors are JSON `{"error_key", "params"}` strings, as in the engine.
#![forbid(unsafe_code)]
use js_sys::{Function, Uint8Array};
use mzlab_core::idxread::{Index, ReadAt};
use mzlab_core::routes::{Query, Routes};
use wasm_bindgen::prelude::*;

/// Stable version of the bridge: the page checks that the loaded `.wasm` is the one it was built with.
const BRIDGE: &str = "rawiso/1";

fn error_json(e: &mzlab_core::Error) -> JsValue {
    JsValue::from_str(&e.to_json())
}

#[wasm_bindgen]
pub fn version() -> String {
    BRIDGE.to_string()
}

/// The message of a panic goes to a global of the worker (`__mzlabPanic`) before the abort, as in the engine.
#[wasm_bindgen(start)]
pub fn start() {
    std::panic::set_hook(Box::new(|info| {
        let _ = js_sys::Reflect::set(
            &js_sys::global(),
            &"__mzlabPanic".into(),
            &info.to_string().into(),
        );
    }));
}

/// Bytes of an index file asked to JS: `read(offset, length) -> Uint8Array` (the reader of `raw`, kept here so that neither module links the other).
struct JsReader(Function);

impl ReadAt for JsReader {
    fn read_at(&mut self, off: u64, buf: &mut [u8]) -> mzlab_core::Result<()> {
        let bad = || mzlab_core::idxread::read_error(off, buf.len());
        let v = self
            .0
            .call2(
                &JsValue::NULL,
                &(off as f64).into(),
                &(buf.len() as f64).into(),
            )
            .map_err(|_| bad())?;
        let a: Uint8Array = v.dyn_into().map_err(|_| bad())?;
        if a.length() as usize != buf.len() {
            return Err(bad());
        }
        a.copy_to(buf);
        Ok(())
    }
}

/// `api/rawiso?mz=M&rt=A,B[&formula=..&adduct=..&bin=nominal&ms_level=..&mz_offset=..&<threshold>=..]` (`Routes::raw_isotopes_route`) over the `.mzidx` of `size`
/// bytes read by `read`: `{members, "result":{..}}`, the bytes `mzlab-cli raw isotopes` prints per sample; `pol` as the engine's `set_polarity` (0 = every section).
#[wasm_bindgen]
pub fn raw_isotopes(size: f64, read: Function, pol: i32, query: &str) -> Result<String, JsValue> {
    let mut r = Routes::new(Index::open(JsReader(read), size as u64).map_err(|e| error_json(&e))?);
    r.pol = [1, -1].contains(&pol).then_some(pol as i8);
    let out = r
        .raw_isotopes_route(&Query::parse(query))
        .map_err(|e| error_json(&e))?;
    Ok(out.unwrap_or_default())
}
