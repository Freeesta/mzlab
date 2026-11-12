//! A reader that lives in a readers module (`wasm-lettori/<family>/`, docs/agenti/lettori-modulo.md) and not in the engine: `ExternalReader` is a `Lettore` over the synchronous
//! functions of the session interface of the module, which the page loads in the same worker (`rust-nativo.js`). The conversion of the answers (JSON, typed arrays) into the types of
//! the contract is in `Replies` and does not touch JS, so it is tried natively with a fake `Module`; `JsModule` is the one that calls the functions. The index and the scans cross as
//! one byte array each (`mzlab_core::transport`), the rest as JSON.
use std::cell::RefCell;
use std::collections::{HashMap, HashSet};

use js_sys::{Array, Float32Array, Float64Array, Function, Reflect, Uint8Array};
use mzlab_core::error::{Error, Result};
use mzlab_core::json::{self, Value};
use mzlab_core::mzml::{ChromKind, Chromatogram, Spectrum};
use mzlab_core::mzmlwrite::Header;
use mzlab_core::readers::{
    cause_key, ChromatogramEntry, Field, Format, Losses, Reader, Sample, ScanEntry, EVENTS_REFUSED,
    SPECTRUM_DAMAGED, TRUNCATED,
};
use mzlab_core::transport::{
    decode_index, decode_scansione, scan_from_parts, Intensity, PROTOCOLLO,
};
use wasm_bindgen::prelude::*;

/// The session interface of a module, as the engine uses it: JSON texts, and the index and the scans as bytes. An error is the `Error` of the module (same key).
pub trait Module {
    /// The version of the transport the module speaks (`transport::PROTOCOLLO`).
    fn protocol(&mut self) -> Result<u32>;
    fn samples(&mut self) -> Result<String>;
    /// The index of sample `c` (`transport::encode_index`).
    fn index(&mut self, c: usize) -> Result<Vec<u8>>;
    /// Scan `k` of sample `c` (`transport::encode_scansione`).
    fn scan(&mut self, c: usize, k: usize) -> Result<Vec<u8>>;
    /// Scan `k` of sample `c` decoded: from the block of `scan`, unless the module hands its columns over apart (`JsModule`).
    fn scan_spectrum(&mut self, c: usize, k: usize) -> Result<Spectrum> {
        decode_scansione(k, &self.scan(c, k)?)
    }
    fn scan_header(&mut self, c: usize, k: usize) -> Result<String>;
    fn chromatograms(&mut self, c: usize) -> Result<String>;
    fn chromatogram(&mut self, c: usize, id: &str) -> Result<String>;
    fn method(&mut self, c: usize) -> Result<String>;
    fn file_header(&mut self, c: usize) -> Result<String>;
    /// The spectra a truncated file declares but does not hold (a module of an older build knows none: 0).
    fn missing(&mut self, _c: usize) -> Result<u32> {
        Ok(0)
    }
    /// The key of why scans are `missing` (`Losses::cause`; a module of an older build says none: the file is cut).
    fn cause(&mut self, _c: usize) -> Result<String> {
        Ok(String::new())
    }
}

/// An answer of the module the engine cannot decode (`err.module.reply`, as `transport` says it).
fn broken(what: &str) -> Error {
    Error::new("err.module.reply").with("detail", what)
}

thread_local! { static NAMES: RefCell<HashSet<&'static str>> = RefCell::new(HashSet::new()); }

/// The text as a `&'static str` (the keys and the names of the parameters of `Error` are static): each distinct text is kept once.
fn interned(s: &str) -> &'static str {
    NAMES.with(|n| {
        let mut n = n.borrow_mut();
        if let Some(&k) = n.get(s) {
            return k;
        }
        let k: &'static str = Box::leak(s.to_string().into_boxed_str());
        n.insert(k);
        k
    })
}

