//! mzML writer over any `Write`, spectrum by spectrum, for readers of vendor files that have no mzML writer of their own (`wiff::to_mzml`).
//! What it writes is what `mzml::read` and `mzlab/reader/mzml.py` read: spectra (level, polarity, profile or centroid, RT in minutes, precursor,
//! m/z as 64-bit floats, intensities as 64-bit when the spectrum has `wide`, otherwise 32-bit, both zlib), and TIC, BPC and SRM chromatograms
//! (the SRM masses are in the `id`, as msconvert writes them, and as isolation windows). No index, no numpress.

use std::io::Write;

use base64::Engine;
use flate2::write::ZlibEncoder;
use flate2::Compression;

use crate::error::{Error, Result};
use crate::mzml::{ChromKind, Chromatogram, Spectrum};

impl<W: Write> MzmlWriter<W> {
    /// Writes the header and opens `<run>`.
    pub fn new(mut out: W, h: &Header) -> Result<Self> {
        let mut s = String::from("<?xml version=\"1.0\" encoding=\"utf-8\"?>\n<mzML xmlns=\"http://psi.hupo.org/ms/mzml\" version=\"1.1.0\" id=\"mzlab\">\n");
        s += "<cvList count=\"2\"><cv id=\"MS\" fullName=\"Proteomics Standards Initiative Mass Spectrometry Ontology\" version=\"4.1.163\" URI=\"https://raw.githubusercontent.com/HUPO-PSI/psi-ms-CV/master/psi-ms.obo\"/><cv id=\"UO\" fullName=\"Unit Ontology\" version=\"09:04:2014\" URI=\"https://raw.githubusercontent.com/bio-ontology-research-group/unit-ontology/master/unit.obo\"/></cvList>\n";
        s += &format!("<fileDescription><fileContent>{}</fileContent><sourceFileList count=\"{}\">", cv("MS:1000579", "MS1 spectrum", ""), h.sources.len());
        for (i, f) in h.sources.iter().enumerate() {
            s += &format!("<sourceFile id=\"SRC{}\" name=\"{}\" location=\"file:///\"/>", i + 1, esc(f));
        }
        s += "</sourceFileList></fileDescription>\n";
        s += &format!("<softwareList count=\"1\"><software id=\"mzlab\" version=\"{}\">{}</software></softwareList>\n", esc(&h.software.1), cv("MS:1000799", "custom unreleased software tool", esc(&h.software.0)));
        s += "<instrumentConfigurationList count=\"1\"><instrumentConfiguration id=\"IC1\">";
        if let Some((acc, name)) = &h.instrument {
            s += &cv(acc, name, "");
        }
        s += &format!("<componentList count=\"{}\">", h.components.len());
        for (i, (kind, acc, name)) in h.components.iter().enumerate() {
            s += &format!("<{kind} order=\"{}\">{}</{kind}>", i + 1, cv(acc, name, ""));
        }
        s += "</componentList><softwareRef ref=\"mzlab\"/></instrumentConfiguration></instrumentConfigurationList>\n";
        s += "<dataProcessingList count=\"1\"><dataProcessing id=\"conversion\"><processingMethod order=\"0\" softwareRef=\"mzlab\">";
        s += &cv("MS:1000544", "Conversion to mzML", "");
        s += &format!("</processingMethod></dataProcessing></dataProcessingList>\n<run id=\"{}\" defaultInstrumentConfigurationRef=\"IC1\">\n", esc(&h.run));
        out.write_all(s.as_bytes()).map_err(io)?;
        Ok(MzmlWriter { out, spectra: 0, chroms: 0 })
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
        if !sp.filter.is_empty() {
            s += &cv("MS:1000512", "filter string", esc(&sp.filter));
        }
        s += &format!("<scanList count=\"1\">{}<scan><cvParam cvRef=\"MS\" accession=\"MS:1000016\" name=\"scan start time\" value=\"{}\" unitCvRef=\"UO\" unitAccession=\"UO:0000031\" unitName=\"minute\"/></scan></scanList>", cv("MS:1000795", "no combination", ""), sp.rt);
        if let Some(p) = sp.precursor {
            s += &format!("<precursorList count=\"1\"><precursor><isolationWindow>{}</isolationWindow><selectedIonList count=\"1\"><selectedIon>{}</selectedIon></selectedIonList></precursor></precursorList>", mz_cv("MS:1000827", "isolation window target m/z", p), mz_cv("MS:1000744", "selected ion m/z", p));
        }
        s += "<binaryDataArrayList count=\"2\">\n";
        s += &array("mz", "mz", true, &f64_bytes(&sp.mz))?;
        s += &match &sp.wide {
            Some(w) => array("intensity", "counts", true, &f64_bytes(w))?,
            None => array("intensity", "counts", false, &sp.intensity.iter().flat_map(|x| x.to_le_bytes()).collect::<Vec<u8>>())?,
        };
        s += "</binaryDataArrayList></spectrum>\n";
        self.spectra += 1;
        self.out.write_all(s.as_bytes()).map_err(io)
    }

