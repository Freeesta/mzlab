//! wasm-bindgen bridge of `mzlab-libro`: one Markdown chapter in, the HTML page of the Teoria out (same bytes as `tools/libro`, `docs/agenti/libro.md`).
//! A `.wasm` of its own for the administrator Teoria editor. Pure function: no state, no offsets; an error is the string `file:line: message`.
use mzlab_libro::{convert, Options};
use wasm_bindgen::prelude::*;

/// `mzlab-libro/1`: the page checks that the loaded `.wasm` is the one it was built with.
#[wasm_bindgen]
pub fn version() -> String {
    "mzlab-libro/1".to_string()
}

/// HTML of the chapter `name` (e.g. `12-dati.md`); `raw_ok` = the file is listed in `tools/libro/importati.txt` (raw ```html blocks allowed).
#[wasm_bindgen]
pub fn convert_chapter(text: &str, name: &str, raw_ok: bool) -> Result<String, JsValue> {
    let opt = Options {
        teoria: true,
        raw_ok,
        ..Options::default()
    };
    convert(text, name, &opt).map_err(|e| JsValue::from_str(&e.to_string()))
}
