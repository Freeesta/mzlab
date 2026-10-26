//! mzML writer over any `Write`, spectrum by spectrum, for readers of vendor files that have no mzML writer of their own (`wiff::to_mzml`).
//! What it writes is what `mzml::read` and `mzlab/reader/mzml.py` read: spectra (level, polarity, profile or centroid, RT in minutes, precursor,
//! m/z as 64-bit floats, intensities as 64-bit when the spectrum has `wide`, otherwise 32-bit, both zlib), and TIC, BPC and SRM chromatograms
//! (the SRM masses are in the `id`, as msconvert writes them, and as isolation windows). No index unless `MzmlWriter::indicizzato`. Numpress only when asked (`Opzioni::numpress`: lossy).

use std::io::Write;

use crate::error::{Error, Result};
use crate::mzml::{ChromKind, Chromatogram, Spectrum};

/// The version of the PSI-MS CV every file declares, indexed or not.
const CV_VERSION: &str = "4.2.6";

/// The zlib level of the arrays. Level 1 is the fastest and 6 the default of zlib: on the synthetic spectra of `examples/mzml_scrivi.rs` level 1 writes 2.3 times faster than 6 and the file is 0.3 % larger.
pub const LIVELLO_ZLIB: u32 = 1;

/// How the arrays are written.
#[derive(Debug, Clone, Copy)]
pub struct Opzioni {
    /// zlib level 0-9 (`LIVELLO_ZLIB`).
    pub livello: u32,
    /// m/z of the spectra with MS-Numpress linear prediction (MS:1002312) and intensities with short logged float (MS:1002314), each followed by zlib (MS:1002746,
    /// MS:1002748) when that is smaller. Lossy: the relative error of the intensities is about 1e-4 and the m/z are rounded to 1e-7 of their range's fixed point.
    pub numpress: bool,
    /// Threads that encode the arrays when the spectra come through `MzmlWriter::spettri` (`thread_predefinito`); 0 and 1 encode on the calling thread. The bytes written do not depend on it.
    pub thread: usize,
}

impl Default for Opzioni {
    fn default() -> Self {
        Opzioni { livello: LIVELLO_ZLIB, numpress: false, thread: thread_predefinito() }
    }
}

impl<W: Write> MzmlWriter<W> {
    /// Writes the header and opens `<run>`.
    pub fn new(out: W, h: &Header) -> Result<Self> {
        Self::con_opzioni(out, h, Opzioni::default())
    }

    /// As `new`, with the zlib level and the numpress choice of `opz`.
    pub fn con_opzioni(out: W, h: &Header, opz: Opzioni) -> Result<Self> {
        Self::avvia(out, h, opz, false)
    }

