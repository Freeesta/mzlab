//! A reader that lives in a readers module (`wasm-lettori/<family>/`, docs/agenti/lettori-modulo.md) and not in the engine: `LettoreEsterno` is a `Lettore` over the synchronous
//! functions of the session interface of the module, which the page loads in the same worker (`rust-nativo.js`). The conversion of the answers (JSON, typed arrays) into the types of
//! the contract is in `Risposte` and does not touch JS, so it is tried natively with a fake `Modulo`; `ModuloJs` is the one that calls the functions.
use std::cell::RefCell;
use std::collections::{HashMap, HashSet};

use js_sys::{Array, Float64Array, Function, Reflect};
use mzlab_core::error::{Error, Result};
use mzlab_core::json::{self, Value};
use mzlab_core::lettori::{
    Campione, Campo, Formato, Lettore, Posizione, VoceCromatogramma, VoceScansione,
};
use mzlab_core::mzml::{ChromKind, Chromatogram, Spectrum};
use mzlab_core::mzmlwrite::Header;
use wasm_bindgen::prelude::*;

/// The session interface of a module, as the engine uses it: JSON texts and the two arrays of a scan. An error is the `Error` of the module (same key).
pub trait Modulo {
    fn campioni(&mut self) -> Result<String>;
    fn indice(&mut self, c: usize) -> Result<String>;
    /// `(m/z, intensity, meta JSON)`.
    fn scansione(&mut self, c: usize, k: usize) -> Result<(Vec<f64>, Vec<f64>, String)>;
    fn intestazione(&mut self, c: usize, k: usize) -> Result<String>;
    fn cromatogrammi(&mut self, c: usize) -> Result<String>;
    fn cromatogramma(&mut self, c: usize, id: &str) -> Result<String>;
    fn metodo(&mut self, c: usize) -> Result<String>;
    fn testata(&mut self, c: usize) -> Result<String>;
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

fn formato(id: &str) -> Formato {
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
    v.map(Value::arr).unwrap_or_default().iter().map(|x| x.f64().unwrap_or(f64::NAN)).collect()
}

/// The conversions of the answers of a module into the types of the contract.
pub(crate) struct Risposte;

impl Risposte {
    /// `{formato, campioni:[{id,name,scans}]}`.
    pub(crate) fn campioni(t: &str) -> Result<(Formato, Vec<Campione>)> {
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
            })
            .collect::<Vec<_>>();
        if c.is_empty() {
            return Err(guasto("samples"));
        }
        Ok((formato(&testo(v.get("formato"))), c))
    }

    /// One object per scan: `{id, rt, level, polarity, tic, base, filter, precursor, pos:[member, offset, length]|null}`.
    pub(crate) fn indice(t: &str) -> Result<Vec<VoceScansione>> {
        let v = doc(t)?;
        Ok(v.arr()
            .iter()
            .map(|r| VoceScansione {
                id: testo(r.get("id")),
                rt: numero(r.get("rt")),
                livello: numero(r.get("level")) as u8,
                polarita: numero(r.get("polarity")) as i8,
                tic: numero(r.get("tic")),
                picco_base: opzionale(r.get("base")),
                filtro: testo(r.get("filter")),
                precursore: opzionale(r.get("precursor")),
                posizione: r.get("pos").filter(|p| !p.is_null()).map(|p| Posizione {
                    membro: numero(p.at(0)) as u16,
                    offset: numero(p.at(1)) as u64,
                    lunghezza: numero(p.at(2)) as u32,
                }),
            })
            .collect())
    }

}

/// The answers about the scans and their headers.
pub struct RisposteScansioni;

impl RisposteScansioni {
    /// The scan: points as the module sends them and `{id, level, rt, polarity, profile, filter, precursor}`.
    pub(crate) fn scansione(k: usize, mz: Vec<f64>, y: Vec<f64>, meta: &str) -> Result<Spectrum> {
        if mz.len() != y.len() {
            return Err(guasto("scan"));
        }
        let m = doc(meta)?;
        let intensity: Vec<f32> = y.iter().map(|&x| x as f32).collect();
        // 64-bit intensities that a 32-bit one cannot hold are kept as they are
        let wide = y
            .iter()
            .zip(&intensity)
            .any(|(&a, &b)| a != f64::from(b) && !(a.is_nan() && b.is_nan()))
            .then_some(y);
        Ok(Spectrum {
            index: k,
            id: testo(m.get("id")),
            level: numero(m.get("level")) as u8,
            rt: numero(m.get("rt")),
            polarity: numero(m.get("polarity")) as i8,
            profile: m.get("profile").and_then(Value::bool).unwrap_or(false),
            filter: testo(m.get("filter")),
            precursor: opzionale(m.get("precursor")),
            mz,
            intensity,
            wide,
        })
    }

