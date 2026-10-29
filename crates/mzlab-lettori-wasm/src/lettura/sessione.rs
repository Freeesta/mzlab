//! The session interface of a module, in plain Rust (the `wasm_bindgen` functions of `lib.rs` only convert): a reader stays open under a handle, and the engine's external reader asks it for the
//! index, a scan, a chromatogram, the header of a scan and the method. Answers are JSON, or one byte array for the index and a scan (`mzlab_core::trasporto`); an error is `{error_key, params}`.
use std::collections::HashMap;

use mzlab_core::lettori::Lettore;
use mzlab_core::trasporto::{encode_indice, encode_scansione};
use mzlab_core::Result;

#[derive(Default)]
pub struct Sessioni {
    aperti: HashMap<u32, Box<dyn Lettore>>,
    prossimo: u32,
}

impl Sessioni {
    /// Keeps `l` open and gives its handle.
    pub fn inserisci(&mut self, l: Box<dyn Lettore>) -> u32 {
        self.prossimo += 1;
        self.aperti.insert(self.prossimo, l);
        self.prossimo
    }

    pub fn chiudi(&mut self, h: u32) {
        self.aperti.remove(&h);
    }

    fn lettore(&mut self, h: u32) -> Result<&mut Box<dyn Lettore>> {
        self.aperti
            .get_mut(&h)
            .ok_or_else(|| mzlab_core::Error::new("error.badIndex").with("h", h))
    }

    /// `{formato, campioni:[{id,name,scans}]}` of the reader.
    pub fn campioni(&mut self, h: u32) -> Result<String> {
        let l = self.lettore(h)?;
        let c: Vec<_> = l
            .campioni()
            .iter()
            .map(|c| serde_json::json!({ "id": c.id, "name": c.nome, "scans": c.scansioni }))
            .collect();
        Ok(serde_json::json!({ "formato": l.formato().id(), "campioni": c }).to_string())
    }

    /// The index of sample `c` as one block of bytes in columns (`trasporto.rs`).
    pub fn indice(&mut self, h: u32, c: usize) -> Result<Vec<u8>> {
        Ok(encode_indice(self.lettore(h)?.indice(c)?))
    }

    /// Scan `k` of sample `c` as one block of bytes: a fixed head, the m/z, the intensities (`trasporto.rs`). The id and the filter are those of the index.
    pub fn scansione(&mut self, h: u32, c: usize, k: usize) -> Result<Vec<u8>> {
        Ok(encode_scansione(&self.lettore(h)?.scansione_dati(c, k)?))
    }

    pub fn intestazione(&mut self, h: u32, c: usize, k: usize) -> Result<String> {
        let v: Vec<_> = self
            .lettore(h)?
            .intestazione(c, k)?
            .iter()
            .map(|f| serde_json::json!({ "name": f.nome, "value": f.valore, "unit": f.unita, "cv": f.cv }))
            .collect();
        Ok(serde_json::Value::Array(v).to_string())
    }
}

impl Sessioni {
    /// The chromatograms of sample `c` without their points.
    pub fn cromatogrammi(&mut self, h: u32, c: usize) -> Result<String> {
        let v: Vec<_> = self.lettore(h)?.cromatogrammi(c)?.iter().map(|x| serde_json::json!({ "id": x.id, "kind": format!("{:?}", x.tipo), "unit": x.unita })).collect();
        Ok(serde_json::Value::Array(v).to_string())
    }

    /// One chromatogram with its points: `{id, kind, q1, q3, unit, time, intensity}`.
    pub fn cromatogramma(&mut self, h: u32, c: usize, id: &str) -> Result<String> {
        let x = self.lettore(h)?.cromatogramma(c, id)?;
        Ok(serde_json::json!({ "id": x.id, "kind": format!("{:?}", x.kind), "q1": x.q1, "q3": x.q3, "unit": x.unit, "time": x.time, "intensity": x.intensity }).to_string())
    }

    /// The `metodo/1` document of sample `c`.
    pub fn metodo(&mut self, h: u32, c: usize) -> Result<String> {
        self.lettore(h)?.metodo(c)
    }

    /// The header of the mzML of sample `c` (`Lettore::testata_mzml`), `null` when the reader has none: `{run, sources, instrument:[acc,name]|null, components:[[kind,acc,name]],
    /// software:[name,version], acquired, serial, source_sha1, source_cv:[[acc,name]], sample:[id,name]|null, run_params:[[name,value]], content:[[acc,name]],
    /// extra_software:[[acc,name,version]], centroided}`.
    pub fn testata(&mut self, h: u32, c: usize) -> Result<String> {
        Ok(match self.lettore(h)?.testata_mzml(c)? {
            None => "null".into(),
            Some(t) => serde_json::json!({
                "run": t.run, "sources": t.sources, "instrument": t.instrument, "components": t.components,
                "software": [t.software.0, t.software.1], "acquired": t.acquired, "serial": t.serial,
                "source_sha1": t.source_sha1, "source_cv": t.source_cv, "sample": t.sample,
                "run_params": t.run_params, "content": t.content, "extra_software": t.extra_software,
                "centroided": t.centroided,
            })
            .to_string(),
        })
    }
}

#[cfg(test)]
#[path = "sessione_prove.rs"]
mod tests;