    fn avvia(out: W, h: &Header, opz: Opzioni, indexed: bool) -> Result<Self> {
        let indexed = indexed && cfg!(not(target_family = "wasm"));
        let mut s = String::from("<?xml version=\"1.0\" encoding=\"utf-8\"?>\n");
        if indexed { s += "<indexedmzML xmlns=\"http://psi.hupo.org/ms/mzml\" xmlns:xsi=\"http://www.w3.org/2001/XMLSchema-instance\" xsi:schemaLocation=\"http://psi.hupo.org/ms/mzml http://psi.hupo.org/ms/mzml/mzML1.1.2_idx.xsd\">\n"; }
        s += "<mzML xmlns=\"http://psi.hupo.org/ms/mzml\" version=\"1.1.0\" id=\"mzlab\">\n";
        s += &format!("<cvList count=\"2\"><cv id=\"MS\" fullName=\"Proteomics Standards Initiative Mass Spectrometry Ontology\" version=\"{CV_VERSION}\" URI=\"https://raw.githubusercontent.com/HUPO-PSI/psi-ms-CV/master/psi-ms.obo\"/><cv id=\"UO\" fullName=\"Unit Ontology\" version=\"09:04:2014\" URI=\"https://raw.githubusercontent.com/bio-ontology-research-group/unit-ontology/master/unit.obo\"/></cvList>\n");
        let content = cv("MS:1000579", "MS1 spectrum", "") + &h.content.iter().map(|(a, n)| cv(a, n, "")).collect::<String>();
        s += &format!("<fileDescription><fileContent>{content}</fileContent>{}</fileDescription>\n", source_list(h));
        if let Some((id, name)) = &h.sample {
            s += &format!("<sampleList count=\"1\"><sample id=\"{}\" name=\"{}\"/></sampleList>\n", esc(id), esc(name));
        }
        s += &format!("<softwareList count=\"{}\"><software id=\"mzlab\" version=\"{}\">{}</software>{}</softwareList>\n", 1 + h.centroided as usize + h.extra_software.len() + h.steps.len(), esc(&h.software.1), cv("MS:1000799", "custom unreleased software tool", esc(&h.software.0)),
            h.extra_software.iter().enumerate().map(|(i, (a, n, v))| format!("<software id=\"acq{}\" version=\"{}\">{}</software>", i + 1, esc(v), cv(a, n, ""))).collect::<String>() + &if h.centroided { format!("<software id=\"mzlab-centroidi\" version=\"{}\">{}</software>", esc(&h.software.1), cv("MS:1000799", "custom unreleased software tool", "mzLab centroidi")) } else { String::new() } + &step_software(&h.steps));
        s += "<instrumentConfigurationList count=\"1\"><instrumentConfiguration id=\"IC1\">";
        if let Some((acc, name)) = &h.instrument {
            s += &cv(acc, name, "");
        }
        if let Some(sn) = &h.serial {
            s += &cv("MS:1000529", "instrument serial number", esc(sn));
        }
        s += &component_list(h);
        s += "<softwareRef ref=\"mzlab\"/></instrumentConfiguration></instrumentConfigurationList>\n";
        s += "<dataProcessingList count=\"1\"><dataProcessing id=\"conversion\"><processingMethod order=\"0\" softwareRef=\"mzlab\">";
        s += &cv("MS:1000544", "Conversion to mzML", "");
        if h.centroided {
            s += &format!("</processingMethod><processingMethod order=\"1\" softwareRef=\"mzlab-centroidi\">{}", cv("MS:1000035", "peak picking", ""));
        }
        s += &step_methods(h);
        s += &format!("</processingMethod></dataProcessing></dataProcessingList>\n<run id=\"{}\" defaultInstrumentConfigurationRef=\"IC1\"{}>\n", esc(&h.run), h.acquired.as_ref().map(|t| format!(" startTimeStamp=\"{}\"", esc(t))).unwrap_or_default());
        for (k, v) in &h.run_params {
            s += &format!("<userParam name=\"{}\" value=\"{}\"/>\n", esc(k), esc(v));
        }
        let mut w = MzmlWriter { out, spectra: 0, chroms: 0, opz, enc: Codifica::new(opz), text: String::new(), idx: indexed.then(indice::Indice::new) };
        w.put(s.as_bytes())?;
        Ok(w)
    }


    /// The number of threads the writer encodes with (`Opzioni::thread`).
    pub fn thread(&self) -> usize {
        self.opz.thread
    }

    pub fn open_spectra(&mut self, count: usize) -> Result<()> {
        self.put(format!("<spectrumList count=\"{count}\" defaultDataProcessingRef=\"conversion\">\n").as_bytes())
    }

    pub fn close_spectra(&mut self) -> Result<()> {
        self.put(b"</spectrumList>\n")
    }

    pub fn open_chromatograms(&mut self, count: usize) -> Result<()> {
        self.put(format!("<chromatogramList count=\"{count}\" defaultDataProcessingRef=\"conversion\">\n").as_bytes())
    }

    pub fn close_chromatograms(&mut self) -> Result<()> {
        self.put(b"</chromatogramList>\n")
    }

    pub fn spectrum(&mut self, sp: &Spectrum) -> Result<()> {
        self.spectrum_with(sp, &SpectrumExtra::default())
    }

