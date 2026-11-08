//! The readers module of ONE family in the browser (the family is chosen by the Cargo feature the .wasm is built with): opens the files of a maker that the engine does not read and writes their mzML to the page piece by piece.
//! A dataset is described as in `crates/mzlab-wasm/src/nativi.rs`: names (relative to the dataset), sizes, `read(i, offset, length) -> Uint8Array`.
use std::io::Write;

use js_sys::Function;
use mzlab_core::lettori::{FileSet, Limiti, Membro};
use mzlab_core::lettori_mzml;
use wasm_bindgen::prelude::*;

mod json_testo;
pub mod sessione;
use sessione::Sessions;

/// Bytes sent to `sink` at a time (each call is an OPFS write in the page: 1 MiB blocks cost a quarter of 64 KiB ones per byte).
const BLOCK: usize = 1 << 20;

fn error_text(e: &mzlab_core::Error) -> String {
    e.to_json()
}

fn error_json(e: &mzlab_core::Error) -> JsValue {
    JsValue::from_str(&error_text(e))
}

/// Bytes fetched from JS at once for a small read: a reader like `opentfraw` asks for a few bytes at a time (4 for a length), and every call of `read` is a slice of a `File` read
/// synchronously (about a millisecond), so the member is read ahead in blocks.
const READ_AHEAD: usize = 256 << 10;

/// `read(i, offset, length) -> Uint8Array`: `n` bytes of member `i` at `off`.
fn from_js(read: &Function, i: u32, off: u64, n: usize) -> mzlab_core::Result<js_sys::Uint8Array> {
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

/// `read.into(i, offset, view)` of the page (`rust-nativo.js`, over OPFS handles): the bytes go from the handle straight into this module's memory. The view (as `lend` of
/// `crates/mzlab-wasm/src/nativi.rs`, the other way) is valid during the call only: JS fills it at once, never keeps it, makes no call into the module meanwhile, and answers the bytes it put.
fn into_of(read: &Function) -> Option<Function> {
    js_sys::Reflect::get(read, &JsValue::from_str("into"))
        .ok()?
        .dyn_into()
        .ok()
}

/// `buf.len()` bytes of member `i` at `off` into `buf`: through `into` when the page has it, otherwise `read` copied.
fn read_member(
    read: &Function,
    into: Option<&Function>,
    i: u32,
    off: u64,
    buf: &mut [u8],
) -> mzlab_core::Result<()> {
    let bad = || mzlab_core::Error::new("error.badIndex");
    let Some(into) = into else {
        from_js(read, i, off, buf.len())?.copy_to(buf);
        return Ok(());
    };
    let mem: js_sys::WebAssembly::Memory = wasm_bindgen::memory().unchecked_into();
    let (at, n) = u32::try_from(buf.as_mut_ptr() as usize)
        .ok()
        .zip(u32::try_from(buf.len()).ok())
        .ok_or_else(bad)?;
    let v = js_sys::Uint8Array::new_with_byte_offset_and_length(&mem.buffer(), at, n);
    let k = into
        .call3(&JsValue::NULL, &i.into(), &(off as f64).into(), &v)
        .map_err(|_| bad())?;
    if k.as_f64() != Some(f64::from(n)) {
        return Err(bad());
    }
    Ok(())
}

/// Blocks kept by the `thermo` module: the reader alternates between the packets of the scans, the trailer records and the index, and a block thrown away by a far read is read
/// again at the next (`raw_tempi`: with 3 the table of a 32 MB Orbitrap file fetches 26 MB instead of 59, the 82 MB profile file 71 instead of 145). The other families keep
/// the one block they had: their gain is not measured and their `.wasm` limits leave no room for the cache.
#[cfg(feature = "thermo")]
const BLOCKS: usize = 3;

/// The longest block of a run of reads in order (the profiles of the scans, one after the other): an OPFS sync-handle read costs about 0.25 ms per call
/// plus 0.085 ms per 64 KiB, so 4 MiB blocks read the scans of a 422 MB `.raw` in about 100 calls instead of 1600 of 256 KiB.
#[cfg(feature = "thermo")]
const READ_AHEAD_MAX: usize = 4 << 20;

/// Member `i` of a dataset, read through `read` and a cache of `BLOCKS` blocks of `READ_AHEAD` bytes, growing to `READ_AHEAD_MAX` along reads in order (the small reads are served from them).
#[cfg(feature = "thermo")]
fn member_js(
    read: Function,
    i: u32,
    dim: u64,
) -> mzlab_core::blocchi::BlockCache<impl FnMut(u64, &mut [u8]) -> mzlab_core::Result<()>> {
    let into = into_of(&read);
    let fetch = move |off: u64, buf: &mut [u8]| read_member(&read, into.as_ref(), i, off, buf);
    mzlab_core::blocchi::BlockCache::new(fetch, dim, READ_AHEAD, BLOCKS).read_ahead(READ_AHEAD_MAX)
}

/// Member `i` of a dataset, read through `read(i, offset, length) -> Uint8Array`; the small reads are served from the last block read.
#[cfg(not(feature = "thermo"))]
struct JsMember {
    read: Function,
    into: Option<Function>,
    i: u32,
    dim: u64,
    inizio: u64,
    blocco: Vec<u8>,
}

#[cfg(not(feature = "thermo"))]
fn member_js(read: Function, i: u32, dim: u64) -> JsMember {
    JsMember {
        into: into_of(&read),
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
            let n = self.dim.saturating_sub(off).min(READ_AHEAD as u64) as usize;
            if buf.len() >= READ_AHEAD || n < buf.len() {
                // big (or past the end, for JS to refuse): as asked
                return read_member(&self.read, self.into.as_ref(), self.i, off, buf);
            }
            self.blocco.resize(n, 0);
            // a failed read leaves no block: its bytes may be half written
            if let Err(e) = read_member(
                &self.read,
                self.into.as_ref(),
                self.i,
                off,
                &mut self.blocco,
            ) {
                self.blocco.clear();
                return Err(e);
            }
            self.inizio = off;
        }
        let da = (off - self.inizio) as usize;
        buf.copy_from_slice(&self.blocco[da..da + buf.len()]);
        Ok(())
    }
}

