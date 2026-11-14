//! The session interface of a module, in plain Rust (the `wasm_bindgen` functions of `lib.rs` only convert): a reader stays open under a handle, and the engine's external reader asks it for the
//! index, a scan, a chromatogram, the header of a scan and the method. Answers are JSON, or one byte array for the index and a scan (`mzlab_core::transport`); an error is `{error_key, params}`.
use std::collections::HashMap;

use super::json_text::{chromatograms_json, fields_json, header_json, samples_json};
use mzlab_core::json::Out;
use mzlab_core::mzml::Spectrum;
use mzlab_core::readers::Reader;
use mzlab_core::transport::{encode_index, encode_scan};
use mzlab_core::Result;

#[derive(Default)]
pub struct Sessions {
    open: HashMap<u32, Box<dyn Reader>>,
    next: u32,
    // the last scan of `scan_parts`, whose columns the engine reads in place
    last: Option<Spectrum>,
}

impl Sessions {
    /// Keeps `l` open and gives its handle.
    pub fn insert(&mut self, l: Box<dyn Reader>) -> u32 {
        self.next += 1;
        self.open.insert(self.next, l);
        self.next
    }

    pub fn close(&mut self, h: u32) {
        self.open.remove(&h);
    }

    fn reader(&mut self, h: u32) -> Result<&mut Box<dyn Reader>> {
        self.open
            .get_mut(&h)
            .ok_or_else(|| mzlab_core::Error::new("error.badIndex").with("h", h))
    }

    /// `{formato, campioni:[{id,name,scans}]}` of the reader.
    pub fn samples(&mut self, h: u32) -> Result<String> {
        let l = self.reader(h)?;
        let mut o = Out::default();
        o.raw("{\"formato\":")
            .str(l.format().id())
            .raw(",\"campioni\":");
        samples_json(&mut o, l.samples());
        o.raw("}");
        Ok(std::mem::take(&mut o.0))
    }

    /// The index of sample `c` as one block of bytes in columns (`transport.rs`).
    pub fn index(&mut self, h: u32, c: usize) -> Result<Vec<u8>> {
        Ok(encode_index(self.reader(h)?.index(c)?))
    }

    /// Scan `k` of sample `c` as one block of bytes: a fixed head, the m/z, the intensities (`transport.rs`). The id and the filter are those of the index.
    pub fn scan(&mut self, h: u32, c: usize, k: usize) -> Result<Vec<u8>> {
        Ok(encode_scan(&self.reader(h)?.scan_data(c, k)?))
    }

    /// Scan `k` of sample `c` kept here until the next one: `scan_parts` hands over views of its columns instead of a block.
    pub fn scan_parts(&mut self, h: u32, c: usize, k: usize) -> Result<&Spectrum> {
        self.last = None;
        let s = self.reader(h)?.scan_data(c, k)?;
        Ok(self.last.insert(s))
    }

    pub fn scan_header(&mut self, h: u32, c: usize, k: usize) -> Result<String> {
        Ok(fields_json(&self.reader(h)?.scan_header(c, k)?))
    }
}

impl Sessions {
    /// The chromatograms of sample `c` without their points.
    pub fn chromatograms(&mut self, h: u32, c: usize) -> Result<String> {
        Ok(chromatograms_json(&self.reader(h)?.chromatograms(c)?))
    }

    /// One chromatogram with its points: `{id, kind, q1, q3, unit, time, intensity}`.
    pub fn chromatogram(&mut self, h: u32, c: usize, id: &str) -> Result<String> {
        Ok(mzlab_lettori::json::chromatogram_json(
            &self.reader(h)?.chromatogram(c, id)?,
        ))
    }

    /// The `metodo/1` document of sample `c`.
    pub fn method(&mut self, h: u32, c: usize) -> Result<String> {
        self.reader(h)?.method(c)
    }

    /// The spectra of sample `c` the file declares beyond the ones read (`Reader::losses`).
    pub fn missing(&mut self, h: u32, c: usize) -> Result<usize> {
        Ok(self.reader(h)?.losses(c)?.lost())
    }

    /// The key of why sample `c` has scans `missing` (`Losses::cause`), empty when none.
    pub fn cause(&mut self, h: u32, c: usize) -> Result<String> {
        Ok(self.reader(h)?.losses(c)?.cause.to_string())
    }

    /// The header of the mzML of sample `c` (`Lettore::mzml_header`), `null` when the reader has none: `{run, sources, instrument:[acc,name]|null, components:[[kind,acc,name]],
    /// software:[name,version], acquired, serial, source_sha1, source_cv:[[acc,name]], sample:[id,name]|null, run_params:[[name,value]], content:[[acc,name]],
    /// extra_software:[[acc,name,version]], centroided}`.
    pub fn file_header(&mut self, h: u32, c: usize) -> Result<String> {
        Ok(self
            .reader(h)?
            .mzml_header(c)?
            .map_or_else(|| "null".into(), |t| header_json(&t)))
    }
}

#[cfg(test)]
#[path = "session_hard.rs"]
mod hard;

#[cfg(test)]
#[path = "session_tests.rs"]
mod tests;
