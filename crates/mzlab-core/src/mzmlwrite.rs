//! mzML writer over any `Write`, spectrum by spectrum, for readers of vendor files that have no mzML writer of their own (`wiff::to_mzml`).
//! What it writes is what `mzml::read` and `mzlab/reader/mzml.py` read: spectra (level, polarity, profile or centroid, RT in minutes, precursor,
//! m/z as 64-bit floats, intensities as 64-bit when the spectrum has `wide`, otherwise 32-bit, both zlib), and TIC, BPC and SRM chromatograms
//! (the SRM masses are in the `id`, as msconvert writes them, and as isolation windows). No index, no numpress.

use std::io::Write;

use base64::Engine;
use flate2::{Compress, Compression, FlushCompress};

use crate::error::{Error, Result};
use crate::mzml::{ChromKind, Chromatogram, Spectrum};

impl<W: Write> MzmlWriter<W> {
    /// Writes the header and opens `<run>`.
    pub fn new(mut out: W, h: &Header) -> Result<Self> {
        let mut s = String::from("<?xml version=\"1.0\" encoding=\"utf-8\"?>\n<mzML xmlns=\"http://psi.hupo.org/ms/mzml\" version=\"1.1.0\" id=\"mzlab\">\n");
        s += "<cvList count=\"2\"><cv id=\"MS\" fullName=\"Proteomics Standards Initiative Mass Spectrometry Ontology\" version=\"4.1.163\" URI=\"https://raw.githubusercontent.com/HUPO-PSI/psi-ms-CV/master/psi-ms.obo\"/><cv id=\"UO\" fullName=\"Unit Ontology\" version=\"09:04:2014\" URI=\"https://raw.githubusercontent.com/bio-ontology-research-group/unit-ontology/master/unit.obo\"/></cvList>\n";
        let content = cv("MS:1000579", "MS1 spectrum", "") + &h.content.iter().map(|(a, n)| cv(a, n, "")).collect::<String>();
        s += &format!("<fileDescription><fileContent>{}</fileContent><sourceFileList count=\"{}\">", content, h.sources.len());
        for (i, f) in h.sources.iter().enumerate() {
            let mut inner: String = h.source_cv.iter().map(|(a, n)| cv(a, n, "")).collect();
            if let Some(x) = h.source_sha1.get(i) {
                inner += &cv("MS:1000569", "SHA-1", x);
            }
            s += &format!("<sourceFile id=\"SRC{}\" name=\"{}\" location=\"file:///\">{inner}</sourceFile>", i + 1, esc(f));
        }
        s += "</sourceFileList></fileDescription>\n";
        if let Some((id, name)) = &h.sample {
            s += &format!("<sampleList count=\"1\"><sample id=\"{}\" name=\"{}\"/></sampleList>\n", esc(id), esc(name));
        }
        s += &format!("<softwareList count=\"{}\"><software id=\"mzlab\" version=\"{}\">{}</software>{}</softwareList>\n", 1 + h.centroided as usize + h.extra_software.len(), esc(&h.software.1), cv("MS:1000799", "custom unreleased software tool", esc(&h.software.0)),
            h.extra_software.iter().enumerate().map(|(i, (a, n, v))| format!("<software id=\"acq{}\" version=\"{}\">{}</software>", i + 1, esc(v), cv(a, n, ""))).collect::<String>() + &if h.centroided { format!("<software id=\"mzlab-centroidi\" version=\"{}\">{}</software>", esc(&h.software.1), cv("MS:1000799", "custom unreleased software tool", "mzLab centroidi")) } else { String::new() });
        s += "<instrumentConfigurationList count=\"1\"><instrumentConfiguration id=\"IC1\">";
        if let Some((acc, name)) = &h.instrument {
            s += &cv(acc, name, "");
        }
        if let Some(sn) = &h.serial {
            s += &cv("MS:1000529", "instrument serial number", esc(sn));
        }
        s += &format!("<componentList count=\"{}\">", h.components.len());
        for (i, (kind, acc, name)) in h.components.iter().enumerate() {
            s += &format!("<{kind} order=\"{}\">{}</{kind}>", i + 1, cv(acc, name, ""));
        }
        s += "</componentList><softwareRef ref=\"mzlab\"/></instrumentConfiguration></instrumentConfigurationList>\n";
        s += "<dataProcessingList count=\"1\"><dataProcessing id=\"conversion\"><processingMethod order=\"0\" softwareRef=\"mzlab\">";
        s += &cv("MS:1000544", "Conversion to mzML", "");
        if h.centroided {
            s += &format!("</processingMethod><processingMethod order=\"1\" softwareRef=\"mzlab-centroidi\">{}", cv("MS:1000035", "peak picking", ""));
        }
        s += &format!("</processingMethod></dataProcessing></dataProcessingList>\n<run id=\"{}\" defaultInstrumentConfigurationRef=\"IC1\"{}>\n", esc(&h.run), h.acquired.as_ref().map(|t| format!(" startTimeStamp=\"{}\"", esc(t))).unwrap_or_default());
        for (k, v) in &h.run_params {
            s += &format!("<userParam name=\"{}\" value=\"{}\"/>\n", esc(k), esc(v));
        }
        out.write_all(s.as_bytes()).map_err(io)?;
        Ok(MzmlWriter { out, spectra: 0, chroms: 0, z: Compress::new(Compression::default(), true), raw: Vec::new(), packed: Vec::new() })
    }