/// The members of a dataset as the page describes them.
struct Members {
    names: Vec<String>,
    sizes: Vec<f64>,
    read: Function,
}

impl Members {
    fn new(names: js_sys::Array, sizes: &[f64], read: Function) -> Result<Members, JsValue> {
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
        Ok(Members {
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
                dati: Box::new(member_js(self.read.clone(), i as u32, s as u64)),
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
    fn send(&mut self) -> std::io::Result<()> {
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
        if self.buf.len() >= BLOCK {
            self.send()?;
        }
        Ok(data.len())
    }
    fn flush(&mut self) -> std::io::Result<()> {
        self.send()
    }
}

/// The family of this module (`agilent`, `sciex`, `thermo`, ...).
#[wasm_bindgen]
pub fn family() -> String {
    mzlab_lettori::famiglia().into()
}

/// What the module sees in a dataset: `{formato, campioni:[{id,name,scans}], motivi:[...]}` (a SCIEX pair counts the scans of sample 0 only, the others say 0). The `motivi` say what the engine's reader would not give: `"srm"` (SRM only
/// as scans), `"pda"` (UV traces), `"variante"` (a file whose points the engine would misread: an Agilent `.D` QqQ MRM or Q-TOF profile or flight times, a SCIEX file of the 3200 reader). The module links no reader of the engine, so it cannot say whether the engine would have opened the file: the page knows.
#[wasm_bindgen]
pub fn probe(names: js_sys::Array, sizes: &[f64], read: Function) -> Result<String, JsValue> {
    let m = Members::new(names, sizes, read)?;
    #[cfg_attr(not(any(feature = "agilent", feature = "tutte")), allow(unused_mut))]
    let mut motivi: Vec<&str> = Vec::new();
    #[cfg(any(feature = "agilent", feature = "tutte"))]
    motivi.extend(mzlab_lettori::reasons_together(&mut m.fileset()));
    let mut l = match open_probe(m.fileset()) {
        Ok(l) => l,
        // a SCIEX file the module itself refuses (a TOF, several periods, an unknown scan type) is a variant too: the engine would read it as a 3200 and be wrong
        #[cfg(feature = "sciex")]
        Err(e)
            if matches!(
                e.key,
                "err.file.wiff"
                    | "err.file.wiffScanType"
                    | "err.file.wiffPeriods"
                    | "err.file.wiffNeeds2"
            ) =>
        {
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
        .str(l.format().id())
        .raw(",\"campioni\":");
    json_testo::samples_json(&mut o, l.samples());
    o.raw(",\"motivi\":[");
    for (i, m) in motivi.iter().enumerate() {
        o.raw(if i > 0 { "," } else { "" }).str(m);
    }
    o.raw("]}");
    Ok(o.0)
}

/// The reader of `fs` for `probe`: a SCIEX pair builds the index of sample 0 only (`apri_solo`), the method of the others is enough for `motivi`;
/// any other set opens as `apri`.
fn open_probe(
    #[cfg_attr(not(any(feature = "sciex", feature = "tutte")), allow(unused_mut))] mut fs: FileSet,
) -> mzlab_core::Result<Box<dyn mzlab_core::lettori::Lettore>> {
    #[cfg(any(feature = "sciex", feature = "tutte"))]
    if mzlab_lettori::riconosci(&mut fs).is_some_and(|v| v.id == mzlab_core::wiff::lettore::VOCE.id)
    {
        return mzlab_core::wiff::lettore::apri_solo(fs, &Limiti::sito(), Some(0));
    }
    mzlab_lettori::apri(fs, &Limiti::sito())
}

/// The reader of `fs` for the mzML of sample `c`: a SCIEX pair reads the heads of the blocks of that injection only (`apri_solo`, as the engine does), any other set opens as `apri`.
fn open_sample(
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

/// Sample `c` of a dataset as mzML, sent to `sink(Uint8Array)` in pieces of about 64 KB; `progress(done, total)` counts the spectra written. Gives the spectra a truncated file lacks (`Lettore::missing`).
/// Indexed mzML with no processing steps and the optical matrix when the sample has one (the page reads UV spectra and the PDA from this file only):
/// the form `mzlab-cli convert --set conv.dad=true` writes (`flow_base.rs` `write_mzml_with`), byte for byte.
#[wasm_bindgen]
pub fn to_mzml(
    names: js_sys::Array,
    sizes: &[f64],
    read: Function,
    c: u32,
    sink: Function,
    progress: Function,
) -> Result<f64, JsValue> {
    to_mzml_with(names, sizes, read, c, true, "standard", sink, progress)
}

/// `to_mzml` with the optical matrix only when `dad` and the compression of «Download .mzML» (`mzmlwrite::opzioni_compressione`): with `dad` false the file of
/// `mzlab-cli convert` with the same `conv.numpress`, byte for byte (the download of a `.wiff` sample, also one the engine reads: the engine leaves out what `estesi` writes).
#[wasm_bindgen]
#[allow(clippy::too_many_arguments)]
pub fn to_mzml_with(
    names: js_sys::Array,
    sizes: &[f64],
    read: Function,
    c: u32,
    dad: bool,
    compression: &str,
    sink: Function,
    progress: Function,
) -> Result<f64, JsValue> {
    let opz = mzlab_core::mzmlwrite::opzioni_compressione(compression)
        .ok_or_else(|| error_json(&mzlab_core::Error::new("err.file.mzmlWrite")))?;
    let m = Members::new(names, sizes, read)?;
    let mut l = open_sample(m.fileset(), c as usize).map_err(|e| error_json(&e))?;
    let lacking = l.missing(c as usize);
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
    let conv = lettori_mzml::Conversione {
        steps: Some(&[]),
        opz: mzlab_core::mzmlwrite::Opzioni { dad, ..opz },
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
thread_local! { static SESSIONS: std::cell::RefCell<Sessions> = std::cell::RefCell::new(Sessions::default()); }

fn with_sessions<T>(f: impl FnOnce(&mut Sessions) -> mzlab_core::Result<T>) -> Result<T, JsValue> {
    SESSIONS
        .with(|s| f(&mut s.borrow_mut()))
        .map_err(|e| error_json(&e))
}

/// Opens a dataset and keeps it under a handle (`close` frees it); `samples(h)` says what it holds.
#[wasm_bindgen]
pub fn open(names: js_sys::Array, sizes: &[f64], read: Function) -> Result<u32, JsValue> {
    open_members(Members::new(names, sizes, read)?.fileset())
}
/// `open` with the SHA-1 of each member when the page took it while copying the file (empty text: unknown): the header of the mzML uses it instead of
/// reading the file again. Only the `thermo` module has it (the other readers take their SHA-1 themselves).
#[cfg(feature = "thermo")]
#[wasm_bindgen]
pub fn open_with_sha1(
    names: js_sys::Array,
    sizes: &[f64],
    read: Function,
    sha1: js_sys::Array,
) -> Result<u32, JsValue> {
    let mut fs = Members::new(names, sizes, read)?.fileset();
    for (x, v) in fs.membri.iter_mut().zip(sha1.iter()) {
        x.sha1 = v.as_string().filter(|t| !t.is_empty());
    }
    open_members(fs)
}
fn open_members(fs: FileSet) -> Result<u32, JsValue> {
    // as it is, without the SRM traces of `mzlab_lettori::apri`: the engine reads the module through `ExternalReader` and decides itself what it does with them
    let l = mzlab_lettori::apri_nudo(fs, &Limiti::sito()).map_err(|e| error_json(&e))?;
    with_sessions(|s| Ok(s.insert(l)))
}
#[wasm_bindgen]
pub fn samples(h: u32) -> Result<String, JsValue> {
    with_sessions(|s| s.samples(h))
}
/// The version of the binary transport of the index and the scans (`mzlab_core::trasporto::PROTOCOLLO`): the engine refuses a module with another.
#[wasm_bindgen]
pub fn protocol() -> u32 {
    mzlab_core::trasporto::PROTOCOLLO
}
/// The index of the sample as one `Uint8Array` (`mzlab_core::trasporto::encode_indice`).
#[wasm_bindgen]
pub fn index(h: u32, c: u32) -> Result<Vec<u8>, JsValue> {
    with_sessions(|s| s.index(h, c as usize))
}
/// A scan as one `Uint8Array` (`mzlab_core::trasporto::encode_scansione`).
#[wasm_bindgen]
pub fn scan(h: u32, c: u32, k: u32) -> Result<Vec<u8>, JsValue> {
    with_sessions(|s| s.scan(h, c as usize, k as usize))
}
/// A scan as `[head, m/z, intensities]`: the head of `trasporto::testa_scansione`, then views of this module's memory (`Float64Array`, and `Float32Array`
/// or, for 64-bit intensities, `Float64Array`), valid until its next call: the engine copies each in one piece instead of a block copied twice.
#[wasm_bindgen]
pub fn scan_parts(h: u32, c: u32, k: u32) -> Result<js_sys::Array, JsValue> {
    with_sessions(|s| {
        let sp = s.scan_parts(h, c as usize, k as usize)?;
        let n = sp.mz.len();
        let a = js_sys::Array::new();
        a.push(&js_sys::Uint8Array::from(
            &mzlab_core::trasporto::testa_scansione(sp)[..],
        ));
        // SAFETY: nothing allocates in this module between here and the engine's copy (the views are read before any other call)
        unsafe {
            a.push(&js_sys::Float64Array::view(&sp.mz));
            match &sp.wide {
                Some(w) => a.push(&js_sys::Float64Array::view(&w[..n.min(w.len())])),
                None => a.push(&js_sys::Float32Array::view(
                    &sp.intensity[..n.min(sp.intensity.len())],
                )),
            };
        }
        Ok(a)
    })
}
#[wasm_bindgen]
pub fn scan_header(h: u32, c: u32, k: u32) -> Result<String, JsValue> {
    with_sessions(|s| s.scan_header(h, c as usize, k as usize))
}
#[wasm_bindgen]
pub fn chromatograms(h: u32, c: u32) -> Result<String, JsValue> {
    with_sessions(|s| s.chromatograms(h, c as usize))
}
#[wasm_bindgen]
pub fn chromatogram(h: u32, c: u32, id: &str) -> Result<String, JsValue> {
    with_sessions(|s| s.chromatogram(h, c as usize, id))
}
#[wasm_bindgen]
pub fn method(h: u32, c: u32) -> Result<String, JsValue> {
    with_sessions(|s| s.method(h, c as usize))
}
/// The spectra of the sample that a truncated file declares but does not hold.
#[wasm_bindgen]
pub fn missing(h: u32, c: u32) -> Result<u32, JsValue> {
    with_sessions(|s| s.missing(h, c as usize)).map(|n| n.min(u32::MAX as usize) as u32)
}
/// The header of the mzML of the sample as JSON (`null` when the reader has none).
#[wasm_bindgen]
pub fn file_header(h: u32, c: u32) -> Result<String, JsValue> {
    with_sessions(|s| s.file_header(h, c as usize))
}
#[wasm_bindgen]
pub fn close(h: u32) {
    SESSIONS.with(|s| s.borrow_mut().close(h));
}