/// The `Error` a module threw: `{"error_key": ..., "params": {...}}`; anything else is an unreadable answer.
pub(crate) fn module_error(e: &str) -> Error {
    let Ok(v) = json::parse(e.as_bytes()) else {
        return broken("module");
    };
    let Some(key) = v.get("error_key").and_then(Value::str) else {
        return broken("module");
    };
    let mut err = Error::new(interned(key));
    if let Some(Value::Obj(p)) = v.get("params") {
        for (k, x) in p {
            err = err.with(interned(k), x.str().unwrap_or_default());
        }
    }
    err
}

fn doc(t: &str) -> Result<Value> {
    json::parse(t.as_bytes())
}

fn text(v: Option<&Value>) -> String {
    v.and_then(Value::str).unwrap_or_default().to_string()
}

fn pairs(v: Option<&Value>) -> Vec<(String, String)> {
    v.map(Value::arr)
        .unwrap_or_default()
        .iter()
        .map(|p| (text(p.at(0)), text(p.at(1))))
        .collect()
}

fn format(id: &str) -> Format {
    match id {
        "thermo-raw" => Format::ThermoRaw,
        "sciex-wiff" => Format::SciexWiff,
        "sciex-dam" => Format::SciexDam,
        "agilent-d" => Format::AgilentD,
        "mzml" => Format::Mzml,
        _ => Format::Altro(interned(id)),
    }
}

fn kind_of(name: &str) -> ChromKind {
    match name {
        "Tic" => ChromKind::Tic,
        "Bpc" => ChromKind::Bpc,
        "Pda" => ChromKind::Pda,
        "Pressure" => ChromKind::Pressure,
        "Srm" => ChromKind::Srm,
        "Sic" => ChromKind::Sic,
        "Analog" => ChromKind::Analog,
        _ => ChromKind::Other,
    }
}

fn number(v: Option<&Value>) -> f64 {
    v.and_then(Value::f64).unwrap_or(f64::NAN)
}

fn optional(v: Option<&Value>) -> Option<f64> {
    v.and_then(Value::f64)
}

fn numbers(v: Option<&Value>) -> Vec<f64> {
    v.map(Value::arr)
        .unwrap_or_default()
        .iter()
        .map(|x| x.f64().unwrap_or(f64::NAN))
        .collect()
}

/// The conversions of the answers of a module into the types of the contract.
pub(crate) struct Replies;

impl Replies {
    /// `{formato, campioni:[{id,name,scans}]}`.
    pub(crate) fn samples(t: &str) -> Result<(Format, Vec<Sample>)> {
        let v = doc(t)?;
        let c = v
            .get("campioni")
            .map(Value::arr)
            .unwrap_or_default()
            .iter()
            .map(|c| Sample {
                id: number(c.get("id")) as u32,
                nome: text(c.get("name")),
                scansioni: number(c.get("scans")) as usize,
                size: c.get("size").and_then(Value::f64).map(|d| d as u64),
            })
            .collect::<Vec<_>>();
        if c.is_empty() {
            return Err(broken("samples"));
        }
        Ok((format(&text(v.get("formato"))), c))
    }

    /// The index as the module wrote it (`transport::decode_index`).
    pub(crate) fn index(b: &[u8]) -> Result<Vec<ScanEntry>> {
        decode_index(b)
    }
}

/// The answers about the scans and their headers.
pub struct ScanReplies;

impl ScanReplies {
    /// The scan as the module sends it, with the id and the filter of its row of the index.
    pub(crate) fn scan(mut s: Spectrum, row: Option<&ScanEntry>) -> Spectrum {
        if let Some(r) = row {
            s.id = r.id.clone();
            s.filter = r.filtro.clone();
        }
        s
    }

