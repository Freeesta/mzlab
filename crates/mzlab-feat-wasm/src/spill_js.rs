//! The weak tables of the files of a series kept by the page, outside the memory of the module: an object `{put(k, Uint8Array), get(k) -> Uint8Array}` over an
//! array of the worker. `align_series_to` takes it as its optional last argument (`mzlab_feat::spill::Spill`).

use js_sys::{Function, Reflect, Uint8Array};
use mzlab_core::error::{Error, Result};
use mzlab_feat::spill::Spill;
use wasm_bindgen::prelude::*;

struct JsSpill {
    put: Function,
    get: Function,
}

/// The spill of the page, or none when the page gives no object.
pub fn of(o: Option<&js_sys::Object>) -> std::result::Result<Option<Box<dyn Spill>>, JsValue> {
    let Some(o) = o else { return Ok(None) };
    let f = |name: &str| Reflect::get(o, &name.into())?.dyn_into::<Function>();
    Ok(Some(Box::new(JsSpill {
        put: f("put")?,
        get: f("get")?,
    })))
}

/// The spill as the engine takes it.
pub fn borrow(s: &mut Option<Box<dyn Spill>>) -> Option<&mut dyn Spill> {
    s.as_mut().map(|b| &mut **b as &mut dyn Spill)
}

fn failed<T>(_: T) -> Error {
    Error::memory_budget().with("fase", "align")
}

impl Spill for JsSpill {
    fn put(&mut self, k: usize, bytes: Vec<u8>) -> Result<()> {
        let (k, bytes) = ((k as u32).into(), Uint8Array::from(&bytes[..]));
        self.put
            .call2(&JsValue::NULL, &k, &bytes)
            .map(|_| ())
            .map_err(failed)
    }
    fn get(&mut self, k: usize) -> Result<Vec<u8>> {
        let v = self
            .get
            .call1(&JsValue::NULL, &(k as u32).into())
            .map_err(failed)?;
        Ok(Uint8Array::new(&v).to_vec())
    }
    fn external(&self) -> bool {
        true
    }
}
