//! mzML reader driven by events (quick-xml) over any `Read`: the browser feeds blocks, the desktop a memory map.
//! Spectra: level, RT (minutes), polarity, precursor, filter string, m/z `f64`, intensity `f32`. Chromatograms: TIC/BPC/SRM/PDA.
//! Numpress is not supported (an error with a key, as in the Python reader).

use std::io::{BufReader, Read};

use base64::Engine;
use quick_xml::events::{BytesStart, Event};
use quick_xml::Reader;

use crate::error::{Error, Result};

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

impl Spectrum {
    /// Intensity of peak `i` as the file wrote it.
    pub fn intensity_f64(&self, i: usize) -> f64 {
        match &self.wide {
            Some(w) => w[i],
            None => self.intensity[i] as f64,
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
    Chromatogram(Chromatogram),
    /// Name of a cvParam of an `<analyzer>` component (instrumentConfiguration), e.g. "quadrupole", "orbitrap".
    Analyzer(String),
}

#[derive(Debug, Default)]
pub struct Run {
    pub spectra: Vec<Spectrum>,
    pub chromatograms: Vec<Chromatogram>,
    /// Names of the analyzer components of the instrument (lower case), in file order.
    pub analyzers: Vec<String>,
}

/// Reads the whole file into memory.
pub fn read<R: Read>(reader: R) -> Result<Run> {
    let mut run = Run::default();
    parse(reader, |item| match item {
        Item::Spectrum(s) => run.spectra.push(s),
        Item::Chromatogram(c) => run.chromatograms.push(c),
        Item::Analyzer(n) => run.analyzers.push(n.to_lowercase()),
    })?;
    Ok(run)
}

#[derive(Clone, Copy, PartialEq)]
enum ArrayKind {
    Mz,
    Intensity,
    Time,
    Other,
}

struct Array {
    kind: ArrayKind,
    f32: bool,
    zlib: bool,
    seconds: bool,
    text: String,
}

#[derive(PartialEq, Clone, Copy)]
enum Scope {
    None,
    Spectrum,
    Chromatogram,
}

fn attrs(e: &BytesStart) -> Vec<(String, String)> {
    e.attributes()
        .with_checks(false)
        .filter_map(|a| a.ok())
        .map(|a| {
            let v = a
                .normalized_value(quick_xml::XmlVersion::Implicit1_0)
                .map(|c| c.into_owned())
                .unwrap_or_default();
            (a.key.as_ref().to_string(), v)
        })
        .collect()
}

fn get<'a>(a: &'a [(String, String)], name: &str) -> Option<&'a str> {
    a.iter().find(|(k, _)| k == name).map(|(_, v)| v.as_str())
}

fn xml_error(e: impl std::fmt::Display) -> Error {
    Error::new("err.file.xml").with("detail", e)
}

fn decode_array(a: &Array, expected: usize) -> Result<(Vec<f64>, Vec<f32>)> {
    let clean: String = a.text.chars().filter(|c| !c.is_whitespace()).collect();
    if clean.is_empty() {
        return Ok((Vec::new(), Vec::new()));
    }
    let raw = base64::engine::general_purpose::STANDARD
        .decode(clean.as_bytes())
        .map_err(|e| Error::new("err.file.base64").with("detail", e))?;
    let bytes = if a.zlib {
        let mut out = Vec::new();
        out.try_reserve(expected.saturating_mul(if a.f32 { 4 } else { 8 }))
            .map_err(|_| Error::memory_budget())?;
        flate2::read::ZlibDecoder::new(&raw[..])
            .read_to_end(&mut out)
            .map_err(|e| Error::new("err.file.zlib").with("detail", e))?;
        out
    } else {
        raw
    };
    let width = if a.f32 { 4 } else { 8 };
    let n = bytes.len() / width;
    let mut f64s: Vec<f64> = Vec::new();
    let mut f32s: Vec<f32> = Vec::new();
    f64s.try_reserve_exact(n)
        .map_err(|_| Error::memory_budget())?;
    if a.f32 {
        f32s.try_reserve_exact(n)
            .map_err(|_| Error::memory_budget())?;
        for c in bytes.chunks_exact(4) {
            let v = f32::from_le_bytes([c[0], c[1], c[2], c[3]]);
            f32s.push(v);
            f64s.push(v as f64);
        }
    } else {
        f32s.try_reserve_exact(n)
            .map_err(|_| Error::memory_budget())?;
        for c in bytes.chunks_exact(8) {
            let v = f64::from_le_bytes([c[0], c[1], c[2], c[3], c[4], c[5], c[6], c[7]]);
            f64s.push(v);
            f32s.push(v as f32);
        }
    }
    Ok((f64s, f32s))
}

struct Spec {
    index: usize,
    id: String,
    len: usize,
    level: u8,
    rt: f64,
    rt_seconds: bool,
    polarity: i8,
    profile: bool,
    filter: String,
    selected: Option<f64>,
    target: Option<f64>,
    arrays: Vec<Array>,
}