    /// `[{name, value, unit, cv}]`.
    pub(crate) fn scan_header(t: &str) -> Result<Vec<Field>> {
        let opt = |v: Option<&Value>| v.and_then(Value::str).map(str::to_string);
        Ok(doc(t)?
            .arr()
            .iter()
            .map(|f| Field {
                nome: text(f.get("name")),
                valore: text(f.get("value")),
                unita: opt(f.get("unit")),
                cv: opt(f.get("cv")),
            })
            .collect())
    }

    /// `[{id, kind, unit}]` (`kind` is the name of the `ChromKind`).
    pub(crate) fn chromatograms(t: &str) -> Result<Vec<ChromatogramEntry>> {
        Ok(doc(t)?
            .arr()
            .iter()
            .map(|x| ChromatogramEntry {
                id: text(x.get("id")),
                tipo: kind_of(&text(x.get("kind"))),
                unita: text(x.get("unit")),
            })
            .collect())
    }
}

/// The answers about the chromatograms and the header of the mzML.
pub struct TraceReplies;

impl TraceReplies {
    /// `{id, kind, q1, q3, unit, time, intensity}`.
    pub(crate) fn chromatogram(t: &str) -> Result<Chromatogram> {
        let x = doc(t)?;
        Ok(Chromatogram {
            index: 0,
            id: text(x.get("id")),
            kind: kind_of(&text(x.get("kind"))),
            q1: optional(x.get("q1")),
            q3: optional(x.get("q3")),
            unit: text(x.get("unit")),
            time: numbers(x.get("time")),
            intensity: numbers(x.get("intensity")),
        })
    }

    /// The header of the mzML (`null`: the reader has none); the fields are those of `Sessions::file_header` of the module.
    pub(crate) fn file_header(t: &str) -> Result<Option<Header>> {
        let v = doc(t)?;
        if v.is_null() {
            return Ok(None);
        }
        let pair = |k: &str| {
            v.get(k)
                .filter(|p| !p.is_null())
                .map(|p| (text(p.at(0)), text(p.at(1))))
        };
        let strs = |k: &str| {
            v.get(k)
                .map(Value::arr)
                .unwrap_or_default()
                .iter()
                .map(|s| s.str().unwrap_or_default().to_string())
                .collect::<Vec<_>>()
        };
        let opt = |k: &str| v.get(k).and_then(Value::str).map(str::to_string);
        Ok(Some(Header {
            run: text(v.get("run")),
            sources: strs("sources"),
            instrument: pair("instrument"),
            components: v
                .get("components")
                .map(Value::arr)
                .unwrap_or_default()
                .iter()
                .map(|c| (text(c.at(0)), text(c.at(1)), text(c.at(2))))
                .collect(),
            software: pair("software").unwrap_or_default(),
            acquired: opt("acquired"),
            serial: opt("serial"),
            source_sha1: strs("source_sha1"),
            source_cv: pairs(v.get("source_cv")),
            sample: pair("sample"),
            run_params: pairs(v.get("run_params")),
            content: pairs(v.get("content")),
            extra_software: v
                .get("extra_software")
                .map(Value::arr)
                .unwrap_or_default()
                .iter()
                .map(|c| (text(c.at(0)), text(c.at(1)), text(c.at(2))))
                .collect(),
            centroided: v.get("centroided").and_then(Value::bool).unwrap_or(false),
            ..Header::default()
        }))
    }
}

/// A `Lettore` over a `Module`.
pub struct ExternalReader<M: Module> {
    m: M,
    format: Format,
    samples: Vec<Sample>,
    indexes: HashMap<usize, Vec<ScanEntry>>,
}

impl<M: Module> ExternalReader<M> {
    pub fn new(mut m: M) -> Result<Self> {
        // a module cached from another version of the site speaks another transport: the page asks to load it again
        if m.protocol()? != PROTOCOLLO {
            return Err(Error::new("err.file.wiffLoad")
                .with("family", "thermo")
                .with("detail", "protocol"));
        }
        let (format, samples) = Replies::samples(&m.samples()?)?;
        Ok(ExternalReader {
            m,
            format,
            samples,
            indexes: HashMap::new(),
        })
    }
}

