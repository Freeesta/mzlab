//! A reader that lives in a readers module (`wasm-lettori/<family>/`, docs/agenti/lettori-modulo.md) and not in the engine: `LettoreEsterno` is a `Lettore` over the synchronous
//! functions of the session interface of the module, which the page loads in the same worker (`rust-nativo.js`). The conversion of the answers (JSON, typed arrays) into the types of
//! the contract is in `Risposte` and does not touch JS, so it is tried natively with a fake `Modulo`; `ModuloJs` is the one that calls the functions. The index and the scans cross as
//! one byte array each (`mzlab_core::trasporto`), the rest as JSON.
use std::cell::RefCell;
use std::collections::{HashMap, HashSet};

use js_sys::{Array, Float32Array, Float64Array, Function, Reflect, Uint8Array};
use mzlab_core::error::{Error, Result};
use mzlab_core::json::{self, Value};
use mzlab_core::lettori::{Campione, Campo, Formato, Lettore, VoceCromatogramma, VoceScansione};
use mzlab_core::mzml::{ChromKind, Chromatogram, Spectrum};
use mzlab_core::mzmlwrite::Header;
use mzlab_core::trasporto::{
    decode_indice, decode_scansione, scansione_da_parti, Intensita, PROTOCOLLO,
};
use wasm_bindgen::prelude::*;

/// The session interface of a module, as the engine uses it: JSON texts, and the index and the scans as bytes. An error is the `Error` of the module (same key).
pub trait Modulo {
    /// The version of the transport the module speaks (`trasporto::PROTOCOLLO`).
    fn protocol(&mut self) -> Result<u32>;
    fn samples(&mut self) -> Result<String>;
    /// The index of sample `c` (`trasporto::encode_indice`).
    fn index(&mut self, c: usize) -> Result<Vec<u8>>;
    /// Scan `k` of sample `c` (`trasporto::encode_scansione`).
    fn scan(&mut self, c: usize, k: usize) -> Result<Vec<u8>>;
    /// Scan `k` of sample `c` decoded: from the block of `scan`, unless the module hands its columns over apart (`ModuloJs`).
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
}

fn guasto(what: &str) -> Error {
    Error::new("err.file.notAccepted").with("detail", what)
}