    /// As `spectrum`, with the SIM mark, the observed range, the scan window and userParams of `extra`.
    pub fn spectrum_with(&mut self, sp: &Spectrum, extra: &SpectrumExtra) -> Result<()> {
        let mut s = std::mem::take(&mut self.text);
        s.clear();
        let res = self.enc.spettro(self.spectra, sp, extra, &mut s);
        self.spectra += 1;
        self.mark(true, &sp.id);
        let w = self.put(s.as_bytes());
        self.text = s;
        res.and(w)
    }

    /// Every trace of a reader: the mass-spectrometer kinds with their term, the other ones (pressure, detector, device) as `chromatogram type` with the unit in a userParam.
    pub fn chromatogram(&mut self, c: &Chromatogram) -> Result<()> {
        let mut s = format!("<chromatogram index=\"{}\" id=\"{}\" defaultArrayLength=\"{}\">{}", self.chroms, esc(&c.id), c.time.len(), chromatogram_kind(c));
        if matches!(c.kind, ChromKind::Pressure | ChromKind::Analog | ChromKind::Other) && !c.unit.is_empty() {
            s += &format!("<userParam name=\"unità\" value=\"{}\"/>", esc(&c.unit));
        }
        if let (Some(q1), Some(q3)) = (c.q1, c.q3) {
            s += &srm_windows(c, q1, q3);
        }
        s += "<binaryDataArrayList count=\"2\">\n";
        self.enc.array(&mut s, "time", "time", Data::F64(&c.time))?;
        self.enc.array(&mut s, "intensity", "counts", Data::F64(&c.intensity))?;
        s += "</binaryDataArrayList></chromatogram>\n";
        self.chroms += 1;
        self.mark(false, &c.id);
        self.put(s.as_bytes())
    }

    pub fn finish(mut self) -> Result<W> {
        self.put(b"</run>\n</mzML>\n")?;
        self.close_index()?;
        Ok(self.out)
    }
}

/// What the header says about the run.
#[derive(Debug, Clone, Default)]
pub struct Header {
    /// `run id` (the name of the source file without extension).
    pub run: String,
    /// Names of the files the data come from.
    pub sources: Vec<String>,
    /// (accession, name) of the instrument model, e.g. ("MS:1000651", "3200 QTRAP").
    pub instrument: Option<(String, String)>,
    /// Components in order: ("source" | "analyzer" | "detector", accession, name).
    pub components: Vec<(String, String, String)>,
    /// Name and version of the program that wrote the file.
    pub software: (String, String),
    /// `startTimeStamp` of the run, ISO 8601 as the instrument wrote it (with its offset).
    pub acquired: Option<String>,
    /// `MS:1000529` instrument serial number.
    pub serial: Option<String>,
    /// SHA-1 (hex) of each file of `sources`, in the same order (shorter or empty: none).
    pub source_sha1: Vec<String>,
    /// (accession, name) cvParams given to every source file: the format and the nativeID format of the maker.
    pub source_cv: Vec<(String, String)>,
    /// `sampleList`: (id, name).
    pub sample: Option<(String, String)>,
    /// userParams of `<run>`: (name, value).
    pub run_params: Vec<(String, String)>,
    /// More terms of `fileContent`: (accession, name).
    pub content: Vec<(String, String)>,
    /// More software (the acquisition program of the maker): (accession, name, version).
    pub extra_software: Vec<(String, String, String)>,
    /// The spectra were reduced to one point per peak by mzLab (adds the `MS:1000035` processing step).
    pub centroided: bool,
    /// Steps of a workflow that changed the spectra after the conversion, one `processingMethod` each (written in order).
    pub steps: Vec<ProcStep>,
}

/// One workflow step as an mzML `processingMethod`: a PSI-MS term, the program that ran it and its parameters.
#[derive(Debug, Clone, Default)]
pub struct ProcStep {
    /// PSI-MS accession and name of the processing term, e.g. ("MS:1000035", "peak picking").
    pub accession: String,
    pub name: String,
    /// Program (the step id, e.g. `features.detect@1`) and its version.
    pub software: String,
    pub version: String,
    /// userParams: (name, value).
    pub params: Vec<(String, String)>,
}