    pub fn open_spectra(&mut self, count: usize) -> Result<()> {
        self.out.write_all(format!("<spectrumList count=\"{count}\" defaultDataProcessingRef=\"conversion\">\n").as_bytes()).map_err(io)
    }

    pub fn close_spectra(&mut self) -> Result<()> {
        self.out.write_all(b"</spectrumList>\n").map_err(io)
    }

    pub fn open_chromatograms(&mut self, count: usize) -> Result<()> {
        self.out.write_all(format!("<chromatogramList count=\"{count}\" defaultDataProcessingRef=\"conversion\">\n").as_bytes()).map_err(io)
    }

    pub fn close_chromatograms(&mut self) -> Result<()> {
        self.out.write_all(b"</chromatogramList>\n").map_err(io)
    }

    pub fn spectrum(&mut self, sp: &Spectrum) -> Result<()> {
        self.spectrum_with(sp, &SpectrumExtra::default())
    }

    /// As `spectrum`, with the SIM mark, the observed range, the scan window and userParams of `extra`.
    pub fn spectrum_with(&mut self, sp: &Spectrum, extra: &SpectrumExtra) -> Result<()> {
        let tic: f64 = (0..sp.mz.len()).map(|i| sp.intensity_f64(i)).sum();
        let base = (0..sp.mz.len()).max_by(|&a, &b| sp.intensity_f64(a).total_cmp(&sp.intensity_f64(b)));
        let mut s = format!("<spectrum index=\"{}\" id=\"{}\" defaultArrayLength=\"{}\">", self.spectra, esc(&sp.id), sp.mz.len());
        s += &cv("MS:1000511", "ms level", sp.level);
        s += &if sp.level <= 1 { cv("MS:1000579", "MS1 spectrum", "") } else { cv("MS:1000580", "MSn spectrum", "") };
        match sp.polarity {
            1 => s += &cv("MS:1000130", "positive scan", ""),
            -1 => s += &cv("MS:1000129", "negative scan", ""),
            _ => {}
        }
        s += &if sp.profile { cv("MS:1000128", "profile spectrum", "") } else { cv("MS:1000127", "centroid spectrum", "") };
        if let Some(b) = base {
            s += &mz_cv("MS:1000504", "base peak m/z", sp.mz[b]);
            s += &cv("MS:1000505", "base peak intensity", sp.intensity_f64(b));
        }
        s += &cv("MS:1000285", "total ion current", tic);
        if extra.sim {
            s += &cv("MS:1000582", "SIM spectrum", "");
        }
        if let (true, Some(&lo), Some(&hi)) = (extra.observed, sp.mz.first(), sp.mz.last()) {
            s += &mz_cv("MS:1000528", "lowest observed m/z", lo);
            s += &mz_cv("MS:1000527", "highest observed m/z", hi);
        }
        for (k, v) in &extra.params {
            s += &format!("<userParam name=\"{}\" value=\"{}\"/>", esc(k), esc(v));
        }
        if !sp.filter.is_empty() {
            s += &cv("MS:1000512", "filter string", esc(&sp.filter));
        }
        s += &format!("<scanList count=\"1\">{}<scan><cvParam cvRef=\"MS\" accession=\"MS:1000016\" name=\"scan start time\" value=\"{}\" unitCvRef=\"UO\" unitAccession=\"UO:0000031\" unitName=\"minute\"/>{}</scan></scanList>", cv("MS:1000795", "no combination", ""), sp.rt, window_xml(extra.window));
        if let Some(p) = sp.precursor {
            s += &format!("<precursorList count=\"1\"><precursor><isolationWindow>{}</isolationWindow><selectedIonList count=\"1\"><selectedIon>{}</selectedIon></selectedIonList></precursor></precursorList>", mz_cv("MS:1000827", "isolation window target m/z", p), mz_cv("MS:1000744", "selected ion m/z", p));
        }
        s += "<binaryDataArrayList count=\"2\">\n";
        self.array(&mut s, "mz", "mz", Data::F64(&sp.mz))?;
        match &sp.wide {
            Some(w) => self.array(&mut s, "intensity", "counts", Data::F64(w))?,
            None => self.array(&mut s, "intensity", "counts", Data::F32(&sp.intensity))?,
        }
        s += "</binaryDataArrayList></spectrum>\n";
        self.spectra += 1;
        self.out.write_all(s.as_bytes()).map_err(io)
    }