impl<M: Module> Reader for ExternalReader<M> {
    fn format(&self) -> Format {
        self.format
    }
    fn samples(&self) -> &[Sample] {
        &self.samples
    }
    fn index(&mut self, c: usize) -> Result<&[ScanEntry]> {
        if !self.indexes.contains_key(&c) {
            let r = Replies::index(&self.m.index(c)?)?;
            self.indexes.insert(c, r);
        }
        Ok(self.indexes.get(&c).map_or(&[][..], Vec::as_slice))
    }
    fn scan(&mut self, c: usize, k: usize) -> Result<Spectrum> {
        let s = self.m.scan_spectrum(c, k)?;
        self.index(c)?;
        Ok(ScanReplies::scan(
            s,
            self.indexes.get(&c).and_then(|v| v.get(k)),
        ))
    }
    fn scan_header(&mut self, c: usize, k: usize) -> Result<Vec<Field>> {
        ScanReplies::scan_header(&self.m.scan_header(c, k)?)
    }
    fn chromatograms(&mut self, c: usize) -> Result<Vec<ChromatogramEntry>> {
        ScanReplies::chromatograms(&self.m.chromatograms(c)?)
    }
    fn chromatogram(&mut self, c: usize, id: &str) -> Result<Chromatogram> {
        TraceReplies::chromatogram(&self.m.chromatogram(c, id)?)
    }
    fn method(&mut self, c: usize) -> Result<String> {
        self.m.method(c)
    }
    fn missing(&mut self, c: usize) -> usize {
        self.m.missing(c).unwrap_or(0) as usize
    }
    /// As the module's reader: scans it lists without their event or with data that do not decode (rows of its index, the latter with no points), else scans out of the index with the cause key the module gives (a cut file by default).
    fn losses(&mut self, c: usize) -> Result<Losses> {
        let lacking = self.missing(c);
        let n = self.index(c)?.len();
        Ok(match self.m.cause(c)?.as_str() {
            EVENTS_REFUSED => Losses::of(n, n.saturating_sub(lacking), EVENTS_REFUSED),
            SPECTRUM_DAMAGED => Losses::of(n, n.saturating_sub(lacking), SPECTRUM_DAMAGED),
            k => Losses::of(n + lacking, n, cause_key(k).unwrap_or(TRUNCATED)),
        })
    }
    fn mzml_header(&mut self, c: usize) -> Result<Option<Header>> {
        TraceReplies::file_header(&self.m.file_header(c)?)
    }
}

/// The functions of a session of the module, as an object of JS functions `{samples, index, scan, scan_header, chromatograms, chromatogram, method, file_header, missing}`
/// that already hold the handle of the session: `samples()`, `index(c)`, `scan(c, k)`, ...
pub(crate) struct JsModule {
    session: JsValue,
    // `scan_parts` of a module that hands a scan over as its head and two views of its columns (an older build has only `scan`)
    parts: Option<Function>,
}

impl JsModule {
    pub(crate) fn new(session: JsValue) -> Self {
        let parts = Reflect::get(&session, &JsValue::from_str("scan_parts"))
            .ok()
            .and_then(|f| f.dyn_into().ok());
        JsModule { session, parts }
    }

    fn call(&self, name: &str, args: &[JsValue]) -> Result<JsValue> {
        let f: Function = Reflect::get(&self.session, &JsValue::from_str(name))
            .ok()
            .and_then(|f| f.dyn_into().ok())
            .ok_or_else(|| broken(name))?;
        let a: Array = args.iter().collect();
        f.apply(&self.session, &a).map_err(|e| from_module(name, e))
    }

    /// A `Uint8Array` answer, copied once into the memory of the engine.
    fn byte(&self, name: &str, args: &[JsValue]) -> Result<Vec<u8>> {
        let a: Uint8Array = self
            .call(name, args)?
            .dyn_into()
            .map_err(|_| broken(name))?;
        Ok(a.to_vec())
    }