    /// TIC, BPC and SRM chromatograms; other kinds are not written.
    pub fn chromatogram(&mut self, c: &Chromatogram) -> Result<()> {
        let kind = match c.kind {
            ChromKind::Tic => cv("MS:1000235", "total ion current chromatogram", ""),
            ChromKind::Bpc => cv("MS:1000628", "basepeak chromatogram", ""),
            ChromKind::Srm => cv("MS:1001473", "selected reaction monitoring chromatogram", ""),
            _ => return Ok(()),
        };
        let mut s = format!("<chromatogram index=\"{}\" id=\"{}\" defaultArrayLength=\"{}\">{kind}", self.chroms, esc(&c.id), c.time.len());
        if let (Some(q1), Some(q3)) = (c.q1, c.q3) {
            s += &format!("<precursor><isolationWindow>{}</isolationWindow></precursor><product><isolationWindow>{}</isolationWindow></product>", mz_cv("MS:1000827", "isolation window target m/z", q1), mz_cv("MS:1000827", "isolation window target m/z", q3));
        }
        s += "<binaryDataArrayList count=\"2\">\n";
        s += &array("time", "time", true, &f64_bytes(&c.time))?;
        s += &array("intensity", "counts", true, &f64_bytes(&c.intensity))?;
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
}

pub struct MzmlWriter<W: Write> {
    out: W,
    spectra: usize,
    chroms: usize,
}

fn io(_: std::io::Error) -> Error {
    Error::new("err.file.mzmlWrite")
}

fn esc(s: &str) -> String {
    s.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;").replace('"', "&quot;")
}

fn b64(bytes: &[u8]) -> Result<(usize, String)> {
    let mut z = ZlibEncoder::new(Vec::with_capacity(bytes.len() / 2 + 16), Compression::default());
    z.write_all(bytes).map_err(io)?;
    let s = base64::engine::general_purpose::STANDARD.encode(z.finish().map_err(io)?);
    Ok((s.len(), s))
}

fn f64_bytes(v: &[f64]) -> Vec<u8> {
    v.iter().flat_map(|x| x.to_le_bytes()).collect()
}

fn array(kind: &str, unit: &str, wide: bool, bytes: &[u8]) -> Result<String> {
    let (len, data) = b64(bytes)?;
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
    Ok(format!(
        "<binaryDataArray encodedLength=\"{len}\"><cvParam cvRef=\"MS\" accession=\"{acc}\" name=\"{name}\" value=\"\"/><cvParam cvRef=\"MS\" accession=\"MS:1000574\" name=\"zlib compression\" value=\"\"/><cvParam cvRef=\"MS\" accession=\"{cv}\" name=\"{label}\" value=\"\" unitCvRef=\"{unit_cv}\" unitAccession=\"{unit_acc}\" unitName=\"{unit_name}\"/><binary>{data}</binary></binaryDataArray>\n"
    ))
}

fn cv(acc: &str, name: &str, value: impl std::fmt::Display) -> String {
    format!("<cvParam cvRef=\"MS\" accession=\"{acc}\" name=\"{name}\" value=\"{value}\"/>")
}

fn mz_cv(acc: &str, name: &str, value: f64) -> String {
    format!("<cvParam cvRef=\"MS\" accession=\"{acc}\" name=\"{name}\" value=\"{value}\" unitCvRef=\"MS\" unitAccession=\"MS:1000040\" unitName=\"m/z\"/>")
}

#[cfg(test)]
mod tests;
