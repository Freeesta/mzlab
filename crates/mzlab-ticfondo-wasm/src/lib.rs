//! wasm-bindgen bridge of `api/tic_background` (`mzlab_core::ticbg_route`, `docs/agenti/tic-fondo.md`): the TIC or BPC without the chemical
//! background, over the `.mzidx` of a file and of its blank. A `.wasm` of its own (the engine has no room for it), fetched by the worker of the
//! engine on the first request (`load("ticfondo")` of `mzlab/web/rust-msngrafo.js`). Errors are JSON `{"error_key", "params"}` strings, as in the engine.
#![forbid(unsafe_code)]
use js_sys::{Function, Uint8Array};
use mzlab_core::idxread::{Index, ReadAt};
use mzlab_core::routes::{Query, Routes};
use wasm_bindgen::prelude::*;

/// Stable version of the bridge: the page checks that the loaded `.wasm` is the one it was built with.
const BRIDGE: &str = "ticfondo/1";

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

/// Bytes of an index file asked to JS: `read(offset, length) -> Uint8Array` (as the engine's reader).
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

fn routes_of(size: f64, read: Function, pol: i32) -> Result<Routes<JsReader>, JsValue> {
    let mut r = Routes::new(Index::open(JsReader(read), size as u64).map_err(|e| error_json(&e))?);
    r.pol = [1, -1].contains(&pol).then_some(pol as i8);
    Ok(r)
}

/// `api/tic_background` (`ticbg_route::route_pair`) over the `.mzidx` of a file and, for `mode=blank`, of its blank (`blank_read` null = none);
/// `pol` as the engine's `set_polarity` (0 = every section). Empty = not served (a filtered trace).
#[wasm_bindgen]
pub fn tic_background(
    size: f64,
    read: Function,
    pol: i32,
    blank_size: f64,
    blank_read: Option<Function>,
    blank_pol: i32,
    query: &str,
) -> Result<String, JsValue> {
    let mut r = routes_of(size, read, pol)?;
    let mut b = match blank_read {
        Some(f) => Some(routes_of(blank_size, f, blank_pol)?),
        None => None,
    };
    let out = mzlab_core::ticbg_route::route_pair(&mut r, b.as_mut(), &Query::parse(query))
        .map_err(|e| error_json(&e))?;
    Ok(out.unwrap_or_default())
}