struct Chrom {
    index: usize,
    id: String,
    len: usize,
    arrays: Vec<Array>,
    unit: String,
    /// Accessions seen in the header of the chromatogram: they decide its kind like in `mzlab/reader/mzml.py`.
    pda: bool,
    pressure: bool,
    ms_term: bool,
}

fn finish_spectrum(s: Spec) -> Result<Spectrum> {
    let (mut mz, mut it, mut wide) = (Vec::new(), Vec::new(), None);
    for a in &s.arrays {
        match a.kind {
            ArrayKind::Mz => mz = decode_array(a, s.len)?.0,
            ArrayKind::Intensity => {
                let (w, f) = decode_array(a, s.len)?;
                it = f;
                wide = if a.f32 { None } else { Some(w) };
            }
            _ => {}
        }
    }
    let n = mz.len().min(it.len());
    mz.truncate(n);
    it.truncate(n);
    if let Some(w) = wide.as_mut() {
        w.truncate(n);
    }
    Ok(Spectrum {
        index: s.index,
        id: s.id,
        level: s.level,
        rt: if s.rt_seconds { s.rt / 60.0 } else { s.rt },
        polarity: s.polarity,
        profile: s.profile,
        filter: s.filter,
        precursor: s.selected.or(s.target),
        mz,
        intensity: it,
        wide,
    })
}

fn number_after(id: &str, key: &str) -> Option<f64> {
    let k = id.find(key)? + key.len();
    let rest = &id[k..];
    let end = rest
        .find(|c: char| !(c.is_ascii_digit() || c == '.'))
        .unwrap_or(rest.len());
    rest[..end].parse().ok()
}

