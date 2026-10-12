//! wasm-bindgen bridge of `mzlab-lib`. It runs only inside the Web Worker of the libraries (`libreria-worker.js`), as a `.wasm` of its own that
//! is fetched only when the libraries are opened or searched. Files are never held whole: the worker gives callbacks for the bytes it has
//! (`read(offset, length) -> Uint8Array` on a Blob read synchronously, or on an OPFS sync handle) and for the ones the builder writes
//! (`write(offset, Uint8Array)`). Errors are JSON `{"error_key": ..., "params": {...}}` strings (the page translates the key), never a text.
#![forbid(unsafe_code)]
use js_sys::{Function, Uint8Array};
use mzlab_lib::build::{BuildOpts, Builder};
use mzlab_lib::error::{Error, Result};
use mzlab_lib::io::{Sink, Source};
use mzlab_lib::parse::{detect, Format, Parser};
use mzlab_lib::read::{cache_budget, Library};
use mzlab_lib::search::{analogs, identity, AnalogHit, AnalogOpts, Hit, IdentityOpts, Query, Tol};
use mzlab_lib::spec::Spectrum;
use serde_json::{json, Value};
use std::cell::RefCell;
use std::rc::Rc;
use wasm_bindgen::prelude::*;

mod fdr;

fn js_err(e: Error) -> JsValue {
    JsValue::from_str(&e.to_json())
}

fn io_err() -> Error {
    Error::new("lib.err.io")
}

/// Version of the library engine, for the page to check the loaded `.wasm`.
#[wasm_bindgen]
pub fn version() -> String {
    "mzlib/1".to_string()
}

/// Total bytes of block cache for all the open libraries: 64 MB, 32 MB on iPad and phones (the one place is `mzlab_lib::read`).
#[wasm_bindgen]
pub fn total_cache_budget(small_device: bool) -> f64 {
    cache_budget(small_device) as f64
}

/// Format of a library file from its name and first bytes: "msp", "mgf", "massbank" or "json"; the error is `lib.err.unsupported` or `lib.err.unknown`.
#[wasm_bindgen]
pub fn detect_format(name: &str, head: &[u8]) -> std::result::Result<String, JsValue> {
    let f = detect(name, head).map_err(js_err)?;
    Ok(match f {
        Format::Msp => "msp",
        Format::Mgf => "mgf",
        Format::MassBank => "massbank",
        Format::Json => "json",
    }
    .to_string())
}

/// Bytes held by JavaScript (an OPFS handle, a memory buffer, a Blob): `read(offset, length) -> Uint8Array`, `write(offset, Uint8Array)`.
struct JsIo {
    read: Option<Function>,
    write: Option<Function>,
    len: u64,
}

impl Source for JsIo {
    fn len(&self) -> u64 {
        self.len
    }
    fn read_at(&self, off: u64, buf: &mut [u8]) -> Result<()> {
        let end = off
            .checked_add(buf.len() as u64)
            .ok_or_else(|| Error::corrupt("truncated"))?;
        if end > self.len {
            return Err(Error::corrupt("truncated"));
        }
        let f = self.read.as_ref().ok_or_else(io_err)?;
        let v = f
            .call2(
                &JsValue::NULL,
                &(off as f64).into(),
                &(buf.len() as f64).into(),
            )
            .map_err(|_| io_err())?;
        let a: Uint8Array = v.dyn_into().map_err(|_| io_err())?;
        if a.length() as usize != buf.len() {
            return Err(Error::corrupt("truncated"));
        }
        a.copy_to(buf);
        Ok(())
    }
}

impl Sink for JsIo {
    fn write_at(&mut self, off: u64, data: &[u8]) -> Result<()> {
        let f = self.write.as_ref().ok_or_else(io_err)?;
        let a = Uint8Array::from(data);
        f.call2(&JsValue::NULL, &(off as f64).into(), &a)
            .map_err(|_| io_err())?;
        self.len = self.len.max(off + data.len() as u64);
        Ok(())
    }
}

/// The scratch file of the builder must report its length at the start (0: it is empty).
fn scratch(read: Function, write: Function) -> JsIo {
    JsIo {
        read: Some(read),
        write: Some(write),
        len: u64::MAX / 4,
    }
}