    /// TIC, BPC, SIC and SRM chromatograms; other kinds are not written.
    pub fn chromatogram(&mut self, c: &Chromatogram) -> Result<()> {
        let kind = match c.kind {
            ChromKind::Tic => cv("MS:1000235", "total ion current chromatogram", ""),
            ChromKind::Bpc => cv("MS:1000628", "basepeak chromatogram", ""),
            ChromKind::Srm => cv("MS:1001473", "selected reaction monitoring chromatogram", ""),
            ChromKind::Sic => cv("MS:1000627", "selected ion current chromatogram", ""),
            _ => return Ok(()),
        };
        let mut s = format!("<chromatogram index=\"{}\" id=\"{}\" defaultArrayLength=\"{}\">{kind}", self.chroms, esc(&c.id), c.time.len());
        if let (Some(q1), Some(q3)) = (c.q1, c.q3) {
            s += &format!("<precursor><isolationWindow>{}</isolationWindow></precursor><product><isolationWindow>{}</isolationWindow></product>", mz_cv("MS:1000827", "isolation window target m/z", q1), mz_cv("MS:1000827", "isolation window target m/z", q3));
        }
        s += "<binaryDataArrayList count=\"2\">\n";
        self.array(&mut s, "time", "time", Data::F64(&c.time))?;
        self.array(&mut s, "intensity", "counts", Data::F64(&c.intensity))?;
        s += "</binaryDataArrayList></chromatogram>\n";
        self.chroms += 1;
        self.out.write_all(s.as_bytes()).map_err(io)
    }