fn finish_chromatogram(c: Chrom) -> Result<Chromatogram> {
    let (mut t, mut y) = (Vec::new(), Vec::new());
    let mut seconds = false;
    for a in &c.arrays {
        match a.kind {
            ArrayKind::Time => {
                t = decode_array(a, c.len)?.0;
                seconds = a.seconds;
            }
            ArrayKind::Intensity => y = decode_array(a, c.len)?.0,
            _ => {}
        }
    }
    let n = t.len().min(y.len());
    t.truncate(n);
    y.truncate(n);
    if seconds {
        t.iter_mut().for_each(|v| *v /= 60.0);
    }
    let kind = match c.id.as_str() {
        "TIC" => ChromKind::Tic,
        "BPC" => ChromKind::Bpc,
        "TWC" => ChromKind::Pda,
        _ if c.pda => ChromKind::Pda,
        s if c.pressure || s.starts_with("Pump Pressure") => ChromKind::Pressure,
        s if s.starts_with("SRM") => ChromKind::Srm,
        _ if !c.ms_term => ChromKind::Analog,
        _ => ChromKind::Other,
    };
    let (q1, q3) = if kind == ChromKind::Srm {
        (number_after(&c.id, "Q1="), number_after(&c.id, "Q3="))
    } else {
        (None, None)
    };
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

/// Parses an mzML stream, calling `f` for every spectrum and chromatogram in file order.
pub fn parse<R: Read, F: FnMut(Item)>(reader: R, mut f: F) -> Result<()> {
    let mut xml = Reader::from_reader(BufReader::with_capacity(1 << 16, reader));
    let mut buf = Vec::new();
    let mut scope = Scope::None;
    let (mut n_spec, mut n_chrom) = (0usize, 0usize);
    let mut spec: Option<Spec> = None;
    let mut chrom: Option<Chrom> = None;
    let mut array: Option<Array> = None;
    let mut in_binary = false;
    let mut in_precursor_list = false;
    let mut in_analyzer = false;

    loop {
        let ev = xml.read_event_into(&mut buf).map_err(xml_error)?;
        match ev {
            Event::Eof => break,
            Event::Start(ref e) | Event::Empty(ref e) => {
                let empty = matches!(ev, Event::Empty(_));
                let name = e.name().as_ref().to_string();
                match name.as_str() {
                    "spectrum" => {
                        let a = attrs(e);
                        spec = Some(Spec {
                            index: get(&a, "index")
                                .and_then(|v| v.parse().ok())
                                .unwrap_or(n_spec),
                            id: get(&a, "id").unwrap_or("").to_string(),
                            len: get(&a, "defaultArrayLength")
                                .and_then(|v| v.parse().ok())
                                .unwrap_or(0),
                            level: 1,
                            rt: 0.0,
                            rt_seconds: false,
                            polarity: 0,
                            profile: false,
                            filter: String::new(),
                            selected: None,
                            target: None,
                            arrays: Vec::new(),
                        });
                        scope = Scope::Spectrum;
                        in_precursor_list = false;
                    }
                    "chromatogram" => {
                        let a = attrs(e);
                        chrom = Some(Chrom {
                            index: get(&a, "index")
                                .and_then(|v| v.parse().ok())
                                .unwrap_or(n_chrom),
                            id: get(&a, "id").unwrap_or("").to_string(),
                            len: get(&a, "defaultArrayLength")
                                .and_then(|v| v.parse().ok())
                                .unwrap_or(0),
                            arrays: Vec::new(),
                            unit: String::new(),
                            pda: false,
                            pressure: false,
                            ms_term: false,
                        });
                        scope = Scope::Chromatogram;
                    }
                    "precursorList" => in_precursor_list = true,
                    "analyzer" if !empty => in_analyzer = true,
                    "binaryDataArray" => {
                        array = Some(Array {
                            kind: ArrayKind::Other,
                            f32: false,
                            zlib: false,
                            seconds: false,
                            text: String::new(),
                        });
                    }
                    "binary" if !empty => in_binary = true,
                    "cvParam" => {
                        let a = attrs(e);
                        if in_analyzer {
                            f(Item::Analyzer(get(&a, "name").unwrap_or("").to_string()));
                        }
                        let acc = get(&a, "accession").unwrap_or("");
                        let value = get(&a, "value").unwrap_or("");
                        let unit = get(&a, "unitName").unwrap_or("");
                        let unit_acc = get(&a, "unitAccession").unwrap_or("");
                        let seconds = unit.starts_with("second") || unit_acc == "UO:0000010";
                        if let (Scope::Chromatogram, Some(c)) = (scope, chrom.as_mut()) {
                            match acc {
                                "MS:1000812" | "MS:1000813" => c.pda = true,
                                "MS:1003019" => c.pressure = true,
                                "MS:1000235" | "MS:1000627" | "MS:1000628" | "MS:1001473" | "MS:1001472" => c.ms_term = true,
                                _ => {}
                            }
                        }
                        if let Some(arr) = array.as_mut() {
                            match acc {
                                "MS:1000514" => arr.kind = ArrayKind::Mz,
                                "MS:1000515" => {
                                    arr.kind = ArrayKind::Intensity;
                                    if let (false, Some(c)) = (unit.is_empty(), chrom.as_mut()) {
                                        c.unit = unit.to_string();
                                    }
                                }
                                "MS:1000595" => {
                                    arr.kind = ArrayKind::Time;
                                    arr.seconds = seconds;
                                }
                                "MS:1000521" => arr.f32 = true,
                                "MS:1000523" => arr.f32 = false,
                                "MS:1000574" => arr.zlib = true,
                                "MS:1002312" | "MS:1002313" | "MS:1002314" => {
                                    return Err(Error::new("err.file.numpress"))
                                }
                                _ => {}
                            }
                        } else if let (Scope::Spectrum, Some(s)) = (scope, spec.as_mut()) {
                            match acc {
                                "MS:1000511" if !in_precursor_list => {
                                    s.level = value.parse::<f64>().map(|v| v as u8).unwrap_or(1)
                                }
                                "MS:1000130" => s.polarity = 1,
                                "MS:1000129" => s.polarity = -1,
                                "MS:1000128" => s.profile = true,
                                "MS:1000512" if !in_precursor_list => s.filter = value.to_string(),
                                "MS:1000016" => {
                                    s.rt = value.parse().unwrap_or(0.0);
                                    s.rt_seconds = seconds;
                                }
                                "MS:1000744" => s.selected = value.parse().ok(),
                                "MS:1000827" => s.target = value.parse().ok(),
                                _ => {}
                            }
                        }
                    }
                    _ => {}
                }
                if empty && name == "binaryDataArray" {
                    // an empty element has no data: close it at once
                    if let (Some(arr), Some(s)) = (array.take(), spec.as_mut()) {
                        s.arrays.push(arr);
                    }
                }
            }
            Event::Text(ref t) if in_binary => {
                if let Some(arr) = array.as_mut() {
                    arr.text.push_str(&t.xml10_content());
                }
            }
            Event::End(ref e) => {
                let name = e.name().as_ref().to_string();
                match name.as_str() {
                    "binary" => in_binary = false,
                    "binaryDataArray" => {
                        if let Some(arr) = array.take() {
                            match scope {
                                Scope::Spectrum => spec.as_mut().map(|s| s.arrays.push(arr)),
                                Scope::Chromatogram => chrom.as_mut().map(|c| c.arrays.push(arr)),
                                Scope::None => None,
                            };
                        }
                    }
                    "precursorList" => in_precursor_list = false,
                    "analyzer" => in_analyzer = false,
                    "spectrum" => {
                        if let Some(s) = spec.take() {
                            f(Item::Spectrum(finish_spectrum(s)?));
                            n_spec += 1;
                        }
                        scope = Scope::None;
                    }
                    "chromatogram" => {
                        if let Some(c) = chrom.take() {
                            f(Item::Chromatogram(finish_chromatogram(c)?));
                            n_chrom += 1;
                        }
                        scope = Scope::None;
                    }
                    _ => {}
                }
            }
            _ => {}
        }
        buf.clear();
    }
    Ok(())
}
