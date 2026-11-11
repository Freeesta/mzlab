//! mzML reader driven by events (quick-xml) over any `Read`: the browser feeds blocks, the desktop a memory map.
//! Spectra: level, RT (minutes), polarity, precursor, filter string, m/z `f64`, intensity `f32`. Chromatograms: TIC/BPC/SRM/PDA.
//! Numpress is not supported (an error with a key, as in the Python reader).

use crate::mzmlarray::{decode_array, Inflate, Want};
mod build;
pub(crate) mod help;
mod peaks;
#[cfg(test)]
mod tests_head;
use crate::numarray::Num;
use crate::spechead::{self, tags::local};
pub use help::courier::{Corriere, Decodificatore};
use help::Decode;
use std::borrow::Cow;
use std::io::{BufReader, Read};

use quick_xml::events::{BytesStart, Event};
use quick_xml::Reader;

use crate::error::{Error, Result};
use crate::readers::Losses;

#[derive(Debug, Clone, PartialEq, Default)]
pub struct Spectrum {
    pub index: usize,
    pub id: String,
    pub level: u8,
    /// Retention time in minutes.
    pub rt: f64,
    /// +1, -1 or 0 (not said).
    pub polarity: i8,
    pub profile: bool,
    pub filter: String,
    /// Selected ion m/z, or the isolation target when the file has no selected ion.
    pub precursor: Option<f64>,
    pub mz: Vec<f64>,
    pub intensity: Vec<f32>,
    /// The intensities with the precision of the file, only when it stores them as 64-bit (`None`: `intensity` as `f64` is exact).
    pub wide: Option<Vec<f64>>,
}

/// The kind of a mobility array or scan attribute from its accession: `k0` (inverse reduced), `ms` (drift time) or `` (generic); `None` for any other accession.
pub fn mobility_of(acc: &str) -> Option<&'static str> {
    match acc {
        "MS:1002815" | "MS:1003006" | "MS:1003008" | "MS:1003155" => Some("k0"),
        "MS:1002476" | "MS:1002477" | "MS:1003153" | "MS:1003156" => Some("ms"),
        "MS:1002816" | "MS:1003007" | "MS:1003154" | "MS:1002893" => Some(""),
        _ => None,
    }
}

