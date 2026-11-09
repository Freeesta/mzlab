//! wasm-bindgen bridge of `mzlab_core::msngraph`: the `.mzidx` of an infusion and its mzML (both read through `read(offset, length)`) give the
//! `msngrafo/1` of a request, with the same functions as `mzlab-cli msn-graph` (same bytes). A `.wasm` of its own, loaded by the worker of the
//! engine on the first request of the graph; `strutture=<id>` (and `ppm_struttura`) adds the cleavages of the SMILES that explain a node; `interpreta=1` adds the interpretation of the losses. Memory: the index is read once for its SHA-256 and once to open it; the output is one JSON string,
//! no offsets (`docs/agenti/memoria.md`). Errors are JSON `{"error_key", "params"}` strings.
use js_sys::{Function, Uint8Array};
use mzlab_core::budget::sha256_hex;
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

/// `complete`: the text of an `msngrafo/1` with the key `vista` (the bytes `mzlab-cli msn-graph complete` writes); a keyed error for what is not one.
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
    let req = from_request(&|k| q.get(k).map(String::from))?;
    let bad = || Error::new("error.badIndex");
    if idx_size.is_nan() || idx_size < 0.0 {
        return Err(bad());
    }
    let size = idx_size as u64;
    let mut bytes = vec![0u8; size as usize];
    JsReader {
        read: read_idx.clone(),
    }
    .read_at(0, &mut bytes)?;
    let sha = sha256_hex(&bytes);
    drop(bytes);
    let scans = Index::open(JsReader { read: read_idx }, size)?.scans;
    let mut m = JsReader { read: read_file };
    let (g, j) = graph_json(&scans, &mut m, (name, &sha), (&req.0, &req.1))?;
    let j = with_query_keys(&g, j, &req.0, q)?;
    Ok((g, j, req.0))
}

/// The optional keys of the query on the graph text `j`: `interpreta=1` and `rotture=1`, shared by the graph and its exports.
fn with_query_keys(g: &Graph, mut j: String, o: &Options, q: &Query) -> mzlab_core::Result<String> {
    // `interpreta=1` (and `ppm_perdita`) adds the key `interpretazione`, as `mzlab-cli msn-graph --interpreta` does
    if q.get("interpreta").is_some_and(|v| v == "1") {
        let ppm = match q.get("ppm_perdita").filter(|v| !v.is_empty()) {
            None => PPM_PERDITA,
            Some(v) => v
                .parse::<f64>()
                .ok()
                .filter(|p| p.is_finite() && *p > 0.0)
                .ok_or_else(|| Error::new("err.request.badNumber").with("name", "ppm_perdita"))?,
        };
        j = with_interpretation(&j, g, ppm);
    }
    // `rotture=1` (with `h_max`, `max_legami` and the window `ppm_struttura`) adds the key `rotture`, as `mzlab-cli msn-graph --rotture` does
    if q.get("rotture").is_some_and(|v| v == "1") {
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
            ppm: num("ppm_struttura", PPM_STRUCTURE)?,
            h_max: num("h_max", 2.0)?.min(4.0) as u8,
            max_bonds: num("max_legami", 2.0)?.clamp(1.0, 2.0) as u8,
        };
        j = mzlab_feat::chem::breaks::breaks_text(g, &j, o.smiles.as_deref(), p)?;
    }
    Ok(j)
}

/// Default window of the cleavages (`ppm_struttura`), ppm; the same as `mzlab-cli msn-graph --strutture`.
const PPM_STRUCTURE: f64 = 10.0;

/// The `msngrafo/1` of the file `nome` for the query string `query` (`formula`, `tol`, `ce`, `ppm_formula`, ..., `scelte`; `api/msn_grafo`).
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
    match q.get("strutture").filter(|n| !n.is_empty()) {
        None => Ok(j),
        Some(node) => {
            let ppm = match q.get("ppm_struttura").filter(|v| !v.is_empty()) {
                None => PPM_STRUCTURE,
                Some(v) => v
                    .parse::<f64>()
                    .ok()
                    .filter(|p| p.is_finite() && *p > 0.0)
                    .ok_or_else(|| {
                        js_err(Error::new("err.request.badNumber").with("name", "ppm_struttura"))
                    })?,
            };
            strutture_testo(&g, &j, o.smiles.as_deref(), (node, ppm)).map_err(js_err)
        }
    }
}

