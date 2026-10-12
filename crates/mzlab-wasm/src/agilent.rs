//! Agilent MassHunter `.D` folders read in the worker: one run written as mzML, sent to JS in pieces. The small files of the folder arrive as bytes
//! (an empty array = the file is missing); `AcqData/MSPeak.bin` is read by offset, block by block, never whole.
use js_sys::Function;
use mzlab_core::agilent::{AgilentD, Files};
use wasm_bindgen::prelude::*;

use crate::{error_json, BlockWriter, Engine, JsReader};

fn some(b: &[u8]) -> Option<&[u8]> {
    (!b.is_empty()).then_some(b)
}

#[wasm_bindgen]
impl Engine {
    /// The run of the folder `name` (`x.D`) as mzML, sent to `sink(Uint8Array)` piece by piece; `progress(done, total)` is told how many spectra are written.
    #[allow(clippy::too_many_arguments)]
    pub fn agilent_to_mzml(
        &mut self,
        name: &str,
        ms_scan: &[u8],
        ms_scan_xsd: &[u8],
        ms_ts: &[u8],
        devices: &[u8],
        contents: &[u8],
        ms_tree: &[u8],
        acqmeth: &[u8],
        peak_size: f64,
        peak_read: Function,
        sink: Function,
        progress: Function,
    ) -> Result<(), JsValue> {
        let files = Files {
            ms_scan,
            ms_scan_xsd: some(ms_scan_xsd),
            ms_peak_len: peak_size as u64,
            ms_ts: some(ms_ts),
            devices: some(devices),
            contents: some(contents),
            ms_tree: some(ms_tree),
            acqmeth: some(acqmeth),
            others: &[],
        };
        let run = AgilentD::open(name, &files).map_err(|e| error_json(&e))?;
        let mut out = BlockWriter {
            sink,
            buf: Vec::new(),
        };
        let mut tell = |done: usize, total: usize| {
            let _ = progress.call2(
                &JsValue::NULL,
                &(done as f64).into(),
                &(total as f64).into(),
            );
        };
        run.to_mzml_progress(
            &mut JsReader { read: peak_read },
            peak_size as u64,
            &mut out,
            &mut tell,
        )
        .map_err(|e| error_json(&e))?;
        std::io::Write::flush(&mut out).map_err(|_| {
            error_json(&mzlab_core::Error::new("err.file.agilent").with("detail", "write"))
        })
    }
}
