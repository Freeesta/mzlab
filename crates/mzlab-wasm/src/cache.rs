//! The routes of the maps of the session and the byte-budgeted cache of the whole-file maps (see `docs/agenti/memoria.md`).

use mzlab_core::routes::Query;
use wasm_bindgen::prelude::*;

use crate::{error_json, session_items, Engine};

/// Budget of the engine in MB when the page does not say (a WASM instance has 256 MB: 160 of cache + 32 of work + code, stack, fragmentation).
const BUDGET_DEFAULT_MB: f64 = 192.0;

#[wasm_bindgen]
impl Engine {
    /// `api/map`, `api/mapsnap`, `api/mappunti`, `api/mappunti3d` over the session `items`; `query` is the query string of the route.
    /// Empty = Python's (a file without index, parameters Python must reject).
    pub fn map_route(&mut self, path: &str, items: &str, query: &str) -> Result<String, JsValue> {
        let out = self.map_route_inner(path, items, query);
        self.trim_cache();
        out
    }

    /// The bytes of the pyramid of map tiles of a level of the file at `slot` (built if need be) to keep in OPFS; empty = none (no index, no peaks).
    pub fn tiles_export(&mut self, slot: u32, level: u8) -> Result<Vec<u8>, JsValue> {
        let Some(r) = self.idx.get_mut(&slot) else { return Ok(Vec::new()) };
        r.tiles_export(level).map_err(|e| error_json(&e))
    }

    /// True when the pyramid of the level is in memory.
    pub fn tiles_held(&self, slot: u32, level: u8) -> bool {
        self.idx.get(&slot).is_some_and(|r| r.tiles_held(level))
    }

    /// Takes the bytes of `tiles_export` as the pyramid of the level; false when they are damaged or from another index (it is then built from the index).
    pub fn tiles_import(&mut self, slot: u32, level: u8, bytes: &[u8]) -> bool {
        self.idx.get_mut(&slot).is_some_and(|r| r.tiles_import(level, bytes))
    }

    /// Budget of the engine in MB (NaN or 0 = 192; the page lowers it after an out-of-memory death): 5/6 is the cache (one LRU), 1/6 the work.
    pub fn set_budget_mb(&mut self, mb: f64) {
        let mb = if mb.is_finite() && mb > 0.0 { mb } else { BUDGET_DEFAULT_MB };
        self.cache_cap = (mb * 1048576.0 * 5.0 / 6.0) as usize;
        self.work_cap = (mb * 1048576.0 / 6.0) as usize;
        self.trim_cache();
    }

    /// Budget of the cache in MB (0 = the default of `set_budget_mb`); kept for the callers that set only the cache.
    pub fn set_cache_budget(&mut self, mb: f64) {
        self.cache_cap = (mb.max(0.0) * 1048576.0) as usize;
    }

    /// Bytes held by the cache: the maps and the arrays and blocks of the indexes of all the files.
    pub fn cache_bytes(&self) -> f64 {
        mzlab_core::routes::cache_bytes(&self.idx) as f64
    }

    /// Drops the least recently used entries of the cache until it fits its budget; the page calls it after every answer.
    pub fn trim_cache(&mut self) {
        let cap = if self.cache_cap == 0 { (BUDGET_DEFAULT_MB * 1048576.0 * 5.0 / 6.0) as usize } else { self.cache_cap };
        mzlab_core::routes::trim_cache(&mut self.idx, cap);
    }

    /// Budget of the work (what cannot be dropped: the mzML opened whole), bytes.
    pub(crate) fn work_budget(&self) -> usize {
        if self.work_cap == 0 { (BUDGET_DEFAULT_MB * 1048576.0 / 6.0) as usize } else { self.work_cap }
    }

    /// A file opened whole (no index) counts on the work budget: over it the file is refused with `error.memoryBudget` (phase `open`), never a trap.
    pub(crate) fn fits_work(&self, run: &mzlab_core::mzml::Run) -> Result<(), JsValue> {
        let held: usize = self.runs.values().map(run_bytes).sum::<usize>() + run_bytes(run);
        let cap = self.work_budget();
        if held > cap { Err(error_json(&mzlab_core::budget::refusal("open", held, cap))) } else { Ok(()) }
    }

    fn map_route_inner(&mut self, path: &str, items: &str, query: &str) -> Result<String, JsValue> {
        let metas = session_items(items)?;
        let r = mzlab_core::maproute::map_route(path, &metas, &mut self.idx, &Query::parse(query)).map_err(|e| error_json(&e))?;
        Ok(r.unwrap_or_default())
    }
}

/// Bytes an mzML opened whole holds (the peak arrays and the texts of every spectrum).
pub(crate) fn run_bytes(run: &mzlab_core::mzml::Run) -> usize {
    run.spectra.iter().map(|s| 160 + s.id.len() + s.filter.len() + s.mz.len() * 8 + s.intensity.len() * 4 + s.wide.as_ref().map_or(0, |w| w.len() * 8)).sum()
}
