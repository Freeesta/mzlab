//! One-file export of the user's data (docs/agenti/esporta-dati.md): wasm-bindgen wrappers of `mzlab_core::zip`.

use wasm_bindgen::prelude::*;

/// Size in bytes of the "stored" ZIP of the files in `packed` (per entry: `u16` name length, `u32` data length, little endian, the name, the data).
#[wasm_bindgen]
pub fn zip_len(packed: &[u8]) -> Result<u32, JsValue> {
    let z = mzlab_core::zip::zip_packed(packed).map_err(|e| JsValue::from_str(&e))?;
    Ok(z.len() as u32)
}

/// Writes that ZIP into `out` (of `zip_len` bytes); returns the bytes written.
#[wasm_bindgen]
pub fn zip_write(packed: &[u8], out: &mut [u8]) -> Result<u32, JsValue> {
    let z = mzlab_core::zip::zip_packed(packed).map_err(|e| JsValue::from_str(&e))?;
    let dst = out
        .get_mut(..z.len())
        .ok_or_else(|| JsValue::from_str("zip: output too small"))?;
    dst.copy_from_slice(&z);
    Ok(z.len() as u32)
}