/// The `software` elements of the steps, `step1`, `step2`, ... (the first id the methods refer to).
#[cfg(not(target_family = "wasm"))]
fn step_software(steps: &[ProcStep]) -> String {
    let one = |(i, p): (usize, &ProcStep)| format!("<software id=\"step{}\" version=\"{}\">{}</software>", i + 1, esc(&p.version), cv("MS:1000799", "custom unreleased software tool", esc(&p.software)));
    steps.iter().enumerate().map(one).collect()
}

/// The `processingMethod` elements of the steps; each closes the previous one, the caller closes the last.
#[cfg(not(target_family = "wasm"))]
fn step_methods(h: &Header) -> String {
    let first = 1 + h.centroided as usize;
    let mut s = String::new();
    for (i, p) in h.steps.iter().enumerate() {
        s += &format!("</processingMethod><processingMethod order=\"{}\" softwareRef=\"step{}\">{}", first + i, i + 1, cv(&p.accession, &p.name, ""));
        s += &p.params.iter().map(|(k, v)| format!("<userParam name=\"{}\" value=\"{}\"/>", esc(k), esc(v))).collect::<String>();
    }
    s
}

// The browser build writes no workflow steps (the indexed mzML is native only).
#[cfg(target_family = "wasm")]
fn step_software(_: &[ProcStep]) -> String {
    String::new()
}
#[cfg(target_family = "wasm")]
fn step_methods(_: &Header) -> String {
    String::new()
}

/// What a spectrum has besides `Spectrum`: kept apart so that the readers that do not need it do not change.
#[derive(Debug, Clone, Default)]
pub struct SpectrumExtra {
    /// `MS:1000582` selected-ion-monitoring spectrum.
    pub sim: bool,
    /// `MS:1000528` / `MS:1000527` lowest and highest observed m/z.
    pub observed: bool,
    /// Scan window (lower, upper) m/z.
    pub window: Option<(f64, f64)>,
    /// `MS:1000045` collision energy (eV) and half-width of the isolation window (m/z) of a spectrum with a plain `Spectrum::precursor`: written in its `<precursor>`.
    pub ce: Option<f64>,
    pub iso_half: Option<f64>,
    /// userParams of the spectrum: (name, value).
    pub params: Vec<(String, String)>,
    /// The precursors, in order (the chain of an MSn scan is one per level, outermost first). Empty: `Spectrum::precursor` is written as target and selected ion.
    #[cfg(feature = "precursori")]
    pub precursors: Vec<Precursore>,
    /// `MS:1000285` total ion current and `MS:1000504` / `MS:1000505` base peak (m/z, intensity) as the maker reports them; `None`: summed from the points.
    #[cfg(feature = "precursori")]
    pub tic: Option<f64>,
    #[cfg(feature = "precursori")]
    pub base_peak: Option<(f64, f64)>,
    /// cvParams of `<scan>` (ion injection time, FAIMS voltage, ...) and of `<spectrum>` (any other term with an accession).
    #[cfg(feature = "precursori")]
    pub scan_params: Vec<CvPar>,
    #[cfg(feature = "precursori")]
    pub cv_params: Vec<CvPar>,
}

pub struct MzmlWriter<W: Write> {
    out: W,
    spectra: usize,
    chroms: usize,
    opz: Opzioni,
    enc: Codifica,
    /// The text of the spectrum, kept from one spectrum to the next.
    text: String,
    /// Positions and checksum, only for `MzmlWriter::indicizzato`.
    #[cfg_attr(target_family = "wasm", allow(dead_code))]
    idx: Option<indice::Indice>,
}

pub(crate) fn io(_: std::io::Error) -> Error {
    Error::new("err.file.mzmlWrite")
}

/// `s` with `&`, `<`, `>` and `"` escaped, in one pass (a text with none of them is copied once).
fn esc(s: &str) -> String {
    if !s.bytes().any(|b| matches!(b, b'&' | b'<' | b'>' | b'"')) {
        return s.to_string();
    }
    let mut out = String::with_capacity(s.len() + 16);
    for c in s.chars() {
        match c {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            c => out.push(c),
        }
    }
    out
}

