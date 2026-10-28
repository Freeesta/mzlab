//! The helper workers of one index build (`idx-aiuti.js`): `JsCorriere` hands the arrays of each spectrum to the page's `send(seq, job, bound)`
//! and takes the answers with `take(seq)` (a `Uint8Array`, or `undefined` when the parser must decode it); `decode_job` is what a helper runs.

use js_sys::{Function, Uint8Array};
use mzlab_core::mzml::{Corriere, Decodificatore};
use std::cell::RefCell;
use wasm_bindgen::prelude::*;

/// The courier of `build_mzidx`; without the two functions (`window` 0) every spectrum is decoded by the builder itself.
pub(crate) struct JsCorriere {
    pub(crate) send: Option<Function>,
    pub(crate) take: Option<Function>,
    pub(crate) window: usize,
}

impl Corriere for JsCorriere {
    fn window(&self) -> usize {
        if self.send.is_some() && self.take.is_some() { self.window } else { 0 }
    }
    fn send(&mut self, seq: u64, job: &[u8], bound: usize) -> bool {
        let Some(f) = &self.send else { return false };
        f.call3(&JsValue::NULL, &(seq as f64).into(), &Uint8Array::from(job), &(bound as f64).into())
            .is_ok_and(|v| v.is_truthy())
    }
    fn take(&mut self, seq: u64) -> Option<Vec<u8>> {
        let v = self.take.as_ref()?.call1(&JsValue::NULL, &(seq as f64).into()).ok()?;
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
