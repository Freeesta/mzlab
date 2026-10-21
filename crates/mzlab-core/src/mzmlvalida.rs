//! The mzML validator: structure (required elements, `count`, array lengths, order of m/z), the PSI-MS vocabulary (accession, name, unit, required
//! terms) and, for `indexedmzML`, the offsets, `indexListOffset` and the SHA-1 `fileChecksum`. One streaming pass, nothing of the file kept but the
//! spectrum being read (and the index, which is as long as the list of spectra). The same pass hands each spectrum and chromatogram to the caller
//! (`analizza`), which is how `examples/mzml_confronta.rs` reads two files. Native only: the page never validates.
//! `mzlab-cli validate` is the command; the vocabulary is `data/psims-cv.json` (`tools/psims_tabella.py`).

mod array;
mod cv;
mod elementi;
mod indice;
mod struttura;
#[cfg(test)]
mod tests;
#[cfg(test)]
mod tests_rotti;

use std::io::Read;

use crate::error::Result;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Severity {
    Error,
    Warning,
}

impl Severity {
    pub fn as_str(self) -> &'static str {
        match self {
            Severity::Error => "error",
            Severity::Warning => "warning",
        }
    }
}

/// One finding: where in the XML (`/indexedmzML/mzML/run/spectrumList/spectrum[3]/...`), a short stable `code` and the sentence.
#[derive(Debug, Clone, PartialEq)]
pub struct Problem {
    pub severity: Severity,
    pub path: String,
    pub code: &'static str,
    pub message: String,
}

/// A spectrum as the file holds it, for the callers that compare files.
#[derive(Debug, Clone, Default)]
pub struct SpecInfo {
    pub index: usize,
    pub id: String,
    pub level: u8,
    /// +1, -1 or 0 (not said)
    pub polarity: i8,
    /// Scan start time in minutes.
    pub rt: Option<f64>,
    /// (selected ion m/z, charge) of each precursor.
    pub precursors: Vec<(Option<f64>, Option<i32>)>,
    pub mz: Vec<f64>,
    pub intensity: Vec<f64>,
}

#[derive(Debug, Clone, Default)]
pub struct ChromInfo {
    pub index: usize,
    pub id: String,
    /// Accession of the chromatogram type (`MS:1000235` TIC, ...).
    pub kind: String,
    pub q1: Option<f64>,
    pub q3: Option<f64>,
    pub time: Vec<f64>,
    pub intensity: Vec<f64>,
}

pub enum Item {
    Spectrum(SpecInfo),
    Chromatogram(ChromInfo),
}

#[derive(Debug, Default)]
pub struct Report {
    pub problems: Vec<Problem>,
    pub spectra: usize,
    pub chromatograms: usize,
    /// The file is an `indexedmzML`.
    pub indexed: bool,
}

impl Report {
    pub fn errors(&self) -> usize {
        self.problems
            .iter()
            .filter(|p| p.severity == Severity::Error)
            .count()
    }

    pub fn is_valid(&self) -> bool {
        self.errors() == 0
    }
}

/// Reads the whole mzML once, calling `sink` for every spectrum and chromatogram in file order. An error is a read that failed; a malformed file is a
/// `Problem` of the report.
pub fn analizza<R: Read>(reader: R, sink: &mut dyn FnMut(Item)) -> Result<Report> {
    struttura::run(reader, sink)
}

/// `analizza` with nobody listening.
pub fn valida<R: Read>(reader: R) -> Result<Report> {
    analizza(reader, &mut |_| {})
}

/// The number of terms in the vocabulary table.
pub fn termini() -> usize {
    cv::get().len()
}
