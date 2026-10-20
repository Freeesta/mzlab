//! wasm-bindgen bridge of `mzlab-libro`: one Markdown chapter in, the HTML page of the Teoria out (same bytes as `tools/libro`, `docs/agenti/libro.md`).
//! A `.wasm` of its own for the administrator Teoria editor. Pure function: no state, no offsets; an error is the string `file:line: message`.
use mzlab_libro::{convert, edit_block, preview, reindex, retitled, Options};
use wasm_bindgen::prelude::*;

/// `mzlab-libro/1`: the page checks that the loaded `.wasm` is the one it was built with.
#[wasm_bindgen]
pub fn version() -> String {
    "mzlab-libro/3".to_string()
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

/// The page for the editor preview: like `convert_chapter`, plus `data-src` on the blocks that can be edited there.
#[wasm_bindgen]
pub fn preview_chapter(text: &str, name: &str, raw_ok: bool) -> Result<String, JsValue> {
    let opt = Options {
        teoria: true,
        raw_ok,
        ..Options::default()
    };
    preview(text, name, &opt).map_err(|e| JsValue::from_str(&e))
}

/// The source with the preview block `src` (its `data-src`) replaced by `new_html`, its inner HTML after the edit; an error text when the edit cannot be written back unchanged in meaning.
#[wasm_bindgen]
pub fn edit_preview_block(
    text: &str,
    name: &str,
    raw_ok: bool,
    src: &str,
    new_html: &str,
) -> Result<String, JsValue> {
    let opt = Options {
        teoria: true,
        raw_ok,
        ..Options::default()
    };
    edit_block(text, name, &opt, src, new_html).map_err(|e| JsValue::from_str(&e))
}

/// Whether the new source retitles h2/h3 sections (the search index then has to be rebuilt with `reindex_chapter`), or the reason why its headings may not change.
#[wasm_bindgen]
pub fn headings_retitled(old_text: &str, new_text: &str, name: &str) -> Result<bool, JsValue> {
    retitled(old_text, new_text, name).map_err(|e| JsValue::from_str(&e))
}

/// The search index (`indice-ricerca.js`) with the rows of the chapter page `file` (e.g. `05-gc.html`) rebuilt from its new `html`.
#[wasm_bindgen]
pub fn reindex_chapter(index_js: &str, file: &str, html: &str) -> Result<String, JsValue> {
    reindex(index_js, file, html).map_err(|e| JsValue::from_str(&e))
}
