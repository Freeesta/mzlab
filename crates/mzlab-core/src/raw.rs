//! Thermo `.raw` reader (OpenTFRaw, Apache-2.0) over any `Read + Seek`: the browser seeks in a `File` (blocks), the desktop in a file.
//! `read` gives the same `Spectrum` values as the mzML reader; `to_mzml` writes the mzML that the rest of the program already reads.
//! Spectra are the centroid peak lists, as in msconvert's default.

use std::io::{Read, Seek, Write};

use opentfraw::{iter_spectra, Polarity, RawFileReader, SpectrumRecord};

use crate::error::{Error, Result};
use crate::mzml::{Run, Spectrum};

fn raw_error(e: impl std::fmt::Display) -> Error {
    Error::new("err.file.raw").with("detail", e)
}

fn spectrum(r: SpectrumRecord) -> Spectrum {
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
        profile: false,
        filter: r.filter.unwrap_or_default(),
        precursor,
        mz: r.mz,
        intensity: r.intensity,
        wide: None,
    }
}

/// Opens the container (indexes only: no scan is read).
pub fn open<R: Read + Seek>(source: R) -> Result<RawFileReader> {
    RawFileReader::open(source).map_err(raw_error)
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

/// Writes the file as mzML to `out` without holding the `.raw` in memory; `name` is the source file name.
pub fn to_mzml<R: Read + Seek, W: Write>(mut source: R, out: &mut W, name: &str) -> Result<()> {
    let raw = open(&mut source)?;
    opentfraw::write_mzml(&raw, &mut source, out, name, false).map_err(raw_error)
}