    /// `[{name, value, unit, cv}]`.
    pub(crate) fn intestazione(t: &str) -> Result<Vec<Campo>> {
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
    pub(crate) fn cromatogrammi(t: &str) -> Result<Vec<VoceCromatogramma>> {
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
    pub(crate) fn cromatogramma(t: &str) -> Result<Chromatogram> {
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

    /// The header of the mzML (`null`: the reader has none); the fields are those of `Sessioni::testata` of the module.
    pub(crate) fn testata(t: &str) -> Result<Option<Header>> {
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
        let (formato, campioni) = Risposte::campioni(&m.campioni()?)?;
        Ok(LettoreEsterno {
            m,
            formato,
            campioni,
            indici: HashMap::new(),
        })
    }
}

impl<M: Modulo> Lettore for LettoreEsterno<M> {
    fn formato(&self) -> Formato {
        self.formato
    }
    fn campioni(&self) -> &[Campione] {
        &self.campioni
    }
    fn indice(&mut self, c: usize) -> Result<&[VoceScansione]> {
        if !self.indici.contains_key(&c) {
            let r = Risposte::indice(&self.m.indice(c)?)?;
            self.indici.insert(c, r);
        }
        Ok(self.indici.get(&c).map_or(&[][..], Vec::as_slice))
    }
    fn scansione(&mut self, c: usize, k: usize) -> Result<Spectrum> {
        let (mz, y, meta) = self.m.scansione(c, k)?;
        RisposteScansioni::scansione(k, mz, y, &meta)
    }
    fn intestazione(&mut self, c: usize, k: usize) -> Result<Vec<Campo>> {
        RisposteScansioni::intestazione(&self.m.intestazione(c, k)?)
    }
    fn cromatogrammi(&mut self, c: usize) -> Result<Vec<VoceCromatogramma>> {
        RisposteScansioni::cromatogrammi(&self.m.cromatogrammi(c)?)
    }
    fn cromatogramma(&mut self, c: usize, id: &str) -> Result<Chromatogram> {
        RisposteTracce::cromatogramma(&self.m.cromatogramma(c, id)?)
    }
    fn metodo(&mut self, c: usize) -> Result<String> {
        self.m.metodo(c)
    }
    fn testata_mzml(&mut self, c: usize) -> Result<Option<Header>> {
        RisposteTracce::testata(&self.m.testata(c)?)
    }
}

/// The functions of a session of the module, as an object of JS functions `{campioni, indice, scansione, intestazione, cromatogrammi, cromatogramma, metodo, testata}`
/// that already hold the handle of the session: `campioni()`, `indice(c)`, `scansione(c, k)`, ...
pub(crate) struct ModuloJs {
    sessione: JsValue,
}

impl ModuloJs {
    pub(crate) fn new(sessione: JsValue) -> Self {
        ModuloJs { sessione }
    }

    fn chiama(&self, nome: &str, args: &[JsValue]) -> Result<JsValue> {
        let f: Function = Reflect::get(&self.sessione, &JsValue::from_str(nome))
            .ok()
            .and_then(|f| f.dyn_into().ok())
            .ok_or_else(|| guasto(nome))?;
        let a: Array = args.iter().collect();
        f.apply(&self.sessione, &a).map_err(|e| match e.as_string() {
            Some(t) => errore_del_modulo(&t),
            None => guasto(nome),
        })
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
        drop(self.chiama("chiudi", &[]));
    }
}

impl Modulo for ModuloJs {
    fn campioni(&mut self) -> Result<String> {
        self.testo("campioni", &[])
    }
    fn indice(&mut self, c: usize) -> Result<String> {
        self.testo("indice", &[(c as f64).into()])
    }
    fn scansione(&mut self, c: usize, k: usize) -> Result<(Vec<f64>, Vec<f64>, String)> {
        let r = self.chiama("scansione", &[(c as f64).into(), (k as f64).into()])?;
        let a: Array = r.dyn_into().map_err(|_| guasto("scansione"))?;
        let col = |i: u32| -> Result<Vec<f64>> {
            let f: Float64Array = a.get(i).dyn_into().map_err(|_| guasto("scansione"))?;
            Ok(f.to_vec())
        };
        let meta = a.get(2).as_string().ok_or_else(|| guasto("scansione"))?;
        Ok((col(0)?, col(1)?, meta))
    }
    fn intestazione(&mut self, c: usize, k: usize) -> Result<String> {
        self.testo("intestazione", &[(c as f64).into(), (k as f64).into()])
    }
    fn cromatogrammi(&mut self, c: usize) -> Result<String> {
        self.testo("cromatogrammi", &[(c as f64).into()])
    }
    fn cromatogramma(&mut self, c: usize, id: &str) -> Result<String> {
        self.testo("cromatogramma", &[(c as f64).into(), id.into()])
    }
    fn metodo(&mut self, c: usize) -> Result<String> {
        self.testo("metodo", &[(c as f64).into()])
    }
    fn testata(&mut self, c: usize) -> Result<String> {
        self.testo("testata", &[(c as f64).into()])
    }
}

#[cfg(test)]
#[path = "esterno_prove.rs"]
mod tests;
