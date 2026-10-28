//! The helper workers of one index build (`idx-aiuti.js`): `JsCorriere` hands the arrays of each spectrum to the page's `send(seq, job, bound)`
//! and takes the answers with `take(seq)` (a `Uint8Array`, or `undefined` when the parser must decode it); `decode_job` is what a helper runs.

use js_sys::{Function, Uint8Array};
use mzlab_core::mzml::{Corriere, Decodificatore};
use std::cell::RefCell;
use wasm_bindgen::prelude::*;

/// The courier of `build_mzidx`; without the two functions (`window` 0) every spectrum is decoded by the builder itself.
pub(crate) struct JsCorriere {
    send: Option<Function>,
    take: Option<Function>,
    window: usize,
}

impl JsCorriere {
    /// From the helpers of `idx-aiuti.js` ({send, take, window}); anything else gives no helper.
    pub(crate) fn new(o: &JsValue) -> Self {
        // `Reflect.get` of undefined throws: no helper
        let get = |k: &str| js_sys::Reflect::get(o, &k.into()).ok();
        let f = |k: &str| get(k).and_then(|v| v.dyn_into::<Function>().ok());
        let window = get("window").and_then(|v| v.as_f64()).unwrap_or(0.0);
        JsCorriere {
            send: f("send"),
            take: f("take"),
            window: window.max(0.0) as usize,
        }
    }
}

impl Corriere for JsCorriere {
    fn window(&self) -> usize {
        if self.send.is_some() && self.take.is_some() {
            self.window
        } else {
            0
        }
    }
    fn send(&mut self, seq: u64, job: &[u8], bound: usize) -> bool {
        let Some(f) = &self.send else { return false };
        f.call3(
            &JsValue::NULL,
            &(seq as f64).into(),
            &Uint8Array::from(job),
            &(bound as f64).into(),
        )
        .is_ok_and(|v| v.is_truthy())
    }
    fn take(&mut self, seq: u64) -> Option<Vec<u8>> {
        let v = self
            .take
            .as_ref()?
            .call1(&JsValue::NULL, &(seq as f64).into())
            .ok()?;
        v.dyn_ref::<Uint8Array>().map(Uint8Array::to_vec)
    }
}

thread_local! {
    static DEC: RefCell<Decodificatore> = RefCell::new(Decodificatore::default());
}

/// A helper's job: the decoded arrays of one spectrum (`mzml::Decodificatore`), or `undefined` when it does not decode (the builder decodes it then).
#[wasm_bindgen]
pub fn decode_job(job: &[u8]) -> Option<Vec<u8>> {
    DEC.with(|d| d.borrow_mut().decode(job))
}