/// The spectra of the same graph as text (`api/msn_grafo/export`: `formato=csv|mgf|msp`, `pulito=1`, `nodo=<id>` besides the query of the graph).
/// The texts of the Flow steps come from the same route: `formato=massbank` (or `msp` with `libreria=1`) is the tree as a library (`msn.library_export`:
/// `livelli=all|ms2`, `annota=0|1`, `id`), `vista=modifications|nodes|peaks` the CSV table of the graph (`msn.table_export`).
#[wasm_bindgen]
pub fn msn_graph_export(
    read_idx: Function,
    idx_size: f64,
    read_file: Function,
    nome: &str,
    query: &str,
) -> Result<String, JsValue> {
    let q = Query::parse(query);
    let clean_flag = q.get("pulito").is_some_and(|v| v == "1");
    if let Some(view) = q.get("vista").filter(|v| !v.is_empty()) {
        let (_, j) = compute(read_idx, idx_size, read_file, nome, &q).map_err(js_err)?;
        let v = mzlab_core::json::parse(j.as_bytes()).map_err(js_err)?;
        return Ok(table_text(&v, view, clean_flag));
    }
    let format = q.get("formato").unwrap_or("");
    if format == "massbank" || q.get("libreria").is_some_and(|v| v == "1") {
        let (_, j) = compute(read_idx, idx_size, read_file, nome, &q).map_err(js_err)?;
        let v = mzlab_core::json::parse(j.as_bytes()).map_err(js_err)?;
        let get = |k: &str, d: &'static str| q.get(k).filter(|s| !s.is_empty()).unwrap_or(d);
        let format = if format == "massbank" {
            "massbank"
        } else {
            "msp"
        };
        let annotate = get("annota", "1") != "0";
        return library_text(
            &v,
            &j,
            (
                get("id", ""),
                format,
                get("livelli", "all"),
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

/// `api/msnlib/aggiungi`: the library text `libreria` ("" = none yet) with the entry of `molecola_id` for the graph of the query added;
/// `{"libreria": text, "doppione": bool}` (the text unchanged when the same entry was already there).
#[wasm_bindgen]
pub fn msnlib_add(
    read_idx: Function,
    idx_size: f64,
    read_file: Function,
    nome: &str,
    query: &str,
    libreria: &str,
    molecola_id: &str,
) -> Result<String, JsValue> {
    let q = Query::parse(query);
    let (g, j) = compute(read_idx, idx_size, read_file, nome, &q).map_err(js_err)?;
    let (l, duplicate) = aggiungi(libreria, molecola_id, &g, &j).map_err(js_err)?;
    let mut o = Out::default();
    o.raw("{\"libreria\":").str(&l).raw(",\"doppione\":");
    o.raw(if duplicate { "true}" } else { "false}" });
    Ok(o.0)
}

/// `api/msnlib/cerca`: the candidates of the library text `libreria` for the graph of the query (`msncerca/1`); the query carries the options of the graph and
/// those of the search (`ppm_prec`, `tol_mz`, `min_prove`, `top`, `delta_max`).
#[wasm_bindgen]
pub fn msnlib_search(
    read_idx: Function,
    idx_size: f64,
    read_file: Function,
    nome: &str,
    query: &str,
    libreria: &str,
) -> Result<String, JsValue> {
    let q = Query::parse(query);
    let o = opzioni_cerca(&|k| q.get(k).map(String::from)).map_err(js_err)?;
    let (_, j) = compute(read_idx, idx_size, read_file, nome, &q).map_err(js_err)?;
    cerca(libreria, &j, &o).map_err(js_err)
}
