//! The readers module of ONE family in the browser (the family is chosen by the Cargo feature the .wasm is built with): opens the files of a maker that the engine does not read and writes their mzML to the page piece by piece.
//! A dataset is described as in `crates/mzlab-wasm/src/nativi.rs`: names (relative to the dataset), sizes, `read(i, offset, length) -> Uint8Array`.
use std::io::Write;

use js_sys::Function;
use mzlab_core::lettori::{FileSet, Limiti, Membro};
use mzlab_core::lettori_mzml;
use wasm_bindgen::prelude::*;

mod json_testo;
pub mod sessione;
use sessione::Sessioni;

/// Bytes sent to `sink` at a time.
const BLOCCO: usize = 64 << 10;

fn error_text(e: &mzlab_core::Error) -> String {
    e.to_json()
}

fn error_json(e: &mzlab_core::Error) -> JsValue {
    JsValue::from_str(&error_text(e))
}

/// Bytes fetched from JS at once for a small read: a reader like `opentfraw` asks for a few bytes at a time (4 for a length), and every call of `read` is a slice of a `File` read
/// synchronously (about a millisecond), so the member is read ahead in blocks.
const LETTURA: usize = 256 << 10;

/// `read(i, offset, length) -> Uint8Array`: `n` bytes of member `i` at `off`.
fn da_js(read: &Function, i: u32, off: u64, n: usize) -> mzlab_core::Result<js_sys::Uint8Array> {
    let bad = || mzlab_core::Error::new("error.badIndex");
    let v = read
        .call3(
            &JsValue::NULL,
            &i.into(),
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

/// Blocks kept by the `thermo` module: the reader alternates between the packets of the scans, the trailer records and the index, and a block thrown away by a far read is read
/// again at the next (`raw_tempi`: with 3 the table of a 32 MB Orbitrap file fetches 26 MB instead of 59, the 82 MB profile file 71 instead of 145). The other families keep
/// the one block they had: their gain is not measured and their `.wasm` limits leave no room for the cache.
#[cfg(feature = "thermo")]
const BLOCCHI: usize = 3;

/// Member `i` of a dataset, read through `read` and a cache of `BLOCCHI` blocks of `LETTURA` bytes (the small reads are served from them).
#[cfg(feature = "thermo")]
fn membro_js(
    read: Function,
    i: u32,
    dim: u64,
) -> mzlab_core::blocchi::BlockCache<impl FnMut(u64, &mut [u8]) -> mzlab_core::Result<()>> {
    let fetch = move |off: u64, buf: &mut [u8]| -> mzlab_core::Result<()> {
        da_js(&read, i, off, buf.len())?.copy_to(buf);
        Ok(())
    };
    mzlab_core::blocchi::BlockCache::new(fetch, dim, LETTURA, BLOCCHI)
}

/// Member `i` of a dataset, read through `read(i, offset, length) -> Uint8Array`; the small reads are served from the last block read.
#[cfg(not(feature = "thermo"))]
struct JsMember {
    read: Function,
    i: u32,
    dim: u64,
    inizio: u64,
    blocco: Vec<u8>,
}

#[cfg(not(feature = "thermo"))]
fn membro_js(read: Function, i: u32, dim: u64) -> JsMember {
    JsMember {
        read,
        i,
        dim,
        inizio: 0,
        blocco: Vec::new(),
    }
}

#[cfg(not(feature = "thermo"))]
impl mzlab_core::idxread::ReadAt for JsMember {
    fn read_at(&mut self, off: u64, buf: &mut [u8]) -> mzlab_core::Result<()> {
        let fine = off + buf.len() as u64;
        let dentro = off >= self.inizio && fine <= self.inizio + self.blocco.len() as u64;
        if !dentro {
            let n = self.dim.saturating_sub(off).min(LETTURA as u64) as usize;
            if buf.len() >= LETTURA || n < buf.len() {
                da_js(&self.read, self.i, off, buf.len())?.copy_to(buf); // big (or past the end, for JS to refuse): as asked
                return Ok(());
            }
            let a = da_js(&self.read, self.i, off, n)?;
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
                dati: Box::new(membro_js(self.read.clone(), i as u32, s as u64)),
                sha1: None,
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
    let mut o = mzlab_core::json::Out::default();
    o.raw("[");
    for (i, e) in mzlab_lettori::estensioni().into_iter().enumerate() {
        o.raw(if i > 0 { "," } else { "" }).str(e);
    }
    o.raw("]");
    o.0
}

/// What the module sees in a dataset: `{formato, campioni:[{id,name,scans}], motivi:[...]}`. The `motivi` say what the engine's reader would not give: `"srm"` (SRM only
/// as scans), `"pda"` (UV traces), `"variante"` (a file whose points the engine would misread: an Agilent `.D` QqQ MRM or Q-TOF profile or flight times, a SCIEX file of the 3200 reader). The module links no reader of the engine, so it cannot say whether the engine would have opened the file: the page knows.
#[wasm_bindgen]
pub fn probe(names: js_sys::Array, sizes: &[f64], read: Function) -> Result<String, JsValue> {
    let m = Membri::new(names, sizes, read)?;
    #[cfg_attr(not(any(feature = "agilent", feature = "tutte")), allow(unused_mut))]
    let mut motivi: Vec<&str> = Vec::new();
    #[cfg(any(feature = "agilent", feature = "tutte"))]
    motivi.extend(mzlab_lettori::motivi_insieme(&mut m.fileset()));
    let mut l = match mzlab_lettori::apri(m.fileset(), &Limiti::sito()) {
        Ok(l) => l,
        // a SCIEX file the module itself refuses (a TOF, several periods, an unknown scan type) is a variant too: the engine would read it as a 3200 and be wrong
        #[cfg(feature = "sciex")]
        Err(e) if e.key == "err.file.wiff" => {
            return Ok(r#"{"formato":"sciex-wiff","campioni":[],"motivi":["variante"]}"#.into())
        }
        Err(e) => return Err(error_json(&e)),
    };
    for x in mzlab_lettori::motivi(&mut *l, 0) {
        if !motivi.contains(&x) {
            motivi.push(x);
        }
    }
    let mut o = mzlab_core::json::Out::default();
    o.raw("{\"formato\":")
        .str(l.formato().id())
        .raw(",\"campioni\":");
    json_testo::campioni_json(&mut o, l.campioni());
    o.raw(",\"motivi\":[");
    for (i, m) in motivi.iter().enumerate() {
        o.raw(if i > 0 { "," } else { "" }).str(m);
    }
    o.raw("]}");
    Ok(o.0)
}

/// The reader of `fs` for the mzML of sample `c`: a SCIEX pair reads the heads of the blocks of that injection only (`apri_solo`, as the engine does), any other set opens as `apri`.
fn apri_campione(
    #[cfg_attr(not(any(feature = "sciex", feature = "tutte")), allow(unused_mut))] mut fs: FileSet,
    c: usize,
) -> mzlab_core::Result<Box<dyn mzlab_core::lettori::Lettore>> {
    #[cfg(any(feature = "sciex", feature = "tutte"))]
    if mzlab_lettori::riconosci(&mut fs).is_some_and(|v| v.id == mzlab_core::wiff::lettore::VOCE.id)
    {
        let l = mzlab_core::wiff::lettore::apri_solo(fs, &Limiti::sito(), Some(c))?;
        return Ok(Box::new(mzlab_lettori::srm::SrmDaScansioni::new(l)));
    }
    let _ = c;
    mzlab_lettori::apri(fs, &Limiti::sito())
}

/// Sample `c` of a dataset as mzML, sent to `sink(Uint8Array)` in pieces of about 64 KB; `progress(done, total)` counts the spectra written. Gives the spectra a truncated file lacks (`Lettore::mancanti`).
#[wasm_bindgen]
pub fn to_mzml(
    names: js_sys::Array,
    sizes: &[f64],
    read: Function,
    c: u32,
    sink: Function,
    progress: Function,
) -> Result<f64, JsValue> {
    let m = Membri::new(names, sizes, read)?;
    let mut l = apri_campione(m.fileset(), c as usize).map_err(|e| error_json(&e))?;
    let lacking = l.mancanti(c as usize);
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
    // indexed mzML with no processing steps: the form `mzlab-cli convert` writes (`flow_base.rs` `write_mzml_with`), byte for byte
    let conv = lettori_mzml::Conversione {
        steps: Some(&[]),
        ..lettori_mzml::Conversione::default()
    };
    conv.scrivi(&mut *l, c as usize, &mut out, &mut tell)
        .map_err(|e| error_json(&e))?;
    out.flush().map_err(|_| {
        error_json(&mzlab_core::Error::new("err.file.notAccepted").with("detail", "write"))
    })?;
    Ok(lacking as f64)
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
    apri_membri(Membri::new(names, sizes, read)?.fileset())
}
/// `apri` with the SHA-1 of each member when the page took it while copying the file (empty text: unknown): the header of the mzML uses it instead of
/// reading the file again. Only the `thermo` module has it (the other readers take their SHA-1 themselves).
#[cfg(feature = "thermo")]
#[wasm_bindgen]
pub fn apri_con_sha1(
    names: js_sys::Array,
    sizes: &[f64],
    read: Function,
    sha1: js_sys::Array,
) -> Result<u32, JsValue> {
    let mut fs = Membri::new(names, sizes, read)?.fileset();
    for (x, v) in fs.membri.iter_mut().zip(sha1.iter()) {
        x.sha1 = v.as_string().filter(|t| !t.is_empty());
    }
    apri_membri(fs)
}
fn apri_membri(fs: FileSet) -> Result<u32, JsValue> {
    // as it is, without the SRM traces of `mzlab_lettori::apri`: the engine reads the module through `LettoreEsterno` and decides itself what it does with them
    let l = mzlab_lettori::apri_nudo(fs, &Limiti::sito()).map_err(|e| error_json(&e))?;
    con(|s| Ok(s.inserisci(l)))
}
#[wasm_bindgen]
pub fn campioni(h: u32) -> Result<String, JsValue> {
    con(|s| s.campioni(h))
}
/// The version of the binary transport of the index and the scans (`mzlab_core::trasporto::PROTOCOLLO`): the engine refuses a module with another.
#[wasm_bindgen]
pub fn protocollo() -> u32 {
    mzlab_core::trasporto::PROTOCOLLO
}
/// The index of the sample as one `Uint8Array` (`mzlab_core::trasporto::encode_indice`).
#[wasm_bindgen]
pub fn indice(h: u32, c: u32) -> Result<Vec<u8>, JsValue> {
    con(|s| s.indice(h, c as usize))
}
/// A scan as one `Uint8Array` (`mzlab_core::trasporto::encode_scansione`).
#[wasm_bindgen]
pub fn scansione(h: u32, c: u32, k: u32) -> Result<Vec<u8>, JsValue> {
    con(|s| s.scansione(h, c as usize, k as usize))
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
/// The spectra of the sample that a truncated file declares but does not hold.
#[wasm_bindgen]
pub fn mancanti(h: u32, c: u32) -> Result<u32, JsValue> {
    con(|s| s.mancanti(h, c as usize)).map(|n| n.min(u32::MAX as usize) as u32)
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