    fn text(&self, name: &str, args: &[JsValue]) -> Result<String> {
        self.call(name, args)?
            .as_string()
            .ok_or_else(|| broken(name))
    }
}

impl Drop for JsModule {
    /// Frees the session of the module.
    fn drop(&mut self) {
        drop(self.call("close", &[]));
    }
}

/// The error a module threw: its JSON, or the key of a broken module.
fn from_module(name: &str, e: JsValue) -> Error {
    match e.as_string() {
        Some(t) => module_error(&t),
        None => broken(name),
    }
}

/// The scan from `scan_parts`: the columns are views of the module's memory, valid until its next call, so each is copied here at once, in one piece.
fn from_parts(k: usize, r: JsValue) -> Result<Spectrum> {
    let a: Array = r.dyn_into().map_err(|_| broken("scan"))?;
    let head: Uint8Array = a.get(0).dyn_into().map_err(|_| broken("scan"))?;
    let mz: Float64Array = a.get(1).dyn_into().map_err(|_| broken("scan"))?;
    let mz = mz.to_vec();
    let y = a.get(2);
    let y = match y.dyn_ref::<Float64Array>() {
        Some(w) => Intensity::Larghe(w.to_vec()),
        None => Intensity::Strette(
            y.dyn_into::<Float32Array>()
                .map_err(|_| broken("scan"))?
                .to_vec(),
        ),
    };
    scan_from_parts(k, &head.to_vec(), mz, y)
}

impl Module for JsModule {
    fn protocol(&mut self) -> Result<u32> {
        self.call("protocol", &[])?
            .as_f64()
            .map(|v| v as u32)
            .ok_or_else(|| broken("protocol"))
    }
    fn samples(&mut self) -> Result<String> {
        self.text("samples", &[])
    }
    fn index(&mut self, c: usize) -> Result<Vec<u8>> {
        self.byte("index", &[(c as f64).into()])
    }
    fn scan(&mut self, c: usize, k: usize) -> Result<Vec<u8>> {
        self.byte("scan", &[(c as f64).into(), (k as f64).into()])
    }
    fn scan_spectrum(&mut self, c: usize, k: usize) -> Result<Spectrum> {
        let Some(f) = &self.parts else {
            return decode_scansione(k, &self.scan(c, k)?);
        };
        let r = f
            .call2(&self.session, &(c as f64).into(), &(k as f64).into())
            .map_err(|e| from_module("scan", e))?;
        from_parts(k, r)
    }
    fn scan_header(&mut self, c: usize, k: usize) -> Result<String> {
        self.text("scan_header", &[(c as f64).into(), (k as f64).into()])
    }
    fn chromatograms(&mut self, c: usize) -> Result<String> {
        self.text("chromatograms", &[(c as f64).into()])
    }
    fn chromatogram(&mut self, c: usize, id: &str) -> Result<String> {
        self.text("chromatogram", &[(c as f64).into(), id.into()])
    }
    fn method(&mut self, c: usize) -> Result<String> {
        self.text("method", &[(c as f64).into()])
    }
    fn file_header(&mut self, c: usize) -> Result<String> {
        self.text("file_header", &[(c as f64).into()])
    }
    fn missing(&mut self, c: usize) -> Result<u32> {
        // a session of an older module has no such function
        Ok(self
            .call("missing", &[(c as f64).into()])
            .ok()
            .and_then(|v| v.as_f64())
            .map_or(0, |v| v as u32))
    }
    fn cause(&mut self, c: usize) -> Result<String> {
        // a session of an older module has no such function
        Ok(self
            .call("cause", &[(c as f64).into()])
            .ok()
            .and_then(|v| v.as_string())
            .unwrap_or_default())
    }
}

#[cfg(test)]
#[path = "external_tests.rs"]
mod tests;
