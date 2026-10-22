//! The detection of one file telling a JavaScript function how far it is: `progress(done, total)`, at most once per 1 % of the peaks (`Pace`). Called from `detect_file_to`.

use js_sys::Function;
use mzlab_core::budget::WriteAt;
use mzlab_core::error::Result;
use mzlab_core::idxread::{Index, ReadAt};
use mzlab_core::pace::Pace;
use mzlab_feat::recipe::Recipe;
use wasm_bindgen::JsValue;

/// `run::detect_to_with` with `progress` (optional) as the callback; the bytes written are the same without it.
pub fn detect_to<R: ReadAt, W: WriteAt>(
    index: &mut Index<R>,
    name: &str,
    r: &Recipe,
    sink: &mut W,
    progress: Option<Function>,
) -> Result<()> {
    let mut tell = |d: usize, t: usize| {
        if let Some(f) = &progress {
            let _ = f.call2(
                &JsValue::NULL,
                &JsValue::from_f64(d as f64),
                &JsValue::from_f64(t as f64),
            );
        }
    };
    let mut pace = Pace::new(&mut tell);
    crate::run::detect_to_with(index, name, r, sink, &mut |d, t| pace.at(d, t))
}