#[wasm_bindgen]
pub struct Importer {
    parser: Option<Parser<Box<dyn FnMut(Spectrum)>>>,
    builder: Rc<RefCell<Option<Builder<JsIo>>>>,
    queue: Rc<RefCell<Vec<Spectrum>>>,
    failed: Option<Error>,
}

#[wasm_bindgen]
impl Importer {
    /// `scratch_read(offset, length)` / `scratch_write(offset, bytes)`: a temporary file or memory kept by the worker.
    #[wasm_bindgen(constructor)]
    pub fn new(
        name: &str,
        head: &[u8],
        scratch_read: Function,
        scratch_write: Function,
    ) -> std::result::Result<Importer, JsValue> {
        let fmt = detect(name, head).map_err(js_err)?;
        let queue: Rc<RefCell<Vec<Spectrum>>> = Rc::new(RefCell::new(Vec::new()));
        let q2 = queue.clone();
        let parser: Parser<Box<dyn FnMut(Spectrum)>> =
            Parser::new(fmt, Box::new(move |s| q2.borrow_mut().push(s)));
        let mut sc = scratch(scratch_read, scratch_write);
        sc.len = 0;
        let builder = Builder::new(sc, BuildOpts::default());
        Ok(Importer {
            parser: Some(parser),
            builder: Rc::new(RefCell::new(Some(builder))),
            queue,
            failed: None,
        })
    }

    /// An importer with no file to parse: spectra are given one by one with `add_spectrum` (the conversion of the old `MZL1` archives).
    #[wasm_bindgen(js_name = forSpectra)]
    pub fn for_spectra(scratch_read: Function, scratch_write: Function) -> Importer {
        let mut sc = scratch(scratch_read, scratch_write);
        sc.len = 0;
        Importer {
            parser: None,
            builder: Rc::new(RefCell::new(Some(Builder::new(sc, BuildOpts::default())))),
            queue: Rc::new(RefCell::new(Vec::new())),
            failed: None,
        }
    }

    /// One spectrum; `fields` is a JSON array of up to ten texts (name, adduct, collision energy, instrument, formula, InChIKey, SMILES, accession, authors, licence).
    #[wasm_bindgen(js_name = addSpectrum)]
    pub fn add_spectrum(
        &mut self,
        prec: f64,
        pol: i32,
        mz: &[f64],
        it: &[f64],
        fields: &str,
    ) -> std::result::Result<(), JsValue> {
        let f: Vec<String> = serde_json::from_str(fields).unwrap_or_default();
        let g = |i: usize| f.get(i).cloned().unwrap_or_default();
        let meta = mzlab_lib::spec::Meta::from_fields([
            g(0),
            g(1),
            g(2),
            g(3),
            g(4),
            g(5),
            g(6),
            g(7),
            g(8),
            g(9),
        ]);
        self.queue.borrow_mut().push(Spectrum {
            prec,
            pol: pol as i8,
            mz: mz.to_vec(),
            it: it.to_vec(),
            meta,
        });
        self.drain().map_err(js_err)
    }

    /// Feeds the next bytes of the file.
    pub fn push(&mut self, chunk: &[u8]) -> std::result::Result<(), JsValue> {
        if let Some(p) = self.parser.as_mut() {
            p.push(chunk);
        }
        self.drain().map_err(js_err)
    }

    fn drain(&mut self) -> Result<()> {
        if let Some(e) = &self.failed {
            return Err(e.clone());
        }
        let items: Vec<Spectrum> = std::mem::take(&mut *self.queue.borrow_mut());
        let mut b = self.builder.borrow_mut();
        let b = b.as_mut().ok_or_else(io_err)?;
        for s in items {
            if let Err(e) = b.add(&s) {
                self.failed = Some(e.clone());
                return Err(e);
            }
        }
        Ok(())
    }

    /// Spectra kept so far.
    pub fn count(&self) -> u32 {
        self.builder.borrow().as_ref().map_or(0, |b| b.len() as u32)
    }

