//! `AcqData/MSScan.bin`: a header (160 bytes in the runs seen; the `i32` at +88 is the offset of the first record) and then fixed records, one per scan.
//! The record is the `ScanRecordType` of `AcqData/MSScan.xsd` packed without padding, `SpectrumParamValues` last: the xsd that the data folder carries says which
//! optional fields this file has, so the layout is read from it (`Layout::from_xsd`). For the three GC/MS runs the xsd gives 184 bytes and the same offsets as the
//! built-in list (`Layout::builtin`, the fallback when the folder has no xsd). A layout that does not divide the file, a field of an unknown type or a missing
//! required field is an error that names the numbers, never a guess.

use crate::error::{Error, Result};

const DATA_OFFSET_AT: usize = 88;
pub const HEADER_LEN: usize = 160;
const HEADER_MIN: usize = HEADER_LEN;

#[derive(Debug, Clone, Copy, PartialEq)]
enum Kind {
    I8,
    I16,
    I32,
    I64,
    F32,
    F64,
}

impl Kind {
    fn size(self) -> usize {
        match self {
            Kind::I8 => 1,
            Kind::I16 => 2,
            Kind::I32 | Kind::F32 => 4,
            Kind::I64 | Kind::F64 => 8,
        }
    }
    fn of_xsd(t: &str) -> Option<Kind> {
        Some(match t.rsplit(':').next()? {
            "byte" | "unsignedByte" | "boolean" => Kind::I8,
            "short" | "unsignedShort" => Kind::I16,
            "int" | "unsignedInt" => Kind::I32,
            "long" | "unsignedLong" => Kind::I64,
            "float" => Kind::F32,
            "double" => Kind::F64,
            _ => return None,
        })
    }
}

/// The fields of the verified layout, in order: the scan part and the spectrum part (`SpectrumParamValues`).
const SCAN_FIELDS: &[(&str, Kind)] = &[
    ("ScanID", Kind::I32),
    ("ScanMethodID", Kind::I32),
    ("TimeSegmentID", Kind::I32),
    ("ScanTime", Kind::F64),
    ("MSLevel", Kind::I32),
    ("ScanType", Kind::I32),
    ("TIC", Kind::F64),
    ("BasePeakMZ", Kind::F64),
    ("BasePeakValue", Kind::F64),
    ("CycleNumber", Kind::I32),
    ("Status", Kind::I32),
    ("IonMode", Kind::I32),
    ("IonPolarity", Kind::I32),
    ("Fragmentor", Kind::F64),
    ("CollisionEnergy", Kind::F64),
    ("MzOfInterest", Kind::F64),
    ("SamplingPeriod", Kind::F64),
    ("MeasuredMassRangeMin", Kind::F64),
    ("MeasuredMassRangeMax", Kind::F64),
    ("Threshold", Kind::F64),
    ("IsFragmentorDynamic", Kind::I32),
    ("IsCollisionEnergyDynamic", Kind::I32),
];
const SPECTRUM_FIELDS: &[(&str, Kind)] = &[
    ("SpectrumFormatID", Kind::I32),
    ("SpectrumOffset", Kind::I64),
    ("ByteCount", Kind::I32),
    ("PointCount", Kind::I32),
    ("MinY", Kind::F64),
    ("MaxY", Kind::F64),
    ("MinX", Kind::F64),
    ("MaxX", Kind::F64),
];
/// Fields that a layout must have: without them there is no scan to read.
const REQUIRED: &[&str] = &[
    "ScanTime",
    "SpectrumOffset",
    "ByteCount",
    "PointCount",
    "SpectrumFormatID",
];

#[derive(Debug, Clone, PartialEq)]
struct Field {
    name: String,
    kind: Kind,
    at: usize,
}

/// Where each field of a record lies.
#[derive(Debug, Clone, PartialEq)]
pub struct Layout {
    fields: Vec<Field>,
    pub record_len: usize,
}

impl Layout {
    fn new(scan: &[(String, Kind)], spectrum: &[(String, Kind)]) -> Result<Layout> {
        let mut at = 0;
        let mut fields = Vec::new();
        for (name, kind) in scan.iter().chain(spectrum) {
            fields.push(Field {
                name: name.clone(),
                kind: *kind,
                at,
            });
            at += kind.size();
        }
        if let Some(m) = REQUIRED
            .iter()
            .find(|r| !fields.iter().any(|f| f.name == **r))
        {
            return Err(bad(format!("the record layout has no {m}")));
        }
        Ok(Layout {
            fields,
            record_len: at,
        })
    }

    /// The layout of the three GC/MS runs (184 bytes).
    pub fn builtin() -> Layout {
        let own = |l: &[(&str, Kind)]| {
            l.iter()
                .map(|(n, k)| (n.to_string(), *k))
                .collect::<Vec<_>>()
        };
        Layout::new(&own(SCAN_FIELDS), &own(SPECTRUM_FIELDS))
            .expect("the built-in layout is complete")
    }
}

fn bad(detail: String) -> Error {
    Error::new("err.file.agilent").with("detail", detail)
}

