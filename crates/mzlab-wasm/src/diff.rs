//! `ref=` on `api/chrom`, `api/xic` and `api/mrm`: the traces come from the indexes of the slots (the sample files and the reference), the
//! subtraction is `mzlab_core::diffroute`. "" = Python's; "E" + JSON = the error of the user's setting (status 400).

use super::{error_json, Engine};
use mzlab_core::diffroute::{mrm_diff, plan, Tr};
use mzlab_core::routes::Query;
use wasm_bindgen::prelude::*;

#[wasm_bindgen]
impl Engine {
    /// `api/chrom` or `api/xic` with `ref` for the files at the slots of the query.
    pub fn diff_traces(&mut self, path: &str, query: &str) -> Result<String, JsValue> {
        let p = match plan(path, &Query::parse(query)) {
            None => return Ok(String::new()),
            Some(Err(e)) => return Ok(format!("E{e}")),
            Some(Ok(p)) => p,
        };
        let mut get = |k: u32| -> Result<Option<Tr>, JsValue> {
            match self.idx.get_mut(&k) {
                None => Ok(None),
                Some(r) => p.fetch(k, r).map_err(|e| error_json(&e)),
            }
        };
        let Some(reference) = get(p.o.reference)? else { return Ok(String::new()) };
        let mut files = Vec::new();
        for &k in &p.ks {
            match get(k)? {
                Some(t) => files.push(t),
                None => return Ok(String::new()),
            }
        }
        Ok(p.finish(&reference, &files))
    }
}

/// `api/mrm` with `ref`: `files` = `[{"k", "transitions"}]` and `reference` = the transitions of the reference file (both as `api/mrm` gives them).
#[wasm_bindgen]
pub fn mrm_diff_post(files: &str, reference: &str, query: &str) -> String {
    match mrm_diff(files, reference, &Query::parse(query)) {
        None => String::new(),
        Some(Ok(json)) => json,
        Some(Err(json)) => format!("E{json}"),
    }
}
