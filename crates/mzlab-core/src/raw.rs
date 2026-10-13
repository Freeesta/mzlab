//! Thermo `.raw` reader (OpenTFRaw, Apache-2.0) over any `Read + Seek`: the browser seeks in a `File` (blocks), the desktop in a file.
//! `read` gives the same `Spectrum` values as the mzML reader; `to_mzml` writes the mzML that the rest of the program already reads.
//! Spectra are what the file holds, as in msconvert's default: the centroid peak list of a scan, or its profile when it has no other (`Spectrum::profile`);
//! nothing is peak-picked here.

use std::io::{Read, Seek, SeekFrom, Write};

use opentfraw::{iter_spectra, Polarity, RawFileReader, ScanMode, SpectrumRecord};

use crate::error::{Error, Result};
use crate::mzml::{Run, Spectrum};
use crate::rawfilter::MzmlFilter;

fn raw_error(e: impl std::fmt::Display) -> Error {
    Error::new("err.file.raw").with("detail", e)
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
    RawFileReader::open(source).map_err(raw_error)
}

/// The text of the instrument method stored in the `.raw` (the Chromeleon script with the gradient), or `None` when the file has none.
/// Only the first ~500 kB are read; the text goes to `mzlab.analog.parse_chromeleon` in the page.
pub fn instrument_method_text<R: Read + Seek>(mut source: R) -> Result<Option<String>> {
    let raw = open(&mut source)?;
    Ok(raw.instrument_method_text(&mut source))
}

/// The `metodo/1` document of a `.raw` (`metodo_raw.rs`); `name` is the file name, `anonymous` leaves out the people, the computer, the serial number and the paths.
pub fn metodo<R: Read + Seek>(mut source: R, name: &str, anonymous: bool) -> Result<String> {
    let raw = open(&mut source)?;
    Ok(metodo_of(&raw, &mut source, name, anonymous))
}

pub(crate) fn metodo_of<R: Read + Seek>(
    raw: &RawFileReader,
    source: &mut R,
    name: &str,
    anonymous: bool,
) -> String {
    let bytes = source.seek(SeekFrom::End(0)).unwrap_or(0);
    let src = crate::metodo::Source {
        format: "thermo-raw",
        files: vec![crate::metodo::SrcFile {
            name: name.to_string(),
            sha1: None,
            bytes,
        }],
        sample: None,
        anonymous,
    };
    crate::metodo_raw::from_raw(raw, source, &src)
}

/// The `userParam` line that carries the `metodo/1` document in the `<run>` of the mzML (`metodo::from_userparam` reads it back).
fn method_userparam<R: Read + Seek>(
    raw: &RawFileReader,
    source: &mut R,
    name: &str,
    anonymous: bool,
) -> String {
    crate::metodo::userparam_xml(&metodo_of(raw, source, name, anonymous))
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
    let doc = method_userparam(&raw, &mut source, name, false);
    let mut filter = MzmlFilter::new(out, false).with_run_extra(doc);
    opentfraw::write_mzml(&raw, &mut source, &mut filter, name, false).map_err(raw_error)?;
    filter.flush().map_err(raw_error)
}

/// A source that counts the scans already read: the packets of the scans lie in the file in scan order, so the scans whose packet
/// starts before the read position are the ones converted (or being converted). `on_scan(done, total)` is called only when `done` grows.
struct Counted<'a, R> {
    inner: R,
    starts: Vec<u64>,
    pos: u64,
    done: u32,
    on_scan: &'a mut dyn FnMut(u32, u32),
}

impl<R: Read> Read for Counted<'_, R> {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        let n = self.inner.read(buf)?;
        self.pos += n as u64;
        let reached = self.starts.partition_point(|&o| o < self.pos);
        let done = reached.saturating_sub(1) as u32; // the packet being read is not converted yet
        if done > self.done {
            self.done = done;
            (self.on_scan)(done, self.starts.len() as u32);
        }
        Ok(n)
    }
}

impl<R: Seek> Seek for Counted<'_, R> {
    fn seek(&mut self, to: SeekFrom) -> std::io::Result<u64> {
        self.pos = self.inner.seek(to)?;
        Ok(self.pos)
    }
}

/// `to_mzml` that reports the real progress: `on_scan(0, total)` once the container is open (the scans are about to be converted),
/// then `on_scan(done, total)` as the scans go by (never backwards), and `on_scan(total, total)` at the end.
/// `profile` keeps the profile data of the file (much larger); otherwise the vendor centroids are written.
pub fn to_mzml_progress<R: Read + Seek, W: Write>(
    source: R,
    out: &mut W,
    name: &str,
    profile: bool,
    on_scan: &mut dyn FnMut(u32, u32),
) -> Result<()> {
    to_mzml_with(
        source,
        out,
        name,
        Convert {
            profile,
            anonymous: false,
        },
        on_scan,
    )
}

/// What `to_mzml_with` does besides the conversion: `profile` as above; `anonymous` leaves the personal data of the acquisition
/// (computer, user folders, paths, the values of the user labels) out of the mzML (`rawfilter`): they stay in the `.raw`.
#[derive(Clone, Copy, Default)]
pub struct Convert {
    pub profile: bool,
    pub anonymous: bool,
}

/// `to_mzml_progress` with the options of `Convert`.
pub fn to_mzml_with<R: Read + Seek, W: Write>(
    mut source: R,
    out: &mut W,
    name: &str,
    opt: Convert,
    on_scan: &mut dyn FnMut(u32, u32),
) -> Result<()> {
    let raw = open(&mut source)?;
    let mut starts: Vec<u64> = raw
        .scan_index
        .iter()
        .map(|e| raw.data_addr.saturating_add(e.offset))
        .collect();
    starts.sort_unstable();
    let total = starts.len() as u32;
    on_scan(0, total);
    let doc = method_userparam(&raw, &mut source, name, opt.anonymous);
    let mut counted = Counted {
        inner: source,
        starts,
        pos: 0,
        done: 0,
        on_scan,
    };
    let mut filter = MzmlFilter::new(&mut *out, opt.anonymous).with_run_extra(doc);
    opentfraw::write_mzml(&raw, &mut counted, &mut filter, name, opt.profile).map_err(raw_error)?;
    filter.flush().map_err(raw_error)?;
    (counted.on_scan)(total, total);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    #[test]
    fn counted_reports_scans_in_order_and_never_backwards() {
        let mut seen: Vec<(u32, u32)> = Vec::new();
        let mut cb = |d: u32, t: u32| seen.push((d, t));
        let mut c = Counted {
            inner: Cursor::new(vec![0u8; 100]),
            starts: vec![10, 30, 50, 70],
            pos: 0,
            done: 0,
            on_scan: &mut cb,
        };
        let mut b = [0u8; 20];
        while c.read(&mut b).unwrap() > 0 {}
        c.seek(SeekFrom::Start(0)).unwrap(); // a jump back (the container is read again) does not lower the count
        c.read(&mut b).unwrap();
        assert_eq!(seen, vec![(1, 4), (2, 4), (3, 4)]);
    }
}