impl Layout {
    /// The layout `MSScan.xsd` describes: the elements of `ScanRecordType` in order (the `SpectrumParamValues` element stands for the elements of
    /// `SpectrumParamsType`), commented-out elements left out. One spectrum per scan only.
    pub fn from_xsd(xsd: &str) -> Result<Layout> {
        let clean = strip_comments(xsd);
        let scan = elements(&clean, "ScanRecordType")?;
        let spectrum = elements(&clean, "SpectrumParamsType")?;
        let mut head = Vec::new();
        for (name, ty) in &scan {
            if name == "SpectrumParamValues" {
                if ty != "SpectrumParamsType" {
                    return Err(bad(format!(
                        "MSScan.xsd: SpectrumParamValues has type {ty}"
                    )));
                }
                continue;
            }
            head.push((
                name.clone(),
                Kind::of_xsd(ty).ok_or_else(|| {
                    bad(format!(
                        "MSScan.xsd: field {name} has the unknown type {ty}"
                    ))
                })?,
            ));
        }
        if !scan.iter().any(|(n, _)| n == "SpectrumParamValues") {
            return Err(bad("MSScan.xsd has no SpectrumParamValues".into()));
        }
        let mut tail = Vec::new();
        for (name, ty) in spectrum {
            tail.push((
                name.clone(),
                Kind::of_xsd(&ty).ok_or_else(|| {
                    bad(format!(
                        "MSScan.xsd: field {name} has the unknown type {ty}"
                    ))
                })?,
            ));
        }
        Layout::new(&head, &tail)
    }
}

fn strip_comments(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut rest = s;
    while let Some(a) = rest.find("<!--") {
        out.push_str(&rest[..a]);
        rest = rest[a..].find("-->").map_or("", |e| &rest[a + e + 3..]);
    }
    out + rest
}

impl Layout {
    /// Names of all the fields of the layout, in order.
    pub fn field_names(&self) -> Vec<String> {
        self.fields.iter().map(|f| f.name.clone()).collect()
    }

    /// Names of the fields that `ScanRec::extra` holds, in order.
    pub fn extra_names(&self) -> Vec<String> {
        self.fields
            .iter()
            .filter(|f| !MAPPED.contains(&f.name.as_str()))
            .map(|f| f.name.clone())
            .collect()
    }

    fn at(&self, name: &str) -> Option<&Field> {
        self.fields.iter().find(|f| f.name == name)
    }

    fn int(&self, r: &[u8], name: &str) -> Option<i64> {
        let f = self.at(name)?;
        let b = &r[f.at..f.at + f.kind.size()];
        Some(match f.kind {
            Kind::I8 => b[0] as i8 as i64,
            Kind::I16 => i16::from_le_bytes([b[0], b[1]]) as i64,
            Kind::I32 => i32::from_le_bytes([b[0], b[1], b[2], b[3]]) as i64,
            Kind::I64 => i64::from_le_bytes(b.try_into().ok()?),
            Kind::F32 => f32::from_le_bytes([b[0], b[1], b[2], b[3]]) as i64,
            Kind::F64 => f64::from_le_bytes(b.try_into().ok()?) as i64,
        })
    }

    fn real(&self, r: &[u8], name: &str) -> Option<f64> {
        let f = self.at(name)?;
        let b = &r[f.at..f.at + f.kind.size()];
        Some(match f.kind {
            Kind::F32 => f32::from_le_bytes([b[0], b[1], b[2], b[3]]) as f64,
            Kind::F64 => f64::from_le_bytes(b.try_into().ok()?),
            _ => self.int(r, name)? as f64,
        })
    }
}

fn attr(tag: &str, name: &str) -> Option<String> {
    let k = format!("{name}=\"");
    let a = tag.find(&k)? + k.len();
    Some(tag[a..a + tag[a..].find('"')?].to_string())
}

/// (name, type) of the `xs:element`s inside the `xs:complexType` called `type_name`.
fn elements(xsd: &str, type_name: &str) -> Result<Vec<(String, String)>> {
    let key = format!("name=\"{type_name}\"");
    let from = xsd
        .find(&key)
        .ok_or_else(|| bad(format!("MSScan.xsd has no {type_name}")))?;
    let body = &xsd[from..];
    let body = &body[..body.find("</xs:complexType>").unwrap_or(body.len())];
    let mut out = Vec::new();
    for t in body.split("<xs:element").skip(1) {
        let tag = &t[..t.find('>').unwrap_or(t.len())];
        if let (Some(n), Some(ty)) = (attr(tag, "name"), attr(tag, "type")) {
            out.push((n, ty));
        }
    }
    Ok(out)
}

/// Fields that have a member in `ScanRec`; all the others go to `extra`.
const MAPPED: &[&str] = &[
    "ScanID",
    "ScanMethodID",
    "TimeSegmentID",
    "ScanTime",
    "MSLevel",
    "ScanType",
    "TIC",
    "BasePeakMZ",
    "BasePeakValue",
    "CycleNumber",
    "IonMode",
    "IonPolarity",
    "Fragmentor",
    "CollisionEnergy",
    "MzOfInterest",
    "SpectrumFormatID",
    "SpectrumOffset",
    "ByteCount",
    "UncompressedByteCount",
    "PointCount",
    "MaxY",
];

