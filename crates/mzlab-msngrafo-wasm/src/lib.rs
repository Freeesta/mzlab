//! wasm-bindgen bridge of `mzlab_core::msngrafo`: the `.mzidx` of an infusion and its mzML (both read through `read(offset, length)`) give the
//! `msngrafo/1` of a request, with the same functions as `mzlab-cli msn-graph` (same bytes). A `.wasm` of its own, loaded by the worker of the
//! engine on the first request of the graph; `strutture=<id>` (and `ppm_struttura`) adds the cleavages of the SMILES that explain a node. Memory: the index is read once for its SHA-256 and once to open it; the output is one JSON string,
//! no offsets (`docs/agenti/memoria.md`). Errors are JSON `{"error_key", "params"}` strings.
use js_sys::{Function, Uint8Array};
use mzlab_core::budget::sha256_hex;
use mzlab_core::error::Error;
use mzlab_core::idxread::{Index, ReadAt};
use mzlab_core::json::Out;
use mzlab_core::msngrafo::{
    aggiungi, cerca, da_richiesta, da_richiesta_export, esporta, grafo_json, opzioni_cerca,
};
use mzlab_core::msngrafo::{Grafo, Opzioni};
use mzlab_core::routes::Query;
use mzlab_feat::chem::strutture::strutture_testo;
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
    let mut o = Out::default();
    o.raw("{\"error_key\":").str(e.key).raw(",\"params\":{");
    for (i, (k, v)) in e.params.iter().enumerate() {
        o.raw(if i > 0 { "," } else { "" }).str(k).raw(":").str(v);
    }
    o.raw("}}");
    JsValue::from_str(&o.0)
}

/// `mzlab-msngrafo/1`: the page checks that the loaded `.wasm` is the one it was built with.
#[wasm_bindgen]
pub fn version() -> String {
    format!("mzlab-msngrafo/1 {}", env!("CARGO_PKG_VERSION"))
}

fn calcola(
    read_idx: Function,
    idx_size: f64,
    read_file: Function,
    nome: &str,
    q: &Query,
) -> mzlab_core::Result<(Grafo, String)> {
    let (g, j, _) = calcola_con(read_idx, idx_size, read_file, nome, q)?;
    Ok((g, j))
}

fn calcola_con(
    read_idx: Function,
    idx_size: f64,
    read_file: Function,
    nome: &str,
    q: &Query,
) -> mzlab_core::Result<(Grafo, String, Opzioni)> {
    let req = da_richiesta(&|k| q.get(k).map(String::from))?;
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
    let (g, j) = grafo_json(&scans, &mut m, (nome, &sha), (&req.0, &req.1))?;
    Ok((g, j, req.0))
}

/// Default window of the cleavages (`ppm_struttura`), ppm; the same as `mzlab-cli msn-graph --strutture`.
const PPM_STRUTTURA: f64 = 10.0;

/// The `msngrafo/1` of the file `nome` for the query string `query` (`formula`, `tol`, `ce`, `ppm_formula`, ..., `scelte`; `api/msn_grafo`).
/// `read_idx` reads the `.mzidx` (`idx_size` bytes) and `read_file` the mzML the index points into.
#[wasm_bindgen]
pub fn msn_grafo(
    read_idx: Function,
    idx_size: f64,
    read_file: Function,
    nome: &str,
    query: &str,
) -> Result<String, JsValue> {
    let q = Query::parse(query);
    let (g, j, o) = calcola_con(read_idx, idx_size, read_file, nome, &q).map_err(js_err)?;
    match q.get("strutture").filter(|n| !n.is_empty()) {
        None => Ok(j),
        Some(nodo) => {
            let ppm = match q.get("ppm_struttura").filter(|v| !v.is_empty()) {
                None => PPM_STRUTTURA,
                Some(v) => v
                    .parse::<f64>()
                    .ok()
                    .filter(|p| p.is_finite() && *p > 0.0)
                    .ok_or_else(|| {
                        js_err(Error::new("err.request.badNumber").with("name", "ppm_struttura"))
                    })?,
            };
            strutture_testo(&g, &j, o.smiles.as_deref(), (nodo, ppm)).map_err(js_err)
        }
    }
}

/// The spectra of the same graph as text (`api/msn_grafo/export`: `formato=csv|mgf|msp`, `pulito=1`, `nodo=<id>` besides the query of the graph).
#[wasm_bindgen]
pub fn msn_grafo_export(
    read_idx: Function,
    idx_size: f64,
    read_file: Function,
    nome: &str,
    query: &str,
) -> Result<String, JsValue> {
    let q = Query::parse(query);
    let (f, pulito, nodo) = da_richiesta_export(&|k| q.get(k).map(String::from)).map_err(js_err)?;
    let (g, _) = calcola(read_idx, idx_size, read_file, nome, &q).map_err(js_err)?;
    Ok(esporta(&g, f, pulito, nodo.as_deref()))
}

/// `api/msnlib/aggiungi`: the library text `libreria` ("" = none yet) with the entry of `molecola_id` for the graph of the query added;
/// `{"libreria": text, "doppione": bool}` (the text unchanged when the same entry was already there).
#[wasm_bindgen]
pub fn msnlib_aggiungi(
    read_idx: Function,
    idx_size: f64,
    read_file: Function,
    nome: &str,
    query: &str,
    libreria: &str,
    molecola_id: &str,
) -> Result<String, JsValue> {
    let q = Query::parse(query);
    let (g, j) = calcola(read_idx, idx_size, read_file, nome, &q).map_err(js_err)?;
    let (l, doppione) = aggiungi(libreria, molecola_id, &g, &j).map_err(js_err)?;
    let mut o = Out::default();
    o.raw("{\"libreria\":").str(&l).raw(",\"doppione\":");
    o.raw(if doppione { "true}" } else { "false}" });
    Ok(o.0)
}

/// `api/msnlib/cerca`: the candidates of the library text `libreria` for the graph of the query (`msncerca/1`); the query carries the options of the graph and
/// those of the search (`ppm_prec`, `tol_mz`, `min_prove`, `top`, `delta_max`).
#[wasm_bindgen]
pub fn msnlib_cerca(
    read_idx: Function,
    idx_size: f64,
    read_file: Function,
    nome: &str,
    query: &str,
    libreria: &str,
) -> Result<String, JsValue> {
    let q = Query::parse(query);
    let o = opzioni_cerca(&|k| q.get(k).map(String::from)).map_err(js_err)?;
    let (_, j) = calcola(read_idx, idx_size, read_file, nome, &q).map_err(js_err)?;
    cerca(libreria, &j, &o).map_err(js_err)
}
