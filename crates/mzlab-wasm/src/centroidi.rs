//! A profile mzML of the archive as a copy with one point per peak (`mzlab_core::centroidi`), sent to JS in pieces like the other conversions.
use js_sys::Function;
use std::io::Write;
use wasm_bindgen::prelude::*;

use crate::{error_json, BlockReader, BlockWriter, Engine};

#[wasm_bindgen]
impl Engine {
    /// A profile mzML of `size` bytes (read through `read(offset, length)`, twice, never held) as a copy with one point per peak, sent to `sink(Uint8Array)`; `progress(done, total)` counts the spectra.
    /// Returns false (nothing sent) when the file has no profile spectrum.
    pub fn mzml_to_centroids(
        &mut self,
        name: &str,
        size: f64,
        read: Function,
        sink: Function,
        progress: Function,
    ) -> Result<bool, JsValue> {
        let mut out = BlockWriter {
            sink,
            buf: Vec::new(),
        };
        let mut tell = |done: usize, total: usize| {
            drop(progress.call2(
                &JsValue::NULL,
                &(done as f64).into(),
                &(total as f64).into(),
            ))
        };
        let did = mzlab_core::centroidi::mzml_to_centroids(
            || Ok(BlockReader::new(read.clone(), size as u64)),
            &mut out,
            name,
            &mut tell,
        )
        .map_err(|e| error_json(&e))?;
        out.flush().map_err(|_| {
            error_json(&mzlab_core::Error::new("err.file.mzmlWrite").with("detail", "write"))
        })?;
        Ok(did)
    }
}
