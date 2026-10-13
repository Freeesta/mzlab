//! `.wiff` + `.wiff.scan` pairs (SCIEX 3200 QTRAP) read in the worker: the samples of a `.wiff` and one sample written as mzML, sent to JS in pieces.
//! The `.wiff` (the container with the indexes and the method) is read whole; the `.wiff.scan` is read by offset, block by block, never whole.
use js_sys::Function;
use mzlab_core::idxread::ReadAt;
use mzlab_core::ole::Ole;
use mzlab_core::wiff::{self, Wiff};
use wasm_bindgen::prelude::*;

use crate::{error_json, BlockWriter, Engine, JsReader};

fn container(size: f64, read: Function, name: &str) -> Result<Ole, JsValue> {
    let mut bytes = vec![0u8; size as usize];
    JsReader { read }.read_at(0, &mut bytes).map_err(|e| error_json(&e))?;
    wiff::container(bytes, name).map_err(|e| error_json(&e))
}

#[wasm_bindgen]
impl Engine {
    /// The sample numbers of a `.wiff` of `size` bytes: JSON `[2,3,...]`.
    pub fn wiff_samples(&mut self, name: &str, size: f64, read: Function) -> Result<String, JsValue> {
        let list: Vec<String> = wiff::samples(&container(size, read, name)?).iter().map(u32::to_string).collect();
        Ok(format!("[{}]", list.join(",")))
    }

    /// Sample `sample` of the `.wiff` as mzML, sent to `sink(Uint8Array)` piece by piece; `progress(done, total)` is told how many spectra are written.
    /// `scan_size < 0`: the `.wiff.scan` is missing.
    #[allow(clippy::too_many_arguments)]
    pub fn wiff_to_mzml(&mut self, name: &str, size: f64, read: Function, scan_size: f64, scan_read: Function, sample: u32, centroid: bool, sink: Function, progress: Function) -> Result<(), JsValue> {
        let ole = container(size, read, name)?;
        let len = (scan_size >= 0.0).then_some(scan_size as u64);
        let mut w = Wiff::open(&ole, name, len, sample).map_err(|e| error_json(&e))?;
        w.wiff_sha1 = Some(ole.sha1_hex());
        let mut out = BlockWriter { sink, buf: Vec::new() };
        let mut tell = |done: usize, total: usize| {
            let _ = progress.call2(&JsValue::NULL, &(done as f64).into(), &(total as f64).into());
        };
        w.to_mzml_progress(&mut JsReader { read: scan_read }, &mut out, name, centroid, &mut tell).map_err(|e| error_json(&e))?;
        std::io::Write::flush(&mut out).map_err(|_| error_json(&mzlab_core::Error::new("err.file.wiff").with("detail", "write")))
    }
}
