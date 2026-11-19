//! wasm-bindgen bridge of `mzlab_core::msngraph`: the `.mzidx` of an infusion and its mzML (both read through `read(offset, length)`) give the
//! `msn-graph/1` of a request, with the same functions as `mzlab-cli msn-graph` (same bytes). A `.wasm` of its own, loaded by the worker of the
//! engine on the first request of the graph; `strutture=<id>` (and `struct_ppm`) adds the cleavages of the SMILES that explain a node; `interpret=1` adds the interpretation of the losses. Memory: the index is read once for its SHA-256 and once to open it; the output is one JSON string,
//! no offsets (`docs/agenti/memory.md`). Errors are JSON `{"error_key", "params"}` strings.
use js_sys::{Function, Uint8Array};
use mzlab_core::budget::{sha256_hex_at, MemBudget};
use mzlab_core::error::Error;
use mzlab_core::idxread::{Index, ReadAt};
use mzlab_core::json::Out;
use mzlab_core::msngraph::{
    aggiungi, cerca, esporta, from_export_request, from_request, graph_json, opzioni_cerca,
};
use mzlab_core::msngraph::{
    library_text, table_text, with_interpretation, Graph, Options, PPM_PERDITA,
};
use mzlab_core::routes::Query;
use mzlab_feat::chem::structures::strutture_testo;
use wasm_bindgen::prelude::*;

/// Bytes of a file asked to JS: `read(offset, length) -> Uint8Array`.
struct JsReader {
    read: Function,
}

impl ReadAt for JsReader {
    fn read_at(&mut self, off: u64, buf: &mut [u8]) -> mzlab_core::Result<()> {
        let bad = || Error::new("error.badIndex");
        let v = self
            .read
            .call2(
                &JsValue::NULL,
                &(off as f64).into(),
                &(buf.len() as f64).into(),
            )
            .map_err(|_| bad())?;
        let a: Uint8Array = v.dyn_into().map_err(|_| bad())?;
        if a.length() as usize != buf.len() {
            return Err(bad());
        }
        a.copy_to(buf);
        Ok(())
    }
}

fn js_err(e: Error) -> JsValue {
    JsValue::from_str(&e.to_json())
}

/// `mzlab-msngrafo/1`: the page checks that the loaded `.wasm` is the one it was built with.
#[wasm_bindgen]
pub fn version() -> String {
    format!("mzlab-msngrafo/1 {}", env!("CARGO_PKG_VERSION"))
}

/// `complete`: the text of an `msn-graph/1` with the key `view` (the bytes `mzlab-cli msn-graph complete` writes); a keyed error for what is not one.
#[wasm_bindgen]
pub fn complete(json: &str) -> Result<String, JsValue> {
    mzlab_core::msngraph::completa(json).map_err(js_err)
}

fn compute(
    read_idx: Function,
    idx_size: f64,
    read_file: Function,
    name: &str,
    q: &Query,
) -> mzlab_core::Result<(Graph, String)> {
    let (g, j, _) = compute_with(read_idx, idx_size, read_file, name, q)?;
    Ok((g, j))
}

fn compute_with(
    read_idx: Function,
    idx_size: f64,
    read_file: Function,
    name: &str,
    q: &Query,
) -> mzlab_core::Result<(Graph, String, Options)> {
    let mut req = from_request(&|k| q.get(k).map(String::from))?;
    // the budget of the instance (`budget_mb` of the page; missing: the default): a consensus over it is the calm `error.memoryBudget`, not a trap
    let mb = q
        .get("budget_mb")
        .and_then(|v| v.parse().ok())
        .unwrap_or(0.0);
    req.0.budget = MemBudget::from_request(mb).cap();
    let bad = || Error::new("error.badIndex");
    if idx_size.is_nan() || idx_size < 0.0 {
        return Err(bad());
    }
    let size = idx_size as u64;
    // hashed in blocks of 1 MiB: no copy of the whole index in memory
    let sha = sha256_hex_at(
        &mut JsReader {
            read: read_idx.clone(),
        },
        size,
    )?;
    let scans = Index::open(JsReader { read: read_idx }, size)?.scans;
    let mut m = JsReader { read: read_file };
    let (g, j) = graph_json(&scans, &mut m, (name, &sha), (&req.0, &req.1))?;
    let j = with_query_keys(&g, j, &req.0, q)?;
    Ok((g, j, req.0))
}

