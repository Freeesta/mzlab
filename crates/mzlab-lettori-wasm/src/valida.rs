//! `valida_mzml`: the mzML validation of `mzlab-cli validate` in the `aperti` module (the page's action «Validate mzML»), reading the file through the page's `read` callback.
//! Returns the JSON text the command prints with `--json` for one file (`mzlab_core::mzmlvalida::json_un_file`).
use std::io::Read;

use js_sys::Function;
use wasm_bindgen::prelude::*;

use crate::{da_js, error_json, LETTURA};

/// The file as a stream: `read(0, offset, length)` in blocks of `LETTURA` bytes.
struct JsStream {
    read: Function,
    dim: u64,
    pos: u64,
}

impl Read for JsStream {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        let n = buf
            .len()
            .min(LETTURA)
            .min(self.dim.saturating_sub(self.pos) as usize);
        if n == 0 {
            return Ok(0);
        }
        let a =
            da_js(&self.read, 0, self.pos, n).map_err(|_| std::io::Error::other("read failed"))?;
        a.copy_to(&mut buf[..n]);
        self.pos += n as u64;
        Ok(n)
    }
}

/// Validates the mzML `name` of `size` bytes (`read(0, offset, length) -> Uint8Array`): the JSON array `mzlab-cli validate --json` prints for it.
#[wasm_bindgen]
pub fn valida_mzml(name: &str, size: f64, read: Function) -> Result<String, JsValue> {
    let s = JsStream {
        read,
        dim: size as u64,
        pos: 0,
    };
    let r = mzlab_core::mzmlvalida::valida(s).map_err(|e| error_json(&e))?;
    Ok(mzlab_core::mzmlvalida::json_un_file(name, &r))
}
