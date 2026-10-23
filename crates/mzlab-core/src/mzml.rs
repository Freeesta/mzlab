//! mzML reader driven by events (quick-xml) over any `Read`: the browser feeds blocks, the desktop a memory map.
//! Spectra: level, RT (minutes), polarity, precursor, filter string, m/z `f64`, intensity `f32`. Chromatograms: TIC/BPC/SRM/PDA.
//! Numpress is not supported (an error with a key, as in the Python reader).

use crate::mzmlarray::{decode_array, Inflate, Want};
use crate::numarray::Num;
use std::borrow::Cow;
use std::io::{BufReader, Read};

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
    /// Selected ion current of one SIM ion (MS:1000627); the id is "SIC <m/z>".
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
}

/// Reads the whole file into memory.
pub fn read<R: Read>(reader: R) -> Result<Run> {
    let mut run = Run::default();
    parse(reader, |item| match item {
        Item::Spectrum(s) => run.spectra.push(s),
        Item::Chromatogram(c) => run.chromatograms.push(c),
        Item::Analyzer(n) => run.analyzers.push(n.to_lowercase()),
        Item::Mobility(_) => {}
    })?;
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
    seconds: bool,
    mobility: &'static str,
    text: String,
}

#[derive(PartialEq, Clone, Copy)]
enum Scope {
    None,
    Spectrum,
    Chromatogram,
}

/// The attributes of an element as borrowed text (no allocation when the value has no escapes).
fn attrs<'a>(e: &'a BytesStart) -> Vec<(&'a str, Cow<'a, str>)> {
    let mut out = Vec::with_capacity(8);
    for a in e.attributes().with_checks(false).filter_map(|a| a.ok()) {
        let v = a
            .normalized_value(quick_xml::XmlVersion::Implicit1_0)
            .unwrap_or_default();
        // `Attribute::key` borrows the element's buffer, not the temporary `a`
        let key = a.key.into_inner();
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
    rt: f64,
    rt_seconds: bool,
    polarity: i8,
    profile: bool,
    filter: String,
    selected: Option<f64>,
    target: Option<f64>,
    mobility_scan: Option<(f64, &'static str)>,
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

fn finish_spectrum(s: Spec, inf: &mut Inflate) -> Result<(Spectrum, Option<Mobility>)> {
    let (mut mz, mut it, mut wide) = (Vec::new(), Vec::new(), None);
    let (mut mob, mut mob_unit) = (None, "");
    for a in &s.arrays {
        match a.kind {
            ArrayKind::Mobility => {
                mob = Some(decode_array(a.num, a.zlib, &a.text, s.len, inf, Want::F64)?.0);
                mob_unit = a.mobility;
            }
            ArrayKind::Mz => mz = decode_array(a.num, a.zlib, &a.text, s.len, inf, Want::F64)?.0,
            ArrayKind::Intensity => {
                if a.num == Num::F32 {
                    it = decode_array(a.num, a.zlib, &a.text, s.len, inf, Want::F32)?.1;
                    wide = None;
                } else {
                    let (w, f) = decode_array(a.num, a.zlib, &a.text, s.len, inf, Want::Both)?;
                    it = f;
                    wide = Some(w);
                }
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
    // an array of another length does not belong to these peaks
    let mob = mob.filter(|m| m.len() == n && n > 0);
    let mobility = (mob.is_some() || s.mobility_scan.is_some()).then_some(Mobility {
        index: s.index,
        values: mob,
        unit: mob_unit,
        scan: s.mobility_scan,
    });
    Ok((Spectrum {
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
    }, mobility))
}

fn number_after(id: &str, key: &str) -> Option<f64> {
    let k = id.find(key)? + key.len();
    let rest = &id[k..];
    let end = rest
        .find(|c: char| !(c.is_ascii_digit() || c == '.'))
        .unwrap_or(rest.len());
    rest[..end].parse().ok()
}

fn finish_chromatogram(c: Chrom, inf: &mut Inflate) -> Result<Chromatogram> {
    let (mut t, mut y) = (Vec::new(), Vec::new());
    let mut seconds = false;
    for a in &c.arrays {
        match a.kind {
            ArrayKind::Time => {
                t = decode_array(a.num, a.zlib, &a.text, c.len, inf, Want::F64)?.0;
                seconds = a.seconds;
            }
            ArrayKind::Intensity => {
                y = decode_array(a.num, a.zlib, &a.text, c.len, inf, Want::F64)?.0
            }
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
        s if s.starts_with("SIC ") => ChromKind::Sic,
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
    let mut inf = Inflate::new();

    loop {
        let ev = xml.read_event_into(&mut buf).map_err(xml_error)?;
        match ev {
            Event::Eof => break,
            Event::Start(ref e) | Event::Empty(ref e) => {
                let empty = matches!(ev, Event::Empty(_));
                let name = e.name().into_inner();
                match name {
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
                            mobility_scan: None,
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
                            num: Num::F64,
                            zlib: false,
                            seconds: false,
                            mobility: "",
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
                                "MS:1000235" | "MS:1000627" | "MS:1000628" | "MS:1001473"
                                | "MS:1001472" => c.ms_term = true,
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
                                "MS:1000574" => arr.zlib = true,
                                "MS:1002312" | "MS:1002313" | "MS:1002314" => {
                                    return Err(Error::new("err.file.numpress"))
                                }
                                _ => {
                                    if let Some(n) = Num::of_accession(acc) {
                                        arr.num = n;
                                    } else if let Some(u) = mobility_of(acc) {
                                        (arr.kind, arr.mobility) = (ArrayKind::Mobility, u);
                                    }
                                }
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
                                _ => s.mobility_scan = s.mobility_scan.or_else(|| mobility_of(acc).zip(value.parse().ok()).map(|(u, v)| (v, u))),
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
                let name = e.name().into_inner();
                match name {
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
                            let (sp, mob) = finish_spectrum(s, &mut inf)?;
                            mob.map(Item::Mobility).into_iter().chain([Item::Spectrum(sp)]).for_each(&mut f);
                            n_spec += 1;
                        }
                        scope = Scope::None;
                    }
                    "chromatogram" => {
                        if let Some(c) = chrom.take() {
                            f(Item::Chromatogram(finish_chromatogram(c, &mut inf)?));
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

#[cfg(test)]
#[path = "mzml_tests.rs"]
mod tests;