impl Spectrum {
    /// Intensity of peak `i` as the file wrote it.
    pub fn intensity_f64(&self, i: usize) -> f64 {
        match &self.wide {
            Some(w) => w.get(i).copied().unwrap_or(0.0),
            None => self.intensity.get(i).map_or(0.0, |&x| x as f64),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChromKind {
    Tic,
    Bpc,
    Pda,
    /// Pump pressure (MS:1003019, or an id that starts with "Pump Pressure").
    Pressure,
    Srm,
    /// Selected ion current of one SIM ion (MS:1000627); the id is "SIC <m/z>" (or "SIM SIC Q1=<m/z> ..." from msconvert, whose m/z is `q1`).
    Sic,
    /// Any other detector or device trace: it carries no mass-spectrometer term.
    Analog,
    Other,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Chromatogram {
    pub index: usize,
    pub id: String,
    pub kind: ChromKind,
    pub q1: Option<f64>,
    pub q3: Option<f64>,
    /// `unitName` of the intensity array ("" when it has none).
    pub unit: String,
    /// Minutes.
    pub time: Vec<f64>,
    pub intensity: Vec<f64>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Item {
    Spectrum(Spectrum),
    /// The ion mobility of the spectrum that follows, only for a spectrum that has any (an ion mobility array or a scan attribute).
    Mobility(Mobility),
    Chromatogram(Chromatogram),
    /// Name of a cvParam of an `<analyzer>` component (instrumentConfiguration), e.g. "quadrupole", "orbitrap".
    Analyzer(String),
}

/// The ion mobility of one spectrum: `unit` is `k0` (inverse reduced mobility, V·s/cm²), `ms` (drift time) or `` (an array the CV does not name more precisely).
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Mobility {
    pub index: usize,
    /// One value per peak, only when the array has as many values as the spectrum has peaks.
    pub values: Option<Vec<f64>>,
    pub unit: &'static str,
    /// The mobility of the whole scan (`MS:1002815` or `MS:1002476`) and its unit.
    pub scan: Option<(f64, &'static str)>,
}

#[derive(Debug, Default)]
pub struct Run {
    pub spectra: Vec<Spectrum>,
    pub chromatograms: Vec<Chromatogram>,
    /// Names of the analyzer components of the instrument (lower case), in file order.
    pub analyzers: Vec<String>,
    /// The spectra the file declares and the spectra read (`parse_counted`).
    pub losses: Losses,
}

/// Reads the whole file into memory.
pub fn read<R: Read>(reader: R) -> Result<Run> {
    let mut run = Run::default();
    let losses = parse_counted(reader, |item| match item {
        Item::Spectrum(s) => run.spectra.push(s),
        Item::Chromatogram(c) => run.chromatograms.push(c),
        Item::Analyzer(n) => run.analyzers.push(n.to_lowercase()),
        Item::Mobility(_) => {}
    })?;
    run.losses = losses;
    Ok(run)
}

#[derive(Clone, Copy, PartialEq)]
enum ArrayKind {
    Mz,
    Intensity,
    Time,
    /// An ion mobility array; the unit is in `Array.mobility`.
    Mobility,
    Other,
}

struct Array {
    kind: ArrayKind,
    num: Num,
    zlib: bool,
    /// Minutes per unit of a time array.
    minutes: f64,
    mobility: &'static str,
    text: String,
}

/// The attributes of an element as borrowed text (no allocation when the value has no escapes).
fn attrs<'a>(e: &'a BytesStart) -> Vec<(&'a str, Cow<'a, str>)> {
    let mut out = Vec::with_capacity(8);
    for a in e.attributes().with_checks(false).filter_map(|a| a.ok()) {
        let v = a
            .normalized_value(quick_xml::XmlVersion::Implicit1_0)
            .unwrap_or_default();
        // `Attribute::key` borrows the element's buffer, not the temporary `a`
        let key = local(a.key.into_inner());
        out.push((key, v));
    }
    out
}

fn get<'a>(a: &'a [(&str, Cow<str>)], name: &str) -> Option<&'a str> {
    a.iter().find(|(k, _)| *k == name).map(|(_, v)| v.as_ref())
}

fn xml_error(e: impl std::fmt::Display) -> Error {
    Error::new("err.file.xml").with("detail", e)
}

struct Spec {
    index: usize,
    id: String,
    len: usize,
    level: u8,
    /// Minutes.
    rt: f64,
    polarity: i8,
    profile: bool,
    filter: String,
    /// The selected ion (or isolation target) of the immediate precursor.
    precursor: Option<f64>,
    mobility_scan: Option<(f64, &'static str)>,
    arrays: Vec<Array>,
}

struct Chrom {
    index: usize,
    id: String,
    len: usize,
    arrays: Vec<Array>,
    unit: String,
    /// Terms of the chromatogram: they decide its kind like in `mzlab/reader/mzml.py`.
    pda: bool,
    pressure: bool,
    ms_term: bool,
    /// `MS:1001473` (selected reaction monitoring chromatogram).
    srm: bool,
    /// The targets of the precursor and product isolation windows.
    q1: Option<f64>,
    q3: Option<f64>,
}

fn finish_spectrum(s: Spec, inf: &mut Inflate) -> Result<(Spectrum, Option<Mobility>)> {
    let p = peaks::decode_peaks(s.arrays.iter().map(Array::arr), s.len, inf)?;
    peaks::assemble(s, p)
}

fn finish_chromatogram(c: Chrom, inf: &mut Inflate) -> Result<Chromatogram> {
    let (mut t, mut y) = (Vec::new(), Vec::new());
    let mut minutes = 1.0;
    for a in &c.arrays {
        match a.kind {
            ArrayKind::Time => {
                t = decode_array(a.num, a.zlib, &a.text, c.len, inf, Want::F64)?.0;
                minutes = a.minutes;
            }
            ArrayKind::Intensity => {
                y = decode_array(a.num, a.zlib, &a.text, c.len, inf, Want::F64)?.0
            }
            _ => {}
        }
    }
    if t.len() != y.len() {
        return Err(lengths(&c.id, c.index, t.len(), y.len()));
    }
    if minutes != 1.0 {
        t.iter_mut().for_each(|v| *v *= minutes);
    }
    let kind = match c.id.as_str() {
        s if s == "TIC" || s.starts_with("TIC ") => ChromKind::Tic,
        "BPC" => ChromKind::Bpc,
        "TWC" => ChromKind::Pda,
        _ if c.pda => ChromKind::Pda,
        s if c.pressure || s.starts_with("Pump Pressure") => ChromKind::Pressure,
        s if s.starts_with("SRM") => ChromKind::Srm,
        _ if c.srm => ChromKind::Srm,
        s if s.starts_with("SIC ") || s.starts_with("SIM SIC ") => ChromKind::Sic,
        _ if !c.ms_term => ChromKind::Analog,
        _ => ChromKind::Other,
    };
    let q1 = || c.q1.or_else(|| build::number_after(&c.id, "Q1="));
    let (q1, q3) = if kind == ChromKind::Srm {
        (q1(), c.q3.or_else(|| build::number_after(&c.id, "Q3=")))
    } else if kind == ChromKind::Sic {
        // msconvert names a SIM trace "SIM SIC Q1=<m/z> ..."; the plain "SIC <m/z>" of our own writer has no Q1
        (q1(), None)
    } else {
        (None, None)
    };
    crate::mzmlarray::finite_points(&c.id, &t, &y)?;
    Ok(Chromatogram {
        index: c.index,
        id: c.id,
        kind,
        q1,
        q3,
        unit: c.unit,
        time: t,
        intensity: y,
    })
}

/// Parses an mzML stream, calling `f` for every spectrum and chromatogram in file order. Spare cores decode the arrays (`help`); the items and their
/// order are the same without them.
pub fn parse<R: Read, F: FnMut(Item)>(reader: R, f: F) -> Result<()> {
    parse_counted(reader, f).map(|_| ())
}

/// `parse` that also says how many spectra the file declares (its `spectrumList` count) and how many were read: fewer read is a cut file
/// (`readers::TRUNCATED`) or a list whose count is wrong (`warn.file.mzmlFewer`), never silent.
pub fn parse_counted<R: Read, F: FnMut(Item)>(reader: R, mut f: F) -> Result<Losses> {
    #[cfg(not(target_family = "wasm"))]
    {
        let lease = crate::spare::lease(help::pool::HELPERS);
        help::pool::parse_helped(reader, &mut f, lease.count())
    }
    #[cfg(target_family = "wasm")]
    help::parse_with(reader, &mut f, &mut help::Plain(Inflate::new()))
}

/// `parse` with the arrays decoded by another instance through `c` (the browser's helper workers, `help::corriere`): the same items in the same order.
pub fn parse_corriere<R: Read, F: FnMut(Item)>(
    reader: R,
    mut f: F,
    c: &mut dyn Corriere,
) -> Result<()> {
    help::parse_with(reader, &mut f, &mut help::courier::Sent::new(c)).map(|_| ())
}

/// A spectrum or chromatogram whose two arrays differ in length: a damaged file, never cut to the shorter.
pub(crate) fn lengths(id: &str, index: usize, a: usize, b: usize) -> Error {
    let chi = if id.is_empty() {
        (index + 1).to_string()
    } else {
        id.to_string()
    };
    Error::new("err.file.mzmlLengths")
        .with("scan", chi)
        .with("a", a)
        .with("b", b)
}

/// The tags of the file go to the shared header reader (`spechead`, local names, CDATA as text); each finished header becomes an item. The analyzer
/// names of the instrument are read here.
fn parse_loop<R: Read>(reader: R, f: &mut dyn FnMut(Item), out: &mut dyn Decode) -> Result<Losses> {
    let mut xml = Reader::from_reader(BufReader::with_capacity(1 << 16, reader));
    let mut buf = Vec::new();
    let (mut n_spec, mut n_chrom) = (0usize, 0usize);
    let mut rd = spechead::Reader::default();
    rd.keep_binary = true;
    let mut levels = build::Levels::default();
    let mut in_analyzer = false;
    // items the lists declare and the items met: a file that declares some and shows none was not understood
    let (mut declared, mut met) = (0usize, 0usize);
    // the spectra the list declares, the spectra whose end was read (optical ones included) and whether the document closes
    let (mut spec_declared, mut spec_read, mut closed) = (0usize, 0usize, false);
    loop {
        let ev = xml.read_event_into(&mut buf).map_err(xml_error)?;
        match ev {
            Event::Eof => break,
            Event::Start(ref e) | Event::Empty(ref e) => {
                let empty = matches!(ev, Event::Empty(_));
                let name = local(e.local_name().into_inner());
                let a = attrs(e);
                match name {
                    "analyzer" if !empty => in_analyzer = true,
                    "cvParam" if in_analyzer => {
                        out.item(Item::Analyzer(get(&a, "name").unwrap_or("").to_string()), f)?
                    }
                    "spectrumList" | "chromatogramList" => {
                        let n: usize = get(&a, "count")
                            .and_then(|v| v.trim().parse().ok())
                            .unwrap_or(0);
                        declared += n;
                        if name == "spectrumList" {
                            spec_declared += n;
                        }
                    }
                    "spectrum" | "chromatogram" => met += 1,
                    _ => {}
                }
                rd.start(name, &a, empty);
            }
            Event::Text(ref t) if rd.in_binary() => {
                // base64 has no entities and decode_array drops white space itself: take the raw text unless it has a reference
                if t.contains('&') {
                    rd.text(&t.xml10_content());
                } else {
                    rd.text(t);
                }
            }
            Event::CData(ref t) if rd.in_binary() => rd.text(&t[..]),
            Event::End(ref e) => {
                let name = local(e.local_name().into_inner());
                match name {
                    "analyzer" => in_analyzer = false,
                    "mzML" => closed = true,
                    _ => {}
                }
                match rd.end(name) {
                    Some(h) if h.chromatogram => {
                        out.chromatogram(build::chromatogram(h, n_chrom)?, f)?;
                        n_chrom += 1;
                    }
                    Some(h) => {
                        spec_read += 1;
                        let (id, level) = (h.id.clone(), h.level());
                        if let Some(s) = build::spectrum(h, n_spec, &levels)? {
                            out.spectrum(s, f)?;
                            n_spec += 1;
                        }
                        levels.push(id, level);
                    }
                    None => {}
                }
            }
            _ => {}
        }
        buf.clear();
    }
    if declared > 0 && met == 0 {
        return Err(Error::new("err.file.mzmlNoItems").with("count", declared));
    }
    let cause = if closed {
        FEWER
    } else {
        crate::readers::TRUNCATED
    };
    Ok(Losses::of(spec_declared, spec_read, cause))
}

/// Cause of a complete file whose `spectrumList` declares more spectra than it holds.
pub const FEWER: &str = "warn.file.mzmlFewer";

/// Whether a time cvParam is in seconds: its `unitName` starts with "second" or its `unitAccession` is UO:0000010 (either alone is enough).
/// The one rule of every mzML reader of the engine.
pub(crate) fn is_seconds(unit_name: Option<&str>, unit_acc: Option<&str>) -> bool {
    unit_name.is_some_and(|u| u.starts_with("second")) || unit_acc == Some("UO:0000010")
}

#[cfg(test)]
#[path = "mzml_tests.rs"]
mod tests;
