//! mzML reader driven by events (quick-xml) over any `Read`: the browser feeds blocks, the desktop a memory map.
//! Spectra: level, RT (minutes), polarity, precursor, filter string, m/z `f64`, intensity `f32`. Chromatograms: TIC/BPC/SRM/PDA.
//! Numpress is not supported (an error with a key, as in the Python reader).

use crate::mzmlarray::{decode_array, Inflate, Want};
mod cv;
pub(crate) mod help;
use help::Decode;
use crate::numarray::Num;
use std::borrow::Cow;
use std::collections::HashMap;
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
    /// An optical (`MS:1000804`) spectrum: read by `mzml_dad`, not a scan.
    uv: bool,
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
        s if s == "TIC" || s.starts_with("TIC ") => ChromKind::Tic,
        "BPC" => ChromKind::Bpc,
        "TWC" => ChromKind::Pda,
        _ if c.pda => ChromKind::Pda,
        s if c.pressure || s.starts_with("Pump Pressure") => ChromKind::Pressure,
        s if s.starts_with("SRM") => ChromKind::Srm,
        s if s.starts_with("SIC ") || s.starts_with("SIM SIC ") => ChromKind::Sic,
        _ if !c.ms_term => ChromKind::Analog,
        _ => ChromKind::Other,
    };
    let (q1, q3) = if kind == ChromKind::Srm {
        (number_after(&c.id, "Q1="), number_after(&c.id, "Q3="))
    } else if kind == ChromKind::Sic {
        // msconvert names a SIM trace "SIM SIC Q1=<m/z> ..."; the plain "SIC <m/z>" of our own writer has no Q1
        (number_after(&c.id, "Q1="), None)
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

/// Parses an mzML stream, calling `f` for every spectrum and chromatogram in file order. Spare cores decode the arrays (`help`); the items and their
/// order are the same without them.
pub fn parse<R: Read, F: FnMut(Item)>(reader: R, mut f: F) -> Result<()> {
    #[cfg(not(target_family = "wasm"))]
    {
        let lease = crate::spare::lease(help::pool::HELPERS);
        help::pool::parse_helped(reader, &mut f, lease.count())
    }
    #[cfg(target_family = "wasm")]
    help::parse_with(reader, &mut f, &mut help::Plain(Inflate::new()))
}

fn parse_loop<R: Read, D: Decode>(reader: R, f: &mut dyn FnMut(Item), out: &mut D) -> Result<()> {
    let mut xml = Reader::from_reader(BufReader::with_capacity(1 << 16, reader));
    let mut buf = Vec::new();
    let (mut n_spec, mut n_chrom) = (0usize, 0usize);
    let mut cur = cv::Cur::default();
    let mut in_binary = false;

    let mut groups: HashMap<String, Vec<[String; 5]>> = HashMap::new();
    // the `referenceableParamGroup` being read: id and its cvParams (name, accession, value, unitName, unitAccession)
    let mut group: Option<(String, Vec<[String; 5]>)> = None;
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
                        cur.spec = Some(Spec {
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
                            uv: false,
                            arrays: Vec::new(),
                        });
                        cur.scope = Scope::Spectrum;
                        cur.in_precursor_list = false;
                    }
                    "chromatogram" => {
                        let a = attrs(e);
                        cur.chrom = Some(Chrom {
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
                        cur.scope = Scope::Chromatogram;
                    }
                    "precursorList" => cur.in_precursor_list = true,
                    "analyzer" if !empty => cur.in_analyzer = true,
                    "binaryDataArray" => {
                        cur.array = Some(Array {
                            kind: ArrayKind::Other,
                            num: Num::F64,
                            zlib: false,
                            seconds: false,
                            mobility: "",
                            text: String::new(),
                        });
                    }
                    "binary" if !empty => in_binary = true,
                    "referenceableParamGroup" if !empty => {
                        group = get(&attrs(e), "id").map(|id| (id.to_string(), Vec::new()));
                    }
                    "cvParam" if group.is_some() => {
                        let a = attrs(e);
                        let own = |k| get(&a, k).unwrap_or("").to_string();
                        let cv = [own("name"), own("accession"), own("value"), own("unitName"), own("unitAccession")];
                        if let Some((_, list)) = group.as_mut() {
                            list.push(cv);
                        }
                    }
                    "cvParam" | "referenceableParamGroupRef" => {
                        let a = attrs(e);
                        // a cvParam is one entry; a group reference replays the cvParams of its group, in the same order
                        let group_list: Vec<[&str; 5]>;
                        let one: [[&str; 5]; 1];
                        let list: &[[&str; 5]] = if name == "cvParam" {
                            one = [[
                                get(&a, "name").unwrap_or(""),
                                get(&a, "accession").unwrap_or(""),
                                get(&a, "value").unwrap_or(""),
                                get(&a, "unitName").unwrap_or(""),
                                get(&a, "unitAccession").unwrap_or(""),
                            ]];
                            &one
                        } else {
                            group_list = get(&a, "ref")
                                .and_then(|r| groups.get(r))
                                .into_iter()
                                .flatten()
                                .map(|cv| [&*cv[0], &*cv[1], &*cv[2], &*cv[3], &*cv[4]])
                                .collect();
                            &group_list
                        };
                        for &cv in list {
                            cur.apply(cv, out, f)?;
                        }
                    }
                    _ => {}
                }
                if empty && name == "binaryDataArray" {
                    // an empty element has no data: close it at once
                    if let (Some(arr), Some(s)) = (cur.array.take(), cur.spec.as_mut()) {
                        s.arrays.push(arr);
                    }
                }
            }
            Event::Text(ref t) if in_binary => {
                if let Some(arr) = cur.array.as_mut() {
                    arr.text.push_str(&t.xml10_content());
                }
            }
            Event::End(ref e) => {
                let name = e.name().into_inner();
                match name {
                    "binary" => in_binary = false,
                    "binaryDataArray" => {
                        if let Some(arr) = cur.array.take() {
                            match cur.scope {
                                Scope::Spectrum => cur.spec.as_mut().map(|s| s.arrays.push(arr)),
                                Scope::Chromatogram => cur.chrom.as_mut().map(|c| c.arrays.push(arr)),
                                Scope::None => None,
                            };
                        }
                    }
                    "precursorList" => cur.in_precursor_list = false,
                    "analyzer" => cur.in_analyzer = false,
                    "referenceableParamGroup" => {
                        if let Some((id, list)) = group.take() {
                            groups.insert(id, list);
                        }
                    }
                    "spectrum" => {
                        if let Some(s) = cur.spec.take().filter(|s| !s.uv) {
                            out.spectrum(s, f)?;
                            n_spec += 1;
                        }
                        cur.scope = Scope::None;
                    }
                    "chromatogram" => {
                        if let Some(c) = cur.chrom.take() {
                            out.chromatogram(c, f)?;
                            n_chrom += 1;
                        }
                        cur.scope = Scope::None;
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
