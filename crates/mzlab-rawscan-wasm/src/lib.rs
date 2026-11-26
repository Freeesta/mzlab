//! wasm-bindgen bridge of `api/rawscan` (`mzlab_core::rawscan`, `docs/agenti/raw-access.md`): what is in the mean spectrum of a time window that is not in
//! the background beside it, from the `.mzidx` of a sample (the members, summary and rows of `mzlab-cli raw scan`). A `.wasm` of its own, so that `raw` and
//! `rawxic` keep their size; fetched by the worker of the engine on the first request (`load("rawscan")` of `mzlab/web/rust-raw.js`). Errors are JSON
//! `{"error_key", "params"}` strings, as in the engine.
#![forbid(unsafe_code)]
use js_sys::{Function, Uint8Array};
use mzlab_core::idxread::{Index, ReadAt};
use mzlab_core::routes::{Query, Routes};
use wasm_bindgen::prelude::*;

/// Stable version of the bridge: the page checks that the loaded `.wasm` is the one it was built with.
const BRIDGE: &str = "rawscan/1";

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

/// `api/rawscan?rt=A,B[&bin=..&mz_range=..&top=..&ms_level=..&channel=..&mz_offset=..&min_z=..]` (`Routes::raw_scan_route`) over the `.mzidx` of `size` bytes
/// read by `read`: `{members, "summary":{..}, "rows":[..]}`, the bytes `mzlab-cli raw scan` prints per sample; `pol` as the engine's `set_polarity` (0 = every section).
#[wasm_bindgen]
pub fn raw_scan(size: f64, read: Function, pol: i32, query: &str) -> Result<String, JsValue> {
    let mut r = Routes::new(Index::open(JsReader(read), size as u64).map_err(|e| error_json(&e))?);
    r.pol = [1, -1].contains(&pol).then_some(pol as i8);
    let out = r
        .raw_scan_route(&Query::parse(query))
        .map_err(|e| error_json(&e))?;
    Ok(out.unwrap_or_default())
}
