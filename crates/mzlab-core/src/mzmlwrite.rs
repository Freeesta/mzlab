//! mzML writer for a `Run` (what the readers of this crate give back): spectra with nativeID, polarity, level, precursor and zlib arrays,
//! chromatograms (TIC, SRM) with their ids as the converter of the instrument vendor writes them. It is the inverse of `mzml::parse`: the
//! file written reads back to the same `Run` (m/z and times as 64-bit floats, intensities with the width the spectrum holds).

use std::io::{self, Write};

use base64::Engine;
use flate2::write::ZlibEncoder;
use flate2::Compression;

use crate::mzml::{ChromKind, Chromatogram, Run, Spectrum};

const CV: &str = "cvRef=\"MS\"";
const MINUTE: &str = "unitCvRef=\"UO\" unitAccession=\"UO:0000031\" unitName=\"minute\"";

fn esc(s: &str) -> String {
    s.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;").replace('"', "&quot;")
}

fn cv(acc: &str, name: &str, value: &str) -> String {
    format!("<cvParam {CV} accession=\"{acc}\" name=\"{name}\" value=\"{value}\"/>")
}

fn encode(bytes: &[u8]) -> io::Result<String> {
    let mut z = ZlibEncoder::new(Vec::new(), Compression::fast());
    z.write_all(bytes)?;
    Ok(base64::engine::general_purpose::STANDARD.encode(z.finish()?))
}

fn array(acc: &str, name: &str, unit: &str, values: ArrayData) -> io::Result<String> {
    let (width, bytes) = match values {
        ArrayData::F64(v) => (cv("MS:1000523", "64-bit float", ""), v.iter().flat_map(|x| x.to_le_bytes()).collect::<Vec<u8>>()),
        ArrayData::F32(v) => (cv("MS:1000521", "32-bit float", ""), v.iter().flat_map(|x| x.to_le_bytes()).collect()),
    };
    let data = if bytes.is_empty() { String::new() } else { encode(&bytes)? };
    Ok(format!(
        "<binaryDataArray encodedLength=\"{}\">{width}{}<cvParam {CV} accession=\"{acc}\" name=\"{name}\" value=\"\"{unit}/><binary>{data}</binary></binaryDataArray>",
        data.len(),
        cv("MS:1000574", "zlib compression", "")
    ))
}

enum ArrayData<'a> {
    F64(&'a [f64]),
    F32(&'a [f32]),
}

fn spectrum(s: &Spectrum, index: usize) -> io::Result<String> {
    let sum: f64 = (0..s.mz.len()).map(|i| s.intensity_f64(i)).sum();
    let mut x = format!("<spectrum index=\"{index}\" id=\"{}\" defaultArrayLength=\"{}\">", esc(&s.id), s.mz.len());
    x += &cv("MS:1000511", "ms level", &s.level.to_string());
    x += &if s.profile { cv("MS:1000128", "profile spectrum", "") } else { cv("MS:1000127", "centroid spectrum", "") };
    match s.polarity {
        1 => x += &cv("MS:1000130", "positive scan", ""),
        -1 => x += &cv("MS:1000129", "negative scan", ""),
        _ => {}
    }
    x += &cv("MS:1000285", "total ion current", &format!("{sum}"));
    x += "<scanList count=\"1\"><scan>";
    x += &format!("<cvParam {CV} accession=\"MS:1000016\" name=\"scan start time\" value=\"{}\" {MINUTE}/>", s.rt);
    if !s.filter.is_empty() {
        x += &cv("MS:1000512", "filter string", &esc(&s.filter));
    }
    x += "</scan></scanList>";
    if let Some(p) = s.precursor {
        x += &format!("<precursorList count=\"1\"><precursor><selectedIonList count=\"1\"><selectedIon>{}</selectedIon></selectedIonList></precursor></precursorList>", cv("MS:1000744", "selected ion m/z", &p.to_string()));
    }
    let inten = match &s.wide {
        Some(w) => ArrayData::F64(w),
        None => ArrayData::F32(&s.intensity),
    };
    x += &format!("<binaryDataArrayList count=\"2\">{}{}</binaryDataArrayList></spectrum>", array("MS:1000514", "m/z array", "", ArrayData::F64(&s.mz))?, array("MS:1000515", "intensity array", "", inten)?);
    Ok(x)
}

fn chromatogram(c: &Chromatogram, index: usize) -> io::Result<String> {
    let mut x = format!("<chromatogram index=\"{index}\" id=\"{}\" defaultArrayLength=\"{}\">", esc(&c.id), c.time.len());
    match c.kind {
        ChromKind::Tic => x += &cv("MS:1000235", "total ion current chromatogram", ""),
        ChromKind::Bpc => x += &cv("MS:1000628", "basepeak chromatogram", ""),
        ChromKind::Srm => x += &cv("MS:1001473", "selected reaction monitoring chromatogram", ""),
        _ => {}
    }
    let unit = if c.unit.is_empty() { String::new() } else { format!(" unitName=\"{}\"", esc(&c.unit)) };
    x += &format!("<binaryDataArrayList count=\"2\">{}{}</binaryDataArrayList></chromatogram>", array("MS:1000595", "time array", &format!(" {MINUTE}"), ArrayData::F64(&c.time))?, array("MS:1000515", "intensity array", &unit, ArrayData::F64(&c.intensity))?);
    Ok(x)
}

/// Writes `run` as mzML. `source` is the name of the file it comes from (`sourceFile`), `instrument` the model cvParam name (e.g. "3200 QTRAP").
pub fn write<W: Write>(run: &Run, out: &mut W, source: &str, instrument: &str) -> io::Result<()> {
    write!(
        out,
        "<?xml version=\"1.0\" encoding=\"utf-8\"?>\n<mzML xmlns=\"http://psi.hupo.org/ms/mzml\" version=\"1.1.0\">\n\
         <fileDescription><fileContent>{}</fileContent><sourceFileList count=\"1\"><sourceFile id=\"SF1\" name=\"{}\" location=\"file:///\"/></sourceFileList></fileDescription>\n\
         <instrumentConfigurationList count=\"1\"><instrumentConfiguration id=\"IC1\">{}</instrumentConfiguration></instrumentConfigurationList>\n<run id=\"{}\">\n",
        if run.spectra.iter().any(|s| s.level > 1) { cv("MS:1000580", "MSn spectrum", "") } else { cv("MS:1000579", "MS1 spectrum", "") },
        esc(source),
        cv("MS:1000651", &esc(instrument), ""),
        esc(source.rsplit_once('.').map_or(source, |p| p.0))
    )?;
    if !run.spectra.is_empty() {
        writeln!(out, "<spectrumList count=\"{}\">", run.spectra.len())?;
        for (i, s) in run.spectra.iter().enumerate() {
            writeln!(out, "{}", spectrum(s, i)?)?;
        }
        writeln!(out, "</spectrumList>")?;
    }
    if !run.chromatograms.is_empty() {
        writeln!(out, "<chromatogramList count=\"{}\">", run.chromatograms.len())?;
        for (i, c) in run.chromatograms.iter().enumerate() {
            writeln!(out, "{}", chromatogram(c, i)?)?;
        }
        writeln!(out, "</chromatogramList>")?;
    }
    writeln!(out, "</run>\n</mzML>")
}
