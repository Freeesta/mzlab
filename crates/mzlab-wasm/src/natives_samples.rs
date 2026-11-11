//! `native_samples`: the samples of a dataset, each with why it has nothing to show (empty in the index, or rebuilt from sections the index lacks).

use js_sys::Function;
use wasm_bindgen::prelude::*;

use crate::natives::Members;

/// The samples of a dataset (`names`, `sizes`, `read(member, offset, length)`): JSON `{"format": id, "samples": [{"id", "name", "scans"}], "unindexed"?: {"recovered", "skipped"}}`.
#[wasm_bindgen]
pub fn native_samples(
    names: js_sys::Array,
    sizes: &[f64],
    read: Function,
) -> Result<String, JsValue> {
    use mzlab_core::json::Out;
    let mut l = Members::new(names, sizes, read)?.open(false)?;
    let (found, skipped) = l.unindexed();
    let n = l.samples().len();
    let mut o = Out::default();
    o.raw("{\"format\":")
        .str(&format!("{:?}", l.format()))
        .raw(",\"samples\":[");
    for i in 0..n {
        let s = &l.samples()[i];
        // why a sample has nothing to show: no scans and no chromatograms, or rebuilt from sections the index does not list (the last `found` samples)
        let none = s.scansioni == 0 && l.chromatograms(i).is_ok_and(|v| v.is_empty());
        let why = [(i + found >= n && found > 0, "inferred"), (none, "empty")]
            .into_iter()
            .find(|w| w.0);
        let s = &l.samples()[i];
        let p = if i > 0 { ",{\"id\":" } else { "{\"id\":" };
        o.raw(p)
            .int(i64::from(s.id))
            .raw(",\"name\":")
            .str(&s.nome)
            .raw(",\"scans\":")
            .int(s.scansioni as i64);
        if let Some(dim) = s.size {
            o.raw(",\"size\":").int(dim as i64);
        }
        if let Some((_, w)) = why {
            o.raw(",\"why\":").str(w);
        }
        o.raw("}");
    }
    o.raw("]");
    // sections of the data file the container's index does not list (an incomplete copy): the page says it once
    if found + skipped > 0 {
        o.raw(",\"unindexed\":{\"recovered\":")
            .int(found as i64)
            .raw(",\"skipped\":")
            .int(skipped as i64)
            .raw("}");
    }
    o.raw("}");
    Ok(o.0)
}