/// The optional keys of the query on the graph text `j`: `interpret=1` and `breaks=1`, shared by the graph and its exports.
fn with_query_keys(g: &Graph, mut j: String, o: &Options, q: &Query) -> mzlab_core::Result<String> {
    // `interpret=1` (and `loss_ppm`) adds the key `interpretation`, as `mzlab-cli msn-graph --interpret` does
    if q.get("interpret").is_some_and(|v| v == "1") {
        let ppm = match q.get("loss_ppm").filter(|v| !v.is_empty()) {
            None => PPM_PERDITA,
            Some(v) => v
                .parse::<f64>()
                .ok()
                .filter(|p| p.is_finite() && *p > 0.0)
                .ok_or_else(|| Error::new("err.request.badNumber").with("name", "loss_ppm"))?,
        };
        j = with_interpretation(&j, g, ppm);
    }
    // `rotture=1` (with `h_max`, `max_legami` and the window `struct_ppm`) adds the key `breaks`, as `mzlab-cli msn-graph --breaks` does
    if q.get("breaks").is_some_and(|v| v == "1") {
        let num = |k: &str, d: f64| -> mzlab_core::Result<f64> {
            match q.get(k).filter(|v| !v.is_empty()) {
                None => Ok(d),
                Some(v) => v
                    .parse::<f64>()
                    .ok()
                    .filter(|p| p.is_finite() && *p >= 0.0)
                    .ok_or_else(|| Error::new("err.request.badNumber").with("name", k)),
            }
        };
        let p = mzlab_feat::chem::breaks::Params {
            ppm: num("struct_ppm", PPM_STRUCTURE)?,
            h_max: num("h_max", 2.0)?.min(4.0) as u8,
            max_bonds: num("max_bonds", 2.0)?.clamp(1.0, 2.0) as u8,
        };
        j = mzlab_feat::chem::breaks::breaks_text(g, &j, o.smiles.as_deref(), p)?;
    }
    Ok(j)
}

/// Default window of the cleavages (`struct_ppm`), ppm; the same as `mzlab-cli msn-graph --structures`.
const PPM_STRUCTURE: f64 = 10.0;

/// The `msn-graph/1` of the file `nome` for the query string `query` (`formula`, `tol`, `ce`, `ppm_formula`, ..., `pick`; `api/msn_grafo`).
/// `read_idx` reads the `.mzidx` (`idx_size` bytes) and `read_file` the mzML the index points into.
#[wasm_bindgen]
pub fn msn_graph(
    read_idx: Function,
    idx_size: f64,
    read_file: Function,
    nome: &str,
    query: &str,
) -> Result<String, JsValue> {
    let q = Query::parse(query);
    let (g, j, o) = compute_with(read_idx, idx_size, read_file, nome, &q).map_err(js_err)?;
    match q.get("structures").filter(|n| !n.is_empty()) {
        None => Ok(j),
        Some(node) => {
            let ppm = match q.get("struct_ppm").filter(|v| !v.is_empty()) {
                None => PPM_STRUCTURE,
                Some(v) => v
                    .parse::<f64>()
                    .ok()
                    .filter(|p| p.is_finite() && *p > 0.0)
                    .ok_or_else(|| {
                        js_err(Error::new("err.request.badNumber").with("name", "struct_ppm"))
                    })?,
            };
            strutture_testo(&g, &j, o.smiles.as_deref(), (node, ppm)).map_err(js_err)
        }
    }
}

