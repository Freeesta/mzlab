//! SCIEX `.wiff` + `.wiff.scan` (`mzlab_core::wiff`): the samples of a pair and one sample as mzML, written in pieces like the `.raw` conversion.

use crate::{error_json, BlockWriter, Engine, JsReader};
use js_sys::Function;
use mzlab_core::wiff;
use std::io::Write;
use wasm_bindgen::prelude::*;

#[wasm_bindgen]
impl Engine {
    /// The numbers of the samples of a `.wiff` (its bytes), as a JSON array. A file that is not a `.wiff`: error `err.file.notWiff`.
    pub fn wiff_samples(&mut self, name: &str, wiff: Vec<u8>) -> Result<String, JsValue> {
        let ole = wiff::container(wiff, name).map_err(|e| error_json(&e))?;
        Ok(format!("[{}]", wiff::samples(&ole).iter().map(u32::to_string).collect::<Vec<_>>().join(",")))
    }

    /// Sample `sample` of a `.wiff` as mzML, sent to `sink(Uint8Array)` piece by piece. `scan_size` is the size of the `.wiff.scan` read through
    /// `read(offset, length)`, or negative when the pair has none (`err.file.wiffNoScan`). `name` is the `.wiff`.
    pub fn wiff_to_mzml(&mut self, name: &str, wiff: Vec<u8>, sample: u32, scan_size: f64, read: Function, sink: Function) -> Result<(), JsValue> {
        let ole = wiff::container(wiff, name).map_err(|e| error_json(&e))?;
        let w = wiff::Wiff::open(&ole, name, (scan_size >= 0.0).then_some(scan_size as u64), sample).map_err(|e| error_json(&e))?;
        let mut out = BlockWriter { sink, buf: Vec::new() };
        w.to_mzml(&mut JsReader { read }, &mut out, name).map_err(|e| error_json(&e))?;
        out.flush().map_err(|_| error_json(&mzlab_core::Error::new("err.file.wiff").with("detail", "write")))
    }
}