    /// Ends the file and writes the library through `out_write(offset, bytes)`; the answer is a JSON summary.
    pub fn finish(&mut self, out_write: Function) -> std::result::Result<String, JsValue> {
        if let Some(p) = self.parser.as_mut() {
            p.end();
        }
        self.drain().map_err(js_err)?;
        let stats = self.parser.as_ref().map(|p| p.stats).unwrap_or_default();
        let b = self
            .builder
            .borrow_mut()
            .take()
            .ok_or_else(|| js_err(io_err()))?;
        let mut out = JsIo {
            read: None,
            write: Some(out_write),
            len: 0,
        };
        let info = b.finish(&mut out).map_err(|e| {
            if e.key == "lib.err.noSpectrum" {
                Error::new("lib.err.noSpectrum").with("dropped", stats.dropped + stats.read)
            } else {
                e
            }
        });
        let info = info.map_err(js_err)?;
        Ok(json!({
            "n": info.n_spectra, "peaks": info.n_peaks, "bytes": info.bytes, "pos": info.pos, "neg": info.neg,
            "read": stats.read, "dropped": stats.dropped + info.emptied, "noPrec": stats.no_prec, "noPeaks": stats.no_peaks + info.emptied, "broken": stats.broken,
        })
        .to_string())
    }
}

#[wasm_bindgen]
pub struct Lib {
    pub(crate) inner: Library<JsIo>,
}

fn tol_of(v: f64, ppm: bool) -> Tol {
    if ppm {
        Tol::Ppm(v)
    } else {
        Tol::Da(v)
    }
}

fn meta_json(lib: &Library<JsIo>, i: u32) -> Value {
    let m = lib.meta(i).unwrap_or_default();
    json!({"name": m.name, "type": m.adduct, "ce": m.ce, "inst": m.instrument, "formula": m.formula, "ik": m.inchikey,
           "smiles": m.smiles, "accession": m.accession, "authors": m.authors, "license": m.license})
}

fn merge(mut a: Value, b: Value) -> Value {
    if let (Some(x), Some(y)) = (a.as_object_mut(), b.as_object()) {
        for (k, v) in y {
            x.insert(k.clone(), v.clone());
        }
    }
    a
}

fn hit_json(lib: &Library<JsIo>, h: &Hit) -> Value {
    merge(
        meta_json(lib, h.index),
        json!({
            "pos": h.index, "prec": h.prec, "polarity": h.pol, "lic": h.licence,
            "ent": h.entropy, "cos": h.cosine, "forward": h.forward, "reverse": h.reverse, "shared": h.shared, "explained": h.explained, "nlShared": h.nl_shared,
            "dprec": h.dprec, "pairs": h.pairs.iter().map(|p| json!([p.0, p.1])).collect::<Vec<_>>(), "mz": h.spec.mz, "it": h.spec.it,
        }),
    )
}

fn analog_json(lib: &Library<JsIo>, h: &AnalogHit) -> Value {
    merge(
        meta_json(lib, h.index),
        json!({
            "pos": h.index, "prec": h.prec, "polarity": h.pol, "lic": h.licence,
            "mcos": h.mc.cosine, "matched": h.mc.matched, "shifted": h.mc.shifted, "delta": h.mc.delta, "coinc": h.coinc, "hybrid": h.hybrid,
            "mz": h.spec.mz, "it": h.spec.it,
            "pairs": h.mc.pairs.iter().map(|p| json!([p.0, p.1, p.2])).collect::<Vec<_>>(),
            "qa": {"mz": h.mc.a_top.mz, "it": h.mc.a_top.it}, "la": {"mz": h.mc.b_top.mz, "it": h.mc.b_top.it},
        }),
    )
}

#[wasm_bindgen]
impl Lib {
    /// Opens a `.mzlib` of `size` bytes read through `read(offset, length)`.
    pub fn open(read: Function, size: f64, cache_bytes: f64) -> std::result::Result<Lib, JsValue> {
        let src = JsIo {
            read: Some(read),
            write: None,
            len: size as u64,
        };
        Library::open(src, cache_bytes as usize)
            .map(|inner| Lib { inner })
            .map_err(js_err)
    }

    pub fn set_cache_budget(&self, bytes: f64) {
        self.inner.set_cache_budget(bytes as usize);
    }

    pub fn n_spectra(&self) -> u32 {
        self.inner.n_spectra()
    }