    pub fn finish(mut self) -> Result<W> {
        self.out.write_all(b"</run>\n</mzML>\n").map_err(io)?;
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
    /// userParams of the spectrum: (name, value).
    pub params: Vec<(String, String)>,
}

pub struct MzmlWriter<W: Write> {
    out: W,
    spectra: usize,
    chroms: usize,
    /// The zlib state and the two scratch buffers are kept from one array to the next (same bytes as a new encoder each time, without the allocations).
    z: Compress,
    raw: Vec<u8>,
    packed: Vec<u8>,
}

/// The values of one binary array: 64-bit or 32-bit floats.
enum Data<'a> {
    F64(&'a [f64]),
    F32(&'a [f32]),
}

fn io(_: std::io::Error) -> Error {
    Error::new("err.file.mzmlWrite")
}

fn esc(s: &str) -> String {
    s.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;").replace('"', "&quot;")
}

impl<W: Write> MzmlWriter<W> {
    /// Appends `<binaryDataArray>` to `s`: the values as little-endian floats, zlib (default level) and base64.
    fn array(&mut self, s: &mut String, kind: &str, unit: &str, data: Data) -> Result<()> {
        self.raw.clear();
        let wide = match data {
            Data::F64(v) => {
                self.raw.reserve(v.len() * 8);
                v.iter().for_each(|x| self.raw.extend_from_slice(&x.to_le_bytes()));
                true
            }
            Data::F32(v) => {
                self.raw.reserve(v.len() * 4);
                v.iter().for_each(|x| self.raw.extend_from_slice(&x.to_le_bytes()));
                false
            }
        };
        self.deflate()?;
        let len = (self.packed.len() + 2) / 3 * 4;
        let (acc, name) = if wide { ("MS:1000523", "64-bit float") } else { ("MS:1000521", "32-bit float") };
        let (unit_acc, unit_name) = match unit {
            "time" => ("UO:0000031", "minute"),
            "mz" => ("MS:1000040", "m/z"),
            _ => ("MS:1000131", "number of detector counts"),
        };
        let (cv, label) = match kind {
            "time" => ("MS:1000595", "time array"),
            "mz" => ("MS:1000514", "m/z array"),
            _ => ("MS:1000515", "intensity array"),
        };
        let unit_cv = if unit == "time" { "UO" } else { "MS" };
        s.push_str(&format!(
            "<binaryDataArray encodedLength=\"{len}\"><cvParam cvRef=\"MS\" accession=\"{acc}\" name=\"{name}\" value=\"\"/><cvParam cvRef=\"MS\" accession=\"MS:1000574\" name=\"zlib compression\" value=\"\"/><cvParam cvRef=\"MS\" accession=\"{cv}\" name=\"{label}\" value=\"\" unitCvRef=\"{unit_cv}\" unitAccession=\"{unit_acc}\" unitName=\"{unit_name}\"/><binary>"
        ));
        base64::engine::general_purpose::STANDARD.encode_string(&self.packed, s);
        s.push_str("</binary></binaryDataArray>\n");
        Ok(())
    }

    /// `raw` compressed into `packed` (a zlib stream at the default level, as `ZlibEncoder` writes it).
    fn deflate(&mut self) -> Result<()> {
        self.z.reset();
        self.packed.clear();
        self.packed.reserve(self.raw.len() / 2 + 64);
        let bad = |_| io(std::io::ErrorKind::Other.into());
        while (self.z.total_in() as usize) < self.raw.len() {
            if self.packed.len() == self.packed.capacity() {
                self.packed.reserve(self.raw.len() / 4 + 4096);
            }
            let done = self.z.total_in() as usize;
            self.z.compress_vec(&self.raw[done..], &mut self.packed, FlushCompress::None).map_err(bad)?;
        }
        loop {
            if self.packed.len() == self.packed.capacity() {
                self.packed.reserve(4096);
            }
            if self.z.compress_vec(&[], &mut self.packed, FlushCompress::Finish).map_err(bad)? == flate2::Status::StreamEnd {
                break;
            }
        }
        Ok(())
    }
}

fn cv(acc: &str, name: &str, value: impl std::fmt::Display) -> String {
    format!("<cvParam cvRef=\"MS\" accession=\"{acc}\" name=\"{name}\" value=\"{value}\"/>")
}

fn window_xml(w: Option<(f64, f64)>) -> String {
    match w {
        Some((lo, hi)) => format!("<scanWindowList count=\"1\"><scanWindow>{}{}</scanWindow></scanWindowList>", mz_cv("MS:1000501", "scan window lower limit", lo), mz_cv("MS:1000500", "scan window upper limit", hi)),
        None => String::new(),
    }
}

fn mz_cv(acc: &str, name: &str, value: f64) -> String {
    format!("<cvParam cvRef=\"MS\" accession=\"{acc}\" name=\"{name}\" value=\"{value}\" unitCvRef=\"MS\" unitAccession=\"MS:1000040\" unitName=\"m/z\"/>")
}

#[cfg(test)]
mod tests;
