//! The detection of one file and the alignment of a series telling a JavaScript function how far they are: `progress(done, total)`, at most once per 1 %
//! (`Pace`). Called from `detect_file_to` and `align_series_to`.

use js_sys::Function;
use mzlab_core::budget::WriteAt;
use mzlab_core::error::Result;
use mzlab_core::idxread::{Index, ReadAt};
use mzlab_core::pace::Pace;
use mzlab_feat::recipe::Recipe;
use wasm_bindgen::JsValue;

/// `progress(done, total)` of JavaScript as a Rust callback (nothing without it).
pub fn calling(progress: Option<Function>) -> impl FnMut(usize, usize) {
    move |d: usize, t: usize| {
        if let Some(f) = &progress {
            let _ = f.call2(
                &JsValue::NULL,
                &JsValue::from_f64(d as f64),
                &JsValue::from_f64(t as f64),
            );
        }
    }
}

/// `run::detect_to_with` with `progress` (optional) as the callback; the bytes written are the same without it.
pub fn detect_to<R: ReadAt, W: WriteAt>(
    index: &mut Index<R>,
    name: &str,
    r: &Recipe,
    sink: &mut W,
    progress: Option<Function>,
) -> Result<()> {
    let mut tell = calling(progress);
    let mut pace = Pace::new(&mut tell);
    crate::run::detect_to_with(index, name, r, sink, &mut |d, t| pace.at(d, t))
}