/// The kind element of a chromatogram: the PSI-MS term of its kind, `chromatogram type` for a trace without a more precise one.
/// The dwell time (userParam, read back by `mzmlhead`), then the precursor with its activation and the product of an SRM transition. The collision energy and the dwell
/// (ms) are the ones the id spells (`ce=`, `dwell=`) and only the readers that know them (`sciex`, `precursori`) write them.
fn srm_windows(c: &Chromatogram, q1: f64, q3: f64) -> String {
    let mut s = String::new();
    #[cfg(any(feature = "sciex", feature = "precursori"))]
    let (ce, dwell) = (crate::mzmlhead::srm_id(&c.id).2, crate::mzmlhead::id_dwell(&c.id));
    #[cfg(not(any(feature = "sciex", feature = "precursori")))]
    let _ = c;
    #[cfg(any(feature = "sciex", feature = "precursori"))]
    if let Some(d) = dwell {
        s += &format!("<userParam name=\"MS_dwell_time\" value=\"{d}\"/>");
    }
    s += &format!("<precursor><isolationWindow>{}</isolationWindow><selectedIonList count=\"1\"><selectedIon>{}</selectedIon></selectedIonList>", mz_cv("MS:1000827", "isolation window target m/z", q1), mz_cv("MS:1000744", "selected ion m/z", q1));
    s += "<activation><cvParam cvRef=\"MS\" accession=\"MS:1000133\" name=\"collision-induced dissociation\" value=\"\"/>";
    #[cfg(any(feature = "sciex", feature = "precursori"))]
    if let Some(ce) = ce {
        s += &format!("<cvParam cvRef=\"MS\" accession=\"MS:1000045\" name=\"collision energy\" value=\"{ce}\" unitCvRef=\"UO\" unitAccession=\"UO:0000266\" unitName=\"electronvolt\"/>");
    }
    s + &format!("</activation></precursor><product><isolationWindow>{}</isolationWindow></product>", mz_cv("MS:1000827", "isolation window target m/z", q3))
}

pub(crate) fn chromatogram_kind(c: &Chromatogram) -> String {
    match c.kind {
        ChromKind::Tic => cv("MS:1000235", "total ion current chromatogram", ""),
        ChromKind::Bpc => cv("MS:1000628", "basepeak chromatogram", ""),
        ChromKind::Srm => cv("MS:1001473", "selected reaction monitoring chromatogram", ""),
        ChromKind::Sic => cv("MS:1000627", "selected ion current chromatogram", ""),
        ChromKind::Pda => cv("MS:1000812", "absorption chromatogram", ""),
        ChromKind::Pressure => cv("MS:1003019", "pressure chromatogram", ""),
        ChromKind::Analog | ChromKind::Other => cv("MS:1000626", "chromatogram type", ""),
    }
}

fn cv(acc: &str, name: &str, value: impl std::fmt::Display) -> String {
    use std::fmt::Write;
    // room for the fixed text and a number: a `format!` starts from the length of its literal and grows while the value is written
    let mut s = String::with_capacity(acc.len() + name.len() + 256);
    let _ = write!(s, "<cvParam cvRef=\"MS\" accession=\"{acc}\" name=\"{name}\" value=\"{value}\"/>");
    s
}

fn window_xml(w: Option<(f64, f64)>) -> String {
    match w {
        Some((lo, hi)) => {
            let mut s = String::with_capacity(768);
            s.push_str("<scanWindowList count=\"1\"><scanWindow>");
            s += &mz_cv("MS:1000501", "scan window lower limit", lo);
            s += &mz_cv("MS:1000500", "scan window upper limit", hi);
            s.push_str("</scanWindow></scanWindowList>");
            s
        }
        None => String::new(),
    }
}

fn mz_cv(acc: &str, name: &str, value: f64) -> String {
    unit_cv(acc, name, value, "MS:1000040", "m/z")
}

