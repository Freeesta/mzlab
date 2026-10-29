//! One-file export of the user's data (docs/agenti/esporta-dati.md): wasm-bindgen wrappers of `mzlab_core::zip`.

use wasm_bindgen::prelude::*;

/// The "stored" ZIP of the files in `packed` (per entry: `u16` name length, `u32` data length, little endian, the name, the data).
fn zip_bytes(packed: &[u8]) -> Result<Vec<u8>, String> {
    mzlab_core::zip::zip_packed(packed)
}

/// The packed entries of the "stored" ZIP `zip` (CRC-32 of every entry checked, unsafe names refused).
fn unzip_bytes(zip: &[u8]) -> Result<Vec<u8>, String> {
    mzlab_core::zip::unzip_packed(zip)
}

/// The "stored" ZIP of the files in `packed`, in one pass (the packed layout of `mzlab_core::zip::pack`).
#[wasm_bindgen]
pub fn zip(packed: &[u8]) -> Result<Vec<u8>, JsValue> {
    zip_bytes(packed).map_err(|e| JsValue::from_str(&e))
}

/// The packed entries of the "stored" ZIP `zip` (the inverse of `zip`), in one pass.
#[wasm_bindgen]
pub fn unzip(zip: &[u8]) -> Result<Vec<u8>, JsValue> {
    unzip_bytes(zip).map_err(|e| JsValue::from_str(&e))
}

#[cfg(test)]
mod tests;
