//! The module `valida` of the page: the mzML validation of `mzlab-cli validate` (action «Validate mzML»), reading the file through the page's `read` callback.
//! `valida_mzml` returns the JSON text the command prints with `--json` for one file (`mzlab_core::mzmlvalida::json_un_file`). A module of its own because of the PSI-MS table.
use std::io::Read;

use js_sys::Function;
use wasm_bindgen::prelude::*;

/// Bytes fetched from JS at once.
const BLOCCO: usize = 256 << 10;

/// The file as a stream: `read(0, offset, length) -> Uint8Array` in blocks of `BLOCCO` bytes.
struct JsStream {
    read: Function,
    dim: u64,
    pos: u64,
}

impl Read for JsStream {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        let n = buf
            .len()
            .min(BLOCCO)
            .min(self.dim.saturating_sub(self.pos) as usize);
        if n == 0 {
            return Ok(0);
        }
        let bad = |_| std::io::Error::other("read failed");
        let v = self
            .read
            .call3(
                &JsValue::NULL,
                &0u32.into(),
                &(self.pos as f64).into(),
                &(n as f64).into(),
            )
            .map_err(|_| bad(()))?;
        let a: js_sys::Uint8Array = v.dyn_into().map_err(|_| bad(()))?;
        if a.length() as usize != n {
            return Err(bad(()));
        }
        a.copy_to(&mut buf[..n]);
        self.pos += n as u64;
        Ok(n)
    }
}

/// The name of this module (as `famiglia()` of the readers modules).
#[wasm_bindgen]
pub fn famiglia() -> String {
    "valida".into()
}

/// Validates the mzML `name` of `size` bytes (`read(0, offset, length) -> Uint8Array`): the JSON array `mzlab-cli validate --json` prints for it.
#[wasm_bindgen]
pub fn valida_mzml(name: &str, size: f64, read: Function) -> Result<String, JsValue> {
    let s = JsStream {
        read,
        dim: size as u64,
        pos: 0,
    };
    let r = mzlab_core::mzmlvalida::valida(s).map_err(|e| {
        JsValue::from_str(&format!("{{\"error_key\":\"{}\",\"params\":{{}}}}", e.key))
    })?;
    Ok(mzlab_core::mzmlvalida::json_un_file(name, &r))
}
