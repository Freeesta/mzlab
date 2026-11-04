//! The streaming gunzip of a `.mzML.gz` for the worker of the engine (`rust-carica.js` `putGz`): pieces of the File in, pieces of the mzML out, written into OPFS as they come.
//! The decoder is `mzlab_core::gunzip` (the same inflate as the readers), so the mzML is the one the desktop app reads from the `.gz`.
use js_sys::Uint8Array;
use wasm_bindgen::prelude::*;

use crate::error_json;

#[wasm_bindgen]
pub struct Gunzip {
    g: mzlab_core::gunzip::Gunzip,
    intero: bool,
}

#[wasm_bindgen]
impl Gunzip {
    /// `tetto`: the most bytes the mzML may have (`err.memory.over4gb` beyond).
    #[wasm_bindgen(constructor)]
    pub fn new(tetto: f64) -> Gunzip {
        Gunzip {
            g: mzlab_core::gunzip::Gunzip::new(tetto as u64),
            intero: false,
        }
    }
    /// The mzML bytes the next piece of the file completes (may be empty).
    pub fn push(&mut self, piece: &[u8]) -> Result<Uint8Array, JsValue> {
        let out = self.g.push(piece).map_err(|e| error_json(&e))?;
        Ok(Uint8Array::from(&out[..]))
    }
    /// The last bytes; `whole()` then says whether the stream ended well (false: a cut file, opened with the spectra it has).
    pub fn finish(&mut self) -> Result<Uint8Array, JsValue> {
        let (out, intero) = self.g.finish().map_err(|e| error_json(&e))?;
        self.intero = intero;
        Ok(Uint8Array::from(&out[..]))
    }
    pub fn whole(&self) -> bool {
        self.intero
    }
    /// Bytes of the mzML given out so far.
    pub fn written(&self) -> f64 {
        self.g.written() as f64
    }
}