/// The fields that have a member in `ScanRec` (the others are in `ScanRec::extra`).
pub fn mapped_fields() -> &'static [&'static str] {
    MAPPED
}

/// A scan record. Times are in minutes; `offset`/`bytes` locate the points in `MSPeak.bin`. A field the layout does not have is 0.
#[derive(Debug, Clone, PartialEq)]
pub struct ScanRec {
    pub id: i32,
    pub method_id: i32,
    /// 0-based (`MSTS.xml` numbers its segments from 1).
    pub segment: i32,
    pub time_min: f64,
    pub level: i32,
    pub scan_type: i32,
    pub tic: f64,
    pub base_mz: f64,
    pub base_value: f64,
    pub cycle: i32,
    pub ion_mode: i32,
    pub polarity: i32,
    pub fragmentor: f64,
    pub collision_energy: f64,
    pub mz_of_interest: f64,
    pub format: i32,
    pub offset: u64,
    pub bytes: u32,
    /// `UncompressedByteCount` when the layout has it: different from `bytes` means the points are compressed.
    pub uncompressed: Option<u32>,
    pub points: u32,
    pub max_y: f64,
    /// The fields of the layout that have no member above (`Layout::extra_names`, same order), so that no value of the record is lost.
    pub extra: Vec<f64>,
}

/// One record of the file read by the layout.
pub(super) fn record(layout: &Layout, r: &[u8]) -> ScanRec {
    let i = |name: &str| layout.int(r, name).unwrap_or(0);
    let f = |name: &str| layout.real(r, name).unwrap_or(0.0);
    ScanRec {
        id: i("ScanID") as i32,
        method_id: i("ScanMethodID") as i32,
        segment: i("TimeSegmentID") as i32,
        time_min: f("ScanTime"),
        level: i("MSLevel") as i32,
        scan_type: i("ScanType") as i32,
        tic: f("TIC"),
        base_mz: f("BasePeakMZ"),
        base_value: f("BasePeakValue"),
        cycle: i("CycleNumber") as i32,
        ion_mode: i("IonMode") as i32,
        polarity: i("IonPolarity") as i32,
        fragmentor: f("Fragmentor"),
        collision_energy: f("CollisionEnergy"),
        mz_of_interest: f("MzOfInterest"),
        format: i("SpectrumFormatID") as i32,
        offset: i("SpectrumOffset").max(0) as u64,
        bytes: i("ByteCount").max(0) as u32,
        uncompressed: layout
            .at("UncompressedByteCount")
            .map(|_| i("UncompressedByteCount").max(0) as u32),
        points: i("PointCount").max(0) as u32,
        max_y: f("MaxY"),
        extra: layout
            .fields
            .iter()
            .filter(|x| !MAPPED.contains(&x.name.as_str()))
            .map(|x| f(&x.name))
            .collect(),
    }
}

/// The records of an `MSScan.bin`. `peak_len`: size of `MSPeak.bin`, to check that every spectrum lies inside it. `scans`: how many scans the other files
/// of the folder announce (`MSTS.xml`), only to say in the error what record size the file would have.
pub fn parse(
    bytes: &[u8],
    layout: &Layout,
    peak_len: u64,
    scans: Option<usize>,
) -> Result<Vec<ScanRec>> {
    if bytes.len() < HEADER_MIN {
        return Err(bad(format!("MSScan.bin has {} bytes", bytes.len())));
    }
    let start = i32::from_le_bytes([
        bytes[DATA_OFFSET_AT],
        bytes[DATA_OFFSET_AT + 1],
        bytes[DATA_OFFSET_AT + 2],
        bytes[DATA_OFFSET_AT + 3],
    ]) as usize;
    let n = layout.record_len;
    if start < DATA_OFFSET_AT + 4 || start > bytes.len() || !(bytes.len() - start).is_multiple_of(n)
    {
        let seen = scans
            .filter(|s| *s > 0 && start <= bytes.len())
            .map(|s| {
                format!(
                    ", {s} scans announced: record size {:.2}",
                    (bytes.len() - start) as f64 / s as f64
                )
            })
            .unwrap_or_default();
        return Err(bad(format!(
            "MSScan.bin: {} bytes, records from {start}, a record of {n} bytes does not fit{seen}",
            bytes.len()
        )));
    }
    let mut out = Vec::new();
    out.try_reserve_exact((bytes.len() - start) / n)
        .map_err(|_| Error::memory_budget())?;
    for (k, r) in bytes[start..].chunks_exact(n).enumerate() {
        let rec = record(layout, r);
        if !rec.time_min.is_finite()
            || rec
                .offset
                .checked_add(rec.bytes as u64)
                .is_none_or(|e| e > peak_len)
        {
            return Err(bad(format!(
                "scan record {} points outside MSPeak.bin or has no time",
                k + 1
            )));
        }
        out.push(rec);
    }
    Ok(out)
}

#[cfg(feature = "estesi")]
#[path = "../agilent_ext/scan_lc.rs"]
mod lc;
