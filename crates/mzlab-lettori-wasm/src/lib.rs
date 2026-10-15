//! The readers module of ONE family in the browser (the family is chosen by the Cargo feature the .wasm is built with): opens the files of a maker that the engine does not read and writes their mzML to the page piece by piece.
//! A dataset is described as in `crates/mzlab-wasm/src/nativi.rs`: names (relative to the dataset), sizes, `read(i, offset, length) -> Uint8Array`.
use std::io::Write;

use js_sys::Function;
use mzlab_core::idxread::ReadAt;
use mzlab_core::lettori::{FileSet, Limiti, Membro};
use mzlab_core::lettori_mzml;
use wasm_bindgen::prelude::*;

pub mod sessione;
use sessione::Sessioni;

/// Bytes sent to `sink` at a time.
const BLOCCO: usize = 64 << 10;

fn error_json(e: &mzlab_core::Error) -> JsValue {
    let params: Vec<String> = e
        .params
        .iter()
        .map(|(k, v)| format!("\"{}\":{:?}", k, v))
        .collect();
    JsValue::from_str(&format!(
        "{{\"error_key\":\"{}\",\"params\":{{{}}}}}",
        e.key,
        params.join(",")
    ))
}

/// Bytes fetched from JS at once for a small read: a reader like `opentfraw` asks for a few bytes at a time (4 for a length), and every call of `read` is a slice of a `File` read
/// synchronously (about a millisecond), so the member is read ahead in blocks.
const LETTURA: usize = 256 << 10;

/// Member `i` of a dataset, read through `read(i, offset, length) -> Uint8Array`; the small reads are served from the last block read.
struct JsMember {
    read: Function,
    i: u32,
    dim: u64,
    inizio: u64,
    blocco: Vec<u8>,
}

impl JsMember {
    fn new(read: Function, i: u32, dim: u64) -> Self {
        JsMember {
            read,
            i,
            dim,
            inizio: 0,
            blocco: Vec::new(),
        }
    }

    fn da_js(&self, off: u64, n: usize) -> mzlab_core::Result<js_sys::Uint8Array> {
        let bad = || mzlab_core::Error::new("error.badIndex");
        let v = self
            .read
            .call3(
                &JsValue::NULL,
                &self.i.into(),
                &(off as f64).into(),
                &(n as f64).into(),
            )
            .map_err(|_| bad())?;
        let a: js_sys::Uint8Array = v.dyn_into().map_err(|_| bad())?;
        if a.length() as usize != n {
            return Err(bad());
        }
        Ok(a)
    }
}

impl ReadAt for JsMember {
    fn read_at(&mut self, off: u64, buf: &mut [u8]) -> mzlab_core::Result<()> {
        let fine = off + buf.len() as u64;
        let dentro = off >= self.inizio && fine <= self.inizio + self.blocco.len() as u64;
        if !dentro {
            let n = LETTURA.min(self.dim.saturating_sub(off) as usize);
            if buf.len() >= LETTURA || n < buf.len() {
                self.da_js(off, buf.len())?.copy_to(buf); // big (or past the end, for JS to refuse): as asked
                return Ok(());
            }
            let a = self.da_js(off, n)?;
            self.blocco.resize(n, 0);
            a.copy_to(&mut self.blocco);
            self.inizio = off;
        }
        let da = (off - self.inizio) as usize;
        buf.copy_from_slice(&self.blocco[da..da + buf.len()]);
        Ok(())
    }
}

/// The members of a dataset as the page describes them.
struct Membri {
    names: Vec<String>,
    sizes: Vec<f64>,
    read: Function,
}

impl Membri {
    fn new(names: js_sys::Array, sizes: &[f64], read: Function) -> Result<Membri, JsValue> {
        let names: Vec<String> = names
            .iter()
            .map(|v| v.as_string().unwrap_or_default())
            .collect();
        if names.len() != sizes.len() || names.is_empty() {
            return Err(error_json(
                &mzlab_core::Error::new("err.file.notAccepted")
                    .with("name", names.first().cloned().unwrap_or_default()),
            ));
        }
        Ok(Membri {
            names,
            sizes: sizes.to_vec(),
            read,
        })
    }

    fn fileset(&self) -> FileSet {
        let membri = self
            .names
            .iter()
            .zip(&self.sizes)
            .enumerate()
            .map(|(i, (n, &s))| Membro {
                nome: n.clone(),
                dim: s as u64,
                dati: Box::new(JsMember::new(self.read.clone(), i as u32, s as u64)),
            })
            .collect();
        FileSet { membri }
    }
}

/// Sends what is written to JS in blocks: `sink(Uint8Array)`.
struct BlockWriter {
    sink: Function,
    buf: Vec<u8>,
}

impl BlockWriter {
    fn manda(&mut self) -> std::io::Result<()> {
        if self.buf.is_empty() {
            return Ok(());
        }
        let a = js_sys::Uint8Array::from(&self.buf[..]);
        self.sink
            .call1(&JsValue::NULL, &a)
            .map_err(|_| std::io::Error::other("sink failed"))?;
        self.buf.clear();
        Ok(())
    }
}

impl Write for BlockWriter {
    fn write(&mut self, data: &[u8]) -> std::io::Result<usize> {
        self.buf.extend_from_slice(data);
        if self.buf.len() >= BLOCCO {
            self.manda()?;
        }
        Ok(data.len())
    }
    fn flush(&mut self) -> std::io::Result<()> {
        self.manda()
    }
}

/// The family of this module (`agilent`, `sciex`, `thermo`, ...).
#[wasm_bindgen]
pub fn famiglia() -> String {
    mzlab_lettori::famiglia().into()
}