thread_local! { static NOMI: RefCell<HashSet<&'static str>> = RefCell::new(HashSet::new()); }

/// The text as a `&'static str` (the keys and the names of the parameters of `Error` are static): each distinct text is kept once.
fn statica(s: &str) -> &'static str {
    NOMI.with(|n| {
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
pub(crate) fn errore_del_modulo(e: &str) -> Error {
    let Ok(v) = json::parse(e.as_bytes()) else {
        return guasto("module");
    };
    let Some(key) = v.get("error_key").and_then(Value::str) else {
        return guasto("module");
    };
    let mut err = Error::new(statica(key));
    if let Some(Value::Obj(p)) = v.get("params") {
        for (k, x) in p {
            err = err.with(statica(k), x.str().unwrap_or_default());
        }
    }
    err
}

fn doc(t: &str) -> Result<Value> {
    json::parse(t.as_bytes())
}

fn testo(v: Option<&Value>) -> String {
    v.and_then(Value::str).unwrap_or_default().to_string()
}

fn coppie(v: Option<&Value>) -> Vec<(String, String)> {
    v.map(Value::arr)
        .unwrap_or_default()
        .iter()
        .map(|p| (testo(p.at(0)), testo(p.at(1))))
        .collect()
}

fn format(id: &str) -> Formato {
    match id {
        "thermo-raw" => Formato::ThermoRaw,
        "sciex-wiff" => Formato::SciexWiff,
        "sciex-dam" => Formato::SciexDam,
        "agilent-d" => Formato::AgilentD,
        "mzml" => Formato::Mzml,
        _ => Formato::Altro(statica(id)),
    }
}

fn tipo(nome: &str) -> ChromKind {
    match nome {
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

fn numero(v: Option<&Value>) -> f64 {
    v.and_then(Value::f64).unwrap_or(f64::NAN)
}

fn opzionale(v: Option<&Value>) -> Option<f64> {
    v.and_then(Value::f64)
}

fn numeri(v: Option<&Value>) -> Vec<f64> {
    v.map(Value::arr)
        .unwrap_or_default()
        .iter()
        .map(|x| x.f64().unwrap_or(f64::NAN))
        .collect()
}

/// The conversions of the answers of a module into the types of the contract.
pub(crate) struct Risposte;

impl Risposte {
    /// `{formato, campioni:[{id,name,scans}]}`.
    pub(crate) fn samples(t: &str) -> Result<(Formato, Vec<Campione>)> {
        let v = doc(t)?;
        let c = v
            .get("campioni")
            .map(Value::arr)
            .unwrap_or_default()
            .iter()
            .map(|c| Campione {
                id: numero(c.get("id")) as u32,
                nome: testo(c.get("name")),
                scansioni: numero(c.get("scans")) as usize,
                dimensione: c.get("size").and_then(Value::f64).map(|d| d as u64),
            })
            .collect::<Vec<_>>();
        if c.is_empty() {
            return Err(guasto("samples"));
        }
        Ok((format(&testo(v.get("formato"))), c))
    }

    /// The index as the module wrote it (`trasporto::decode_indice`).
    pub(crate) fn index(b: &[u8]) -> Result<Vec<VoceScansione>> {
        decode_indice(b)
    }
}

/// The answers about the scans and their headers.
pub struct RisposteScansioni;

impl RisposteScansioni {
    /// The scan as the module sends it, with the id and the filter of its row of the index.
    pub(crate) fn scan(mut s: Spectrum, riga: Option<&VoceScansione>) -> Spectrum {
        if let Some(r) = riga {
            s.id = r.id.clone();
            s.filter = r.filtro.clone();
        }
        s
    }

    /// `[{name, value, unit, cv}]`.
    pub(crate) fn scan_header(t: &str) -> Result<Vec<Campo>> {
        let opz = |v: Option<&Value>| v.and_then(Value::str).map(str::to_string);
        Ok(doc(t)?
            .arr()
            .iter()
            .map(|f| Campo {
                nome: testo(f.get("name")),
                valore: testo(f.get("value")),
                unita: opz(f.get("unit")),
                cv: opz(f.get("cv")),
            })
            .collect())
    }

    /// `[{id, kind, unit}]` (`kind` is the name of the `ChromKind`).
    pub(crate) fn chromatograms(t: &str) -> Result<Vec<VoceCromatogramma>> {
        Ok(doc(t)?
            .arr()
            .iter()
            .map(|x| VoceCromatogramma {
                id: testo(x.get("id")),
                tipo: tipo(&testo(x.get("kind"))),
                unita: testo(x.get("unit")),
            })
            .collect())
    }
}

/// The answers about the chromatograms and the header of the mzML.
pub struct RisposteTracce;

impl RisposteTracce {
    /// `{id, kind, q1, q3, unit, time, intensity}`.
    pub(crate) fn chromatogram(t: &str) -> Result<Chromatogram> {
        let x = doc(t)?;
        Ok(Chromatogram {
            index: 0,
            id: testo(x.get("id")),
            kind: tipo(&testo(x.get("kind"))),
            q1: opzionale(x.get("q1")),
            q3: opzionale(x.get("q3")),
            unit: testo(x.get("unit")),
            time: numeri(x.get("time")),
            intensity: numeri(x.get("intensity")),
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
                .map(|p| (testo(p.at(0)), testo(p.at(1))))
        };
        let strs = |k: &str| {
            v.get(k)
                .map(Value::arr)
                .unwrap_or_default()
                .iter()
                .map(|s| s.str().unwrap_or_default().to_string())
                .collect::<Vec<_>>()
        };
        let opz = |k: &str| v.get(k).and_then(Value::str).map(str::to_string);
        Ok(Some(Header {
            run: testo(v.get("run")),
            sources: strs("sources"),
            instrument: pair("instrument"),
            components: v
                .get("components")
                .map(Value::arr)
                .unwrap_or_default()
                .iter()
                .map(|c| (testo(c.at(0)), testo(c.at(1)), testo(c.at(2))))
                .collect(),
            software: pair("software").unwrap_or_default(),
            acquired: opz("acquired"),
            serial: opz("serial"),
            source_sha1: strs("source_sha1"),
            source_cv: coppie(v.get("source_cv")),
            sample: pair("sample"),
            run_params: coppie(v.get("run_params")),
            content: coppie(v.get("content")),
            extra_software: v
                .get("extra_software")
                .map(Value::arr)
                .unwrap_or_default()
                .iter()
                .map(|c| (testo(c.at(0)), testo(c.at(1)), testo(c.at(2))))
                .collect(),
            centroided: v.get("centroided").and_then(Value::bool).unwrap_or(false),
            ..Header::default()
        }))
    }
}

/// A `Lettore` over a `Modulo`.
pub struct LettoreEsterno<M: Modulo> {
    m: M,
    formato: Formato,
    campioni: Vec<Campione>,
    indici: HashMap<usize, Vec<VoceScansione>>,
}

impl<M: Modulo> LettoreEsterno<M> {
    pub fn new(mut m: M) -> Result<Self> {
        // a module cached from another version of the site speaks another transport: the page asks to load it again
        if m.protocol()? != PROTOCOLLO {
            return Err(Error::new("err.file.wiffLoad")
                .with("family", "thermo")
                .with("detail", "protocol"));
        }
        let (formato, campioni) = Risposte::samples(&m.samples()?)?;
        Ok(LettoreEsterno {
            m,
            formato,
            campioni,
            indici: HashMap::new(),
        })
    }
}

impl<M: Modulo> Lettore for LettoreEsterno<M> {
    fn format(&self) -> Formato {
        self.formato
    }
    fn samples(&self) -> &[Campione] {
        &self.campioni
    }
    fn index(&mut self, c: usize) -> Result<&[VoceScansione]> {
        if !self.indici.contains_key(&c) {
            let r = Risposte::index(&self.m.index(c)?)?;
            self.indici.insert(c, r);
        }
        Ok(self.indici.get(&c).map_or(&[][..], Vec::as_slice))
    }
    fn scan(&mut self, c: usize, k: usize) -> Result<Spectrum> {
        let s = self.m.scan_spectrum(c, k)?;
        self.index(c)?;
        Ok(RisposteScansioni::scan(
            s,
            self.indici.get(&c).and_then(|v| v.get(k)),
        ))
    }
    fn scan_header(&mut self, c: usize, k: usize) -> Result<Vec<Campo>> {
        RisposteScansioni::scan_header(&self.m.scan_header(c, k)?)
    }
    fn chromatograms(&mut self, c: usize) -> Result<Vec<VoceCromatogramma>> {
        RisposteScansioni::chromatograms(&self.m.chromatograms(c)?)
    }
    fn chromatogram(&mut self, c: usize, id: &str) -> Result<Chromatogram> {
        RisposteTracce::chromatogram(&self.m.chromatogram(c, id)?)
    }
    fn method(&mut self, c: usize) -> Result<String> {
        self.m.method(c)
    }
    fn missing(&mut self, c: usize) -> usize {
        self.m.missing(c).unwrap_or(0) as usize
    }
    fn mzml_header(&mut self, c: usize) -> Result<Option<Header>> {
        RisposteTracce::file_header(&self.m.file_header(c)?)
    }
}

/// The functions of a session of the module, as an object of JS functions `{samples, index, scan, scan_header, chromatograms, chromatogram, method, file_header, missing}`
/// that already hold the handle of the session: `samples()`, `index(c)`, `scan(c, k)`, ...
pub(crate) struct ModuloJs {
    sessione: JsValue,
    // `scan_parts` of a module that hands a scan over as its head and two views of its columns (an older build has only `scan`)
    parti: Option<Function>,
}

impl ModuloJs {
    pub(crate) fn new(sessione: JsValue) -> Self {
        let parti = Reflect::get(&sessione, &JsValue::from_str("scan_parts"))
            .ok()
            .and_then(|f| f.dyn_into().ok());
        ModuloJs { sessione, parti }
    }

    fn chiama(&self, nome: &str, args: &[JsValue]) -> Result<JsValue> {
        let f: Function = Reflect::get(&self.sessione, &JsValue::from_str(nome))
            .ok()
            .and_then(|f| f.dyn_into().ok())
            .ok_or_else(|| guasto(nome))?;
        let a: Array = args.iter().collect();
        f.apply(&self.sessione, &a).map_err(|e| dal_modulo(nome, e))
    }

    /// A `Uint8Array` answer, copied once into the memory of the engine.
    fn byte(&self, nome: &str, args: &[JsValue]) -> Result<Vec<u8>> {
        let a: Uint8Array = self
            .chiama(nome, args)?
            .dyn_into()
            .map_err(|_| guasto(nome))?;
        Ok(a.to_vec())
    }

    fn testo(&self, nome: &str, args: &[JsValue]) -> Result<String> {
        self.chiama(nome, args)?
            .as_string()
            .ok_or_else(|| guasto(nome))
    }
}

impl Drop for ModuloJs {
    /// Frees the session of the module.
    fn drop(&mut self) {
        drop(self.chiama("close", &[]));
    }
}

/// The error a module threw: its JSON, or the key of a broken module.
fn dal_modulo(nome: &str, e: JsValue) -> Error {
    match e.as_string() {
        Some(t) => errore_del_modulo(&t),
        None => guasto(nome),
    }
}

/// The scan from `scan_parts`: the columns are views of the module's memory, valid until its next call, so each is copied here at once, in one piece.
fn dalle_parti(k: usize, r: JsValue) -> Result<Spectrum> {
    let a: Array = r.dyn_into().map_err(|_| guasto("scan"))?;
    let testa: Uint8Array = a.get(0).dyn_into().map_err(|_| guasto("scan"))?;
    let mz: Float64Array = a.get(1).dyn_into().map_err(|_| guasto("scan"))?;
    let mz = mz.to_vec();
    let y = a.get(2);
    let y = match y.dyn_ref::<Float64Array>() {
        Some(w) => Intensita::Larghe(w.to_vec()),
        None => Intensita::Strette(
            y.dyn_into::<Float32Array>()
                .map_err(|_| guasto("scan"))?
                .to_vec(),
        ),
    };
    scansione_da_parti(k, &testa.to_vec(), mz, y)
}

impl Modulo for ModuloJs {
    fn protocol(&mut self) -> Result<u32> {
        self.chiama("protocol", &[])?
            .as_f64()
            .map(|v| v as u32)
            .ok_or_else(|| guasto("protocol"))
    }
    fn samples(&mut self) -> Result<String> {
        self.testo("samples", &[])
    }
    fn index(&mut self, c: usize) -> Result<Vec<u8>> {
        self.byte("index", &[(c as f64).into()])
    }
    fn scan(&mut self, c: usize, k: usize) -> Result<Vec<u8>> {
        self.byte("scan", &[(c as f64).into(), (k as f64).into()])
    }
    fn scan_spectrum(&mut self, c: usize, k: usize) -> Result<Spectrum> {
        let Some(f) = &self.parti else {
            return decode_scansione(k, &self.scan(c, k)?);
        };
        let r = f
            .call2(&self.sessione, &(c as f64).into(), &(k as f64).into())
            .map_err(|e| dal_modulo("scan", e))?;
        dalle_parti(k, r)
    }
    fn scan_header(&mut self, c: usize, k: usize) -> Result<String> {
        self.testo("scan_header", &[(c as f64).into(), (k as f64).into()])
    }
    fn chromatograms(&mut self, c: usize) -> Result<String> {
        self.testo("chromatograms", &[(c as f64).into()])
    }
    fn chromatogram(&mut self, c: usize, id: &str) -> Result<String> {
        self.testo("chromatogram", &[(c as f64).into(), id.into()])
    }
    fn method(&mut self, c: usize) -> Result<String> {
        self.testo("method", &[(c as f64).into()])
    }
    fn file_header(&mut self, c: usize) -> Result<String> {
        self.testo("file_header", &[(c as f64).into()])
    }
    fn missing(&mut self, c: usize) -> Result<u32> {
        // a session of an older module has no such function
        Ok(self
            .chiama("missing", &[(c as f64).into()])
            .ok()
            .and_then(|v| v.as_f64())
            .map_or(0, |v| v as u32))
    }
}

#[cfg(test)]
#[path = "esterno_prove.rs"]
mod tests;
