//! FDR of the library search for the page (lower bound; see `mzlab_lib::decoy`).
use crate::{js_err, tol_of, Lib};
use mzlab_lib::decoy::{search_fdr, search_fdr_json, DecoyKind};
use mzlab_lib::error::Error;
use mzlab_lib::search::{IdentityOpts, Query};
use wasm_bindgen::prelude::*;

#[wasm_bindgen]
impl Lib {
    /// FDR of the library search (lower bound, see `decoy.rs`) for a batch of queries on a library built with decoys. The queries are concatenated:
    /// `starts[k]..starts[k+1]` are the peaks of query `k` in `mz`/`it` (`starts` has one more value than `precs`). `kind`: 0 fragments, 1 precursor.
    /// Returns the JSON of `search_fdr_json`.
    #[allow(clippy::too_many_arguments)]
    pub fn search_with_fdr(
        &self,
        precs: &[f64],
        pols: &[i32],
        starts: &[u32],
        mz: &[f64],
        it: &[f64],
        tol: f64,
        ppm: bool,
        frag: f64,
        kind: u32,
    ) -> std::result::Result<String, JsValue> {
        let n = precs.len();
        let ok = pols.len() == n
            && starts.len() == n + 1
            && mz.len() == it.len()
            && starts.windows(2).all(|w| w[0] <= w[1])
            && starts.last().is_none_or(|&e| e as usize <= mz.len());
        if !ok {
            return Err(js_err(Error::new("lib.err.io")));
        }
        let o = IdentityOpts {
            prec_tol: tol_of(tol, ppm),
            frag_da: frag,
            pol: 1,
            top: 20,
        };
        let kind = if kind == 1 {
            DecoyKind::Precursor
        } else {
            DecoyKind::Fragments
        };
        let qs: Vec<Option<Query>> = (0..n)
            .map(|k| {
                let (a, b) = (starts[k] as usize, starts[k + 1] as usize);
                Query::new(precs[k], pols[k] as i8, &mz[a..b], &it[a..b])
            })
            .collect();
        let res = search_fdr(&self.inner, &qs, &o, kind).map_err(js_err)?;
        Ok(search_fdr_json(&res))
    }
}
