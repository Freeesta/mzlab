//! `method/1` of one sample of a `.wiff` dropped without its `.scan` (module `sciex`): the same `from_wiff` the CLI and the app run, with the extended metadata (`estesi`), because the
//! engine's .wasm is built without it (its cap has no room). `read(0, offset, length) -> Uint8Array`, the callback of the module (`rust-metodo.js` wraps the engine's reader).
use js_sys::Function;
use mzlab_core::method::{Source, SrcFile};
use mzlab_core::ole::Ole;
use wasm_bindgen::prelude::*;

/// A `.wiff` is read whole to open its container; more than this is refused with phase `whole` and the sizes, as every reader does.
const WIFF_MAX: u64 = 192 << 20;

fn fail(e: &mzlab_core::Error) -> JsValue {
    JsValue::from_str(&e.to_json())
}

/// `method/1` of `sample` of the `.wiff` of `size` bytes (`sha1` = its SHA-1 when known, `""` otherwise).
#[wasm_bindgen]
pub fn wiff_metodo(
    name: &str,
    size: f64,
    read: Function,
    sample: u32,
    sha1: &str,
) -> Result<String, JsValue> {
    let whole = mzlab_core::readers::Limits {
        byte: WIFF_MAX,
        ..mzlab_core::readers::Limits::sito()
    };
    whole.fits(size as u64).map_err(|e| fail(&e))?;
    let data = crate::reading::from_js(&read, 0, 0, size as usize)
        .map_err(|e| fail(&e))?
        .to_vec();
    let ole = Ole::open(data, name, &mzlab_core::readers::Limits::held()).map_err(|e| fail(&e))?;
    let src = Source {
        format: "sciex-wiff",
        files: vec![SrcFile {
            name: name.to_string(),
            sha1: (!sha1.is_empty()).then(|| sha1.to_string()),
            bytes: size as u64,
        }],
        sample: Some(sample),
    };
    Ok(mzlab_core::method_wiff::from_wiff(&ole, sample, &src))
}
