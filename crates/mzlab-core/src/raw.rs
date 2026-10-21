//! Thermo `.raw` reader (OpenTFRaw, Apache-2.0) over any `Read + Seek`: the browser seeks in a `File` (blocks), the desktop in a file.
//! `read` gives the same `Spectrum` values as the mzML reader; `to_mzml` writes the mzML that the rest of the program already reads.
//! Spectra are what the file holds, as in msconvert's default: the centroid peak list of a scan, or its profile when it has no other (`Spectrum::profile`);
//! nothing is peak-picked here.

use std::io::{Read, Seek, SeekFrom};

use opentfraw::{iter_spectra, Polarity, RawFileReader, ScanMode, SpectrumRecord};

use crate::error::{Error, Result};
use crate::mzml::{Run, Spectrum};

/// Maps an OpenTFRaw error to the key the page shows: a format version the reader does not know has its own message (`err.file.rawVersion`); anything else
/// keeps the generic `err.file.raw` with the reader's text.
pub fn thermo_error(e: &opentfraw::Error) -> Error {
    match e {
        opentfraw::Error::UnsupportedVersion(v) => {
            Error::new("err.file.rawVersion").with("version", v)
        }
        _ => Error::new("err.file.raw").with("detail", e),
    }
}

pub(crate) fn spectrum(r: SpectrumRecord) -> Spectrum {
    let precursor = r
        .precursor
        .as_ref()
        .and_then(|p| p.selected_mz.or(p.target_mz));
    Spectrum {
        index: r.index,
        id: format!("controllerType=0 controllerNumber=1 scan={}", r.scan_number),
        level: r.ms_level.min(255) as u8,
        rt: r.retention_time_min,
        polarity: match r.polarity {
            Some(Polarity::Positive) => 1,
            Some(Polarity::Negative) => -1,
            None => 0,
        },
        profile: r.scan_mode == Some(ScanMode::Profile),
        filter: r.filter.unwrap_or_default(),
        precursor,
        mz: r.mz,
        intensity: r.intensity,
        wide: None,
    }
}

/// Opens the container (indexes only: no scan is read).
pub fn open<R: Read + Seek>(source: R) -> Result<RawFileReader> {
    RawFileReader::open(source).map_err(|e| thermo_error(&e))
}

/// The text of the instrument method stored in the `.raw` (the Chromeleon script with the gradient), or `None` when the file has none.
/// Only the first ~500 kB are read; the text goes to `mzlab.analog.parse_chromeleon` in the page.
pub fn instrument_method_text<R: Read + Seek>(mut source: R) -> Result<Option<String>> {
    let raw = open(&mut source)?;
    Ok(raw.instrument_method_text(&mut source))
}

/// The `metodo/1` document of a `.raw` (`metodo_raw.rs`); `name` is the file name.
pub fn metodo<R: Read + Seek>(mut source: R, name: &str) -> Result<String> {
    let raw = open(&mut source)?;
    Ok(metodo_of(&raw, &mut source, name))
}

pub(crate) fn metodo_of<R: Read + Seek>(raw: &RawFileReader, source: &mut R, name: &str) -> String {
    metodo_sha1(raw, source, name, None)
}

/// As `metodo_of`, with the SHA-1 of the file when the caller has it.
pub(crate) fn metodo_sha1<R: Read + Seek>(
    raw: &RawFileReader,
    source: &mut R,
    name: &str,
    sha1: Option<String>,
) -> String {
    let bytes = source.seek(SeekFrom::End(0)).unwrap_or(0);
    let src = crate::metodo::Source {
        format: "thermo-raw",
        files: vec![crate::metodo::SrcFile {
            name: name.to_string(),
            sha1,
            bytes,
        }],
        sample: None,
    };
    crate::metodo_raw::from_raw(raw, source, &src)
}

/// Streams the spectra one at a time (centroids); `on` gets each before the next is read.
pub fn parse<R: Read + Seek>(mut source: R, mut on: impl FnMut(Spectrum)) -> Result<()> {
    let raw = open(&mut source)?;
    for rec in iter_spectra(&raw, &mut source, false) {
        on(spectrum(rec));
    }
    Ok(())
}

/// Reads the whole file into memory (as `mzml::read` does).
pub fn read<R: Read + Seek>(source: R) -> Result<Run> {
    let mut run = Run::default();
    parse(source, |s| run.spectra.push(s))?;
    Ok(run)
}