/// The extensions the module accepts (JSON array, lower case with the dot).
#[wasm_bindgen]
pub fn estensioni() -> String {
    serde_json::to_string(&mzlab_lettori::estensioni()).unwrap_or_else(|_| "[]".into())
}

/// What the module sees in a dataset: `{formato, campioni:[{id,name,scans}], motivi:[...]}`. The `motivi` say what the engine's reader would not give: `"srm"` (SRM only
/// as scans), `"pda"` (UV traces). The module links no reader of the engine, so it cannot say whether the engine would have opened the file: the page knows.
#[wasm_bindgen]
pub fn probe(names: js_sys::Array, sizes: &[f64], read: Function) -> Result<String, JsValue> {
    let m = Membri::new(names, sizes, read)?;
    let mut motivi: Vec<&str> = Vec::new();
    let mut l = mzlab_lettori::apri(m.fileset(), &Limiti::sito()).map_err(|e| error_json(&e))?;
    let campioni: Vec<serde_json::Value> = l
        .campioni()
        .iter()
        .map(|c| serde_json::json!({ "id": c.id, "name": c.nome, "scans": c.scansioni }))
        .collect();
    for x in mzlab_lettori::motivi(&mut *l, 0) {
        if !motivi.contains(&x) {
            motivi.push(x);
        }
    }
    Ok(
        serde_json::json!({ "formato": l.formato().id(), "campioni": campioni, "motivi": motivi })
            .to_string(),
    )
}

/// Sample `c` of a dataset as mzML, sent to `sink(Uint8Array)` in pieces of about 64 KB; `progress(done, total)` counts the spectra written.
#[wasm_bindgen]
pub fn to_mzml(
    names: js_sys::Array,
    sizes: &[f64],
    read: Function,
    c: u32,
    sink: Function,
    progress: Function,
) -> Result<(), JsValue> {
    let m = Membri::new(names, sizes, read)?;
    let mut l = mzlab_lettori::apri(m.fileset(), &Limiti::sito()).map_err(|e| error_json(&e))?;
    let mut out = BlockWriter {
        sink,
        buf: Vec::new(),
    };
    let mut tell = |done: usize, total: usize| {
        let _ = progress.call2(
            &JsValue::NULL,
            &(done as f64).into(),
            &(total as f64).into(),
        );
    };
    lettori_mzml::scrivi(&mut *l, c as usize, &mut out, &mut tell).map_err(|e| error_json(&e))?;
    out.flush().map_err(|_| {
        error_json(&mzlab_core::Error::new("err.file.notAccepted").with("detail", "write"))
    })
}

// ---- session interface (for the external reader of the engine): a reader stays open under a handle ----
thread_local! { static SESSIONI: std::cell::RefCell<Sessioni> = std::cell::RefCell::new(Sessioni::default()); }

fn con<T>(f: impl FnOnce(&mut Sessioni) -> mzlab_core::Result<T>) -> Result<T, JsValue> {
    SESSIONI
        .with(|s| f(&mut s.borrow_mut()))
        .map_err(|e| error_json(&e))
}

/// Opens a dataset and keeps it under a handle (`chiudi` frees it); `campioni(h)` says what it holds.
#[wasm_bindgen]
pub fn apri(names: js_sys::Array, sizes: &[f64], read: Function) -> Result<u32, JsValue> {
    let m = Membri::new(names, sizes, read)?;
    // as it is, without the SRM traces of `mzlab_lettori::apri`: the engine reads the module through `LettoreEsterno` and decides itself what it does with them
    let l = mzlab_lettori::apri_nudo(m.fileset(), &Limiti::sito()).map_err(|e| error_json(&e))?;
    con(|s| Ok(s.inserisci(l)))
}
#[wasm_bindgen]
pub fn campioni(h: u32) -> Result<String, JsValue> {
    con(|s| s.campioni(h))
}
#[wasm_bindgen]
pub fn indice(h: u32, c: u32) -> Result<String, JsValue> {
    con(|s| s.indice(h, c as usize))
}
/// `[mz: Float64Array, intensity: Float64Array, meta: JSON]`.
#[wasm_bindgen]
pub fn scansione(h: u32, c: u32, k: u32) -> Result<js_sys::Array, JsValue> {
    let s = con(|s| s.scansione(h, c as usize, k as usize))?;
    let a = js_sys::Array::new();
    a.push(&js_sys::Float64Array::from(&s.mz[..]));
    a.push(&js_sys::Float64Array::from(&s.intensita[..]));
    a.push(&JsValue::from_str(&s.meta));
    Ok(a)
}
#[wasm_bindgen]
pub fn intestazione(h: u32, c: u32, k: u32) -> Result<String, JsValue> {
    con(|s| s.intestazione(h, c as usize, k as usize))
}
#[wasm_bindgen]
pub fn cromatogrammi(h: u32, c: u32) -> Result<String, JsValue> {
    con(|s| s.cromatogrammi(h, c as usize))
}
#[wasm_bindgen]
pub fn cromatogramma(h: u32, c: u32, id: &str) -> Result<String, JsValue> {
    con(|s| s.cromatogramma(h, c as usize, id))
}
#[wasm_bindgen]
pub fn metodo(h: u32, c: u32) -> Result<String, JsValue> {
    con(|s| s.metodo(h, c as usize))
}
/// The header of the mzML of the sample as JSON (`null` when the reader has none).
#[wasm_bindgen]
pub fn testata(h: u32, c: u32) -> Result<String, JsValue> {
    con(|s| s.testata(h, c as usize))
}
#[wasm_bindgen]
pub fn chiudi(h: u32) {
    SESSIONI.with(|s| s.borrow_mut().chiudi(h));
}
