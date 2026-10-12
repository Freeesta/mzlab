//! The routes of the maps of the session and the byte-budgeted cache of the whole-file maps (see `docs/agenti/memoria.md`).

use mzlab_core::routes::Query;
use wasm_bindgen::prelude::*;

use crate::{error_json, session_items, Engine};

/// Default budget of the cache of maps, bytes (the whole engine has 160 MB of cache and 32 MB of work in 256 MB).
const MAP_CACHE_DEFAULT: usize = 64 << 20;

#[wasm_bindgen]
impl Engine {
    /// `api/map`, `api/mapsnap`, `api/mappunti`, `api/mappunti3d` over the session `items`; `query` is the query string of the route.
    /// Empty = Python's (a file without index, parameters Python must reject).
    pub fn map_route(&mut self, path: &str, items: &str, query: &str) -> Result<String, JsValue> {
        let out = self.map_route_inner(path, items, query);
        let cap = if self.cache_cap == 0 { MAP_CACHE_DEFAULT } else { self.cache_cap };
        mzlab_core::routes::trim_map_cache(&mut self.idx, cap);
        out
    }

    /// Budget of the cache of maps in MB (0 = 64): the least recently used maps of all the files go first.
    pub fn set_cache_budget(&mut self, mb: f64) {
        self.cache_cap = (mb.max(0.0) * 1048576.0) as usize;
    }

    /// Bytes held by the cached maps of all the files.
    pub fn cache_bytes(&self) -> f64 {
        mzlab_core::routes::map_cache_bytes(&self.idx) as f64
    }

    fn map_route_inner(&mut self, path: &str, items: &str, query: &str) -> Result<String, JsValue> {
        let metas = session_items(items)?;
        let r = mzlab_core::maproute::map_route(path, &metas, &mut self.idx, &Query::parse(query)).map_err(|e| error_json(&e))?;
        Ok(r.unwrap_or_default())
    }
}