    /// Checks every block and page against its CRC (a sound file or `lib.err.corrupt`).
    pub fn verify(&self) -> std::result::Result<(), JsValue> {
        self.inner.verify().map_err(js_err)
    }

    /// Identity: the best `top` spectra of the polarity (`pol`: 1, -1, 0 = any) within the precursor tolerance (ppm or Da).
    #[allow(clippy::too_many_arguments)]
    pub fn identity(
        &self,
        prec: f64,
        pol: i32,
        mz: &[f64],
        it: &[f64],
        tol: f64,
        ppm: bool,
        frag: f64,
        top: u32,
    ) -> std::result::Result<String, JsValue> {
        let Some(q) = Query::new(prec, pol as i8, mz, it) else {
            return Ok(json!({"results": [], "query": {"mz": [], "it": []}}).to_string());
        };
        let hits = identity(
            &self.inner,
            &q,
            &IdentityOpts {
                prec_tol: tol_of(tol, ppm),
                frag_da: frag,
                pol: pol as i8,
                top: top as usize,
            },
        )
        .map_err(js_err)?;
        Ok(json!({"results": hits.iter().map(|h| hit_json(&self.inner, h)).collect::<Vec<_>>(), "query": {"mz": q.spec.mz, "it": q.spec.it}}).to_string())
    }

    /// Best identity only, without peaks (for «Identify all MS2»): JSON object or `null`. `alt` is the best hit of another structure (another first
    /// InChIKey block, or another name when there is no key): the lead of the best over it is the «distinct» gate of the confidence levels.
    pub fn best(
        &self,
        prec: f64,
        pol: i32,
        mz: &[f64],
        it: &[f64],
        tol: f64,
        ppm: bool,
        frag: f64,
    ) -> std::result::Result<String, JsValue> {
        let Some(q) = Query::new(prec, pol as i8, mz, it) else {
            return Ok("null".into());
        };
        let hits = identity(
            &self.inner,
            &q,
            &IdentityOpts {
                prec_tol: tol_of(tol, ppm),
                frag_da: frag,
                pol: pol as i8,
                top: 8,
            },
        )
        .map_err(js_err)?;
        let light = |h: &Hit| merge(meta_json(&self.inner, h.index), json!({"ent": h.entropy, "cos": h.cosine, "shared": h.shared, "dprec": h.dprec, "prec": h.prec, "nlib": h.spec.mz.len()}));
        let key = |h: &Hit| {
            let m = self.inner.meta(h.index).unwrap_or_default();
            let ik: String = m.inchikey.chars().take(14).collect::<String>().to_uppercase();
            if ik.is_empty() {
                format!("n:{}", m.name.to_lowercase().chars().filter(|c| c.is_ascii_alphanumeric()).collect::<String>())
            } else {
                ik
            }
        };
        Ok(match hits.first() {
            None => "null".into(),
            Some(h) => {
                let k = key(h);
                let alt = hits.iter().skip(1).find(|x| key(x) != k).map(&light).unwrap_or(Value::Null);
                merge(light(h), json!({"alt": alt})).to_string()
            }
        })
    }

    /// Analogues (modified cosine): the best `top`.
    pub fn analogs(
        &self,
        prec: f64,
        pol: i32,
        mz: &[f64],
        it: &[f64],
        frag: f64,
        max_delta: f64,
        top: u32,
    ) -> std::result::Result<String, JsValue> {
        let Some(q) = Query::new(prec, pol as i8, mz, it) else {
            return Ok(json!({"results": [], "query": {"mz": [], "it": []}}).to_string());
        };
        let o = AnalogOpts {
            frag_da: frag,
            pol: pol as i8,
            max_delta,
            top: top as usize,
            ..AnalogOpts::default()
        };
        let hits = analogs(&self.inner, &q, &o).map_err(js_err)?;
        Ok(json!({"results": hits.iter().map(|h| analog_json(&self.inner, h)).collect::<Vec<_>>(), "query": {"mz": q.spec.mz, "it": q.spec.it}}).to_string())
    }
}

/// Test hook: the supervisor of the page must recover from a dead instance (`panic = "abort"` turns this into a trap).
#[wasm_bindgen]
pub fn crash() {
    panic!("crash hook");
}
