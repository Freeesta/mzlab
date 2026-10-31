//! The session interface of a module, in plain Rust (the `wasm_bindgen` functions of `lib.rs` only convert): a reader stays open under a handle, and the engine's external reader asks it for the
//! index, a scan, a chromatogram, the header of a scan and the method. Answers are JSON, or one byte array for the index and a scan (`mzlab_core::trasporto`); an error is `{error_key, params}`.
use std::collections::HashMap;

use super::json_testo::{
    campioni_json, cromatogramma_json, cromatogrammi_json, intestazione_json, testata_json,
};
use mzlab_core::json::Out;
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
        let mut o = Out::default();
        o.raw("{\"formato\":")
            .str(l.formato().id())
            .raw(",\"campioni\":");
        campioni_json(&mut o, l.campioni());
        o.raw("}");
        Ok(std::mem::take(&mut o.0))
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
        Ok(intestazione_json(&self.lettore(h)?.intestazione(c, k)?))
    }
}

impl Sessioni {
    /// The chromatograms of sample `c` without their points.
    pub fn cromatogrammi(&mut self, h: u32, c: usize) -> Result<String> {
        Ok(cromatogrammi_json(&self.lettore(h)?.cromatogrammi(c)?))
    }

    /// One chromatogram with its points: `{id, kind, q1, q3, unit, time, intensity}`.
    pub fn cromatogramma(&mut self, h: u32, c: usize, id: &str) -> Result<String> {
        Ok(cromatogramma_json(&self.lettore(h)?.cromatogramma(c, id)?))
    }

    /// The `metodo/1` document of sample `c`.
    pub fn metodo(&mut self, h: u32, c: usize) -> Result<String> {
        self.lettore(h)?.metodo(c)
    }

    /// The spectra of sample `c` that a truncated file declares but does not hold (`Lettore::mancanti`).
    pub fn mancanti(&mut self, h: u32, c: usize) -> Result<usize> {
        Ok(self.lettore(h)?.mancanti(c))
    }

    /// The header of the mzML of sample `c` (`Lettore::testata_mzml`), `null` when the reader has none: `{run, sources, instrument:[acc,name]|null, components:[[kind,acc,name]],
    /// software:[name,version], acquired, serial, source_sha1, source_cv:[[acc,name]], sample:[id,name]|null, run_params:[[name,value]], content:[[acc,name]],
    /// extra_software:[[acc,name,version]], centroided}`.
    pub fn testata(&mut self, h: u32, c: usize) -> Result<String> {
        Ok(self
            .lettore(h)?
            .testata_mzml(c)?
            .map_or_else(|| "null".into(), |t| testata_json(&t)))
    }
}

#[cfg(test)]
#[path = "sessione_duro.rs"]
mod duro;

#[cfg(test)]
#[path = "sessione_prove.rs"]
mod tests;