#[cfg(feature = "precursori")]
fn unit_cv(acc: &str, name: &str, value: f64, unit_acc: &str, unit_name: &str) -> String {
    use std::fmt::Write;
    let mut v = String::with_capacity(32);
    let _ = write!(v, "{value}");
    unit_cv_str(acc, name, &v, "MS", unit_acc, unit_name)
}

#[cfg(not(feature = "precursori"))]
fn unit_cv(acc: &str, name: &str, value: f64, unit_acc: &str, unit_name: &str) -> String {
    use std::fmt::Write;
    let mut s = String::with_capacity(acc.len() + name.len() + unit_acc.len() + unit_name.len() + 128);
    let _ = write!(s, "<cvParam cvRef=\"MS\" accession=\"{acc}\" name=\"{name}\" value=\"{value}\" unitCvRef=\"MS\" unitAccession=\"{unit_acc}\" unitName=\"{unit_name}\"/>");
    s
}

#[cfg(feature = "precursori")]
/// A cvParam with a unit whose value is already text; `unit_ref` is the CV that defines the unit (`MS` or `UO`).
fn unit_cv_str(acc: &str, name: &str, value: &str, unit_ref: &str, unit_acc: &str, unit_name: &str) -> String {
    ["<cvParam cvRef=\"MS\" accession=\"", acc, "\" name=\"", name, "\" value=\"", value, "\" unitCvRef=\"", unit_ref, "\" unitAccession=\"", unit_acc, "\" unitName=\"", unit_name, "\"/>"].concat()
}

/// The `sourceFileList` of the header; the schema wants at least one child, so no source means no list.
fn source_list(h: &Header) -> String {
    if h.sources.is_empty() {
        return String::new();
    }
    let mut s = format!("<sourceFileList count=\"{}\">", h.sources.len());
    for (i, f) in h.sources.iter().enumerate() {
        let mut inner: String = h.source_cv.iter().map(|(a, n)| cv(a, n, "")).collect();
        if let Some(x) = h.source_sha1.get(i) {
            inner += &cv("MS:1000569", "SHA-1", x);
        }
        s += &format!("<sourceFile id=\"SRC{}\" name=\"{}\" location=\"file:///\">{inner}</sourceFile>", i + 1, esc(f));
    }
    s + "</sourceFileList>"
}

/// The `componentList` of the instrument, left out when empty (same schema rule).
fn component_list(h: &Header) -> String {
    if h.components.is_empty() {
        return String::new();
    }
    let mut s = format!("<componentList count=\"{}\">", h.components.len());
    for (i, (kind, acc, name)) in h.components.iter().enumerate() {
        s += &format!("<{kind} order=\"{}\">{}</{kind}>", i + 1, cv(acc, name, ""));
    }
    s + "</componentList>"
}

mod codifica;
mod matrice;
mod thread;
pub use thread::thread_predefinito;
use codifica::{Codifica, Data};
#[cfg(not(target_family = "wasm"))]
mod pool;
mod testa;
pub(crate) use testa::spectrum_head;
#[cfg(feature = "precursori")]
mod spettro;
#[cfg(feature = "precursori")]
pub use spettro::{attivazione_da_nome, CvPar, Precursore};
#[cfg(feature = "precursori")]
use spettro::{params_xml, precursor_list};

/// The cvParams of `<scan>` the reader gives besides the start time and the filter.
fn scan_xml(extra: &SpectrumExtra) -> String {
    #[cfg(feature = "precursori")]
    return params_xml(&extra.scan_params);
    #[cfg(not(feature = "precursori"))]
    {
        let _ = extra;
        String::new()
    }
}
#[cfg(not(target_family = "wasm"))]
mod indice;
#[cfg(target_family = "wasm")]
#[path = "mzmlwrite/indice_wasm.rs"]
mod indice;
#[cfg(test)]
mod tests;
#[cfg(test)]
mod tests_indice;
#[cfg(all(test, feature = "precursori"))]
mod tests_precursori;
#[cfg(test)]
mod tests_xsd;
#[cfg(all(test, not(target_family = "wasm")))]
mod tests_pool;
