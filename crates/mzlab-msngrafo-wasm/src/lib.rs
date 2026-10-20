//! wasm-bindgen bridge of `mzlab_core::msngrafo`: the `.mzidx` of an infusion and its mzML (both read through `read(offset, length)`) give the
//! `msngrafo/1` of a request, with the same functions as `mzlab-cli msn-graph` (same bytes). A `.wasm` of its own, loaded by the worker of the
//! engine on the first request of the graph. Memory: the index is read once for its SHA-256 and once to open it; the output is one JSON string,
//! no offsets (`docs/agenti/memoria.md`). Errors are JSON `{"error_key", "params"}` strings.
use js_sys::{Function, Uint8Array};
use mzlab_core::budget::sha256_hex;
use mzlab_core::error::Error;
use mzlab_core::idxread::{Index, ReadAt};
use mzlab_core::json::Out;
use mzlab_core::msngrafo::{da_richiesta, grafo_json};
use mzlab_core::routes::Query;
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

fn grafo(
    read_idx: Function,
    idx_size: f64,
    read_file: Function,
    nome: &str,
    query: &str,
) -> mzlab_core::Result<String> {
    let q = Query::parse(query);
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
    Ok(grafo_json(&scans, &mut m, (nome, &sha), (&req.0, &req.1))?.1)
}

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
    grafo(read_idx, idx_size, read_file, nome, query).map_err(js_err)
}
