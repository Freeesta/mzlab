//! The POST routes of the statistics of the series (`api/stat/profile|blank|reference|pca`).

use wasm_bindgen::prelude::*;

/// `mzlab-core::statroute`: the JSON answer, prefixed with `E` for an error (status 400); empty when `path` is not one of the routes.
#[wasm_bindgen]
pub fn stat_post(path: &str, body: &str) -> String {
    match mzlab_core::statroute::stat_route(path, body) {
        None => String::new(),
        Some(Ok(json)) => json,
        Some(Err(json)) => format!("E{json}"),
    }
}