/// The spectra of the same graph as text (`api/msn_grafo/export`: `format=csv|mgf|msp`, `clean=1`, `node=<id>` besides the query of the graph).
/// The texts of the Flow steps come from the same route: `format=massbank` (or `msp` with `library=1`) is the tree as a library (`msn.library_export`:
/// `levels=all|ms2`, `annotate=0|1`, `id`), `view=modifications|nodes|peaks` the CSV table of the graph (`msn.table_export`).
#[wasm_bindgen]
pub fn msn_graph_export(
    read_idx: Function,
    idx_size: f64,
    read_file: Function,
    nome: &str,
    query: &str,
) -> Result<String, JsValue> {
    let q = Query::parse(query);
    let clean_flag = q.get("clean").is_some_and(|v| v == "1");
    if let Some(view) = q.get("view").filter(|v| !v.is_empty()) {
        let (_, j) = compute(read_idx, idx_size, read_file, nome, &q).map_err(js_err)?;
        let v = mzlab_core::json::parse(j.as_bytes()).map_err(js_err)?;
        return Ok(table_text(&v, view, clean_flag));
    }
    let format = q.get("format").unwrap_or("");
    if format == "massbank" || q.get("library").is_some_and(|v| v == "1") {
        let (_, j) = compute(read_idx, idx_size, read_file, nome, &q).map_err(js_err)?;
        let v = mzlab_core::json::parse(j.as_bytes()).map_err(js_err)?;
        let get = |k: &str, d: &'static str| q.get(k).filter(|s| !s.is_empty()).unwrap_or(d);
        let format = if format == "massbank" {
            "massbank"
        } else {
            "msp"
        };
        let annotate = get("annotate", "1") != "0";
        return library_text(
            &v,
            &j,
            (
                get("id", ""),
                format,
                get("levels", "all"),
                clean_flag,
                annotate,
            ),
        )
        .map_err(js_err);
    }
    let (f, clean, node) = from_export_request(&|k| q.get(k).map(String::from)).map_err(js_err)?;
    let (g, _) = compute(read_idx, idx_size, read_file, nome, &q).map_err(js_err)?;
    Ok(esporta(&g, f, clean, node.as_deref()))
}

/// `api/msnlib/add`: the library text `library` ("" = none yet) with the entry of `molecule_id` for the graph of the query added;
/// `{"library": text, "duplicate": bool}` (the text unchanged when the same entry was already there).
#[wasm_bindgen]
pub fn msnlib_add(
    read_idx: Function,
    idx_size: f64,
    read_file: Function,
    nome: &str,
    query: &str,
    library: &str,
    molecule_id: &str,
) -> Result<String, JsValue> {
    let q = Query::parse(query);
    let (g, j) = compute(read_idx, idx_size, read_file, nome, &q).map_err(js_err)?;
    let (l, duplicate) = aggiungi(library, molecule_id, &g, &j).map_err(js_err)?;
    let mut o = Out::default();
    o.raw("{\"library\":").str(&l).raw(",\"duplicate\":");
    o.raw(if duplicate { "true}" } else { "false}" });
    Ok(o.0)
}

/// `api/msnlib/search`: the candidates of the library text `library` for the graph of the query (`msn-search/1`); the query carries the options of the graph and
/// those of the search (`precursor_ppm`, `tol_mz`, `min_evidence`, `top`, `delta_max`).
#[wasm_bindgen]
pub fn msnlib_search(
    read_idx: Function,
    idx_size: f64,
    read_file: Function,
    nome: &str,
    query: &str,
    library: &str,
) -> Result<String, JsValue> {
    let q = Query::parse(query);
    let o = opzioni_cerca(&|k| q.get(k).map(String::from)).map_err(js_err)?;
    let (_, j) = compute(read_idx, idx_size, read_file, nome, &q).map_err(js_err)?;
    cerca(library, &j, &o).map_err(js_err)
}
