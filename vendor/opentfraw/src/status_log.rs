//! The instrument status log: the temperatures, voltages, vacuum and pump values the instrument reports while it acquires.
//!
//! Where the run header points (`inst_log_addr`) the records start; in a v66 file the schema (a [`GenericDataHeader`]) ends exactly there, and
//! each record is a `f32` time in minutes followed by the fields of the schema. [`read_before`] finds that schema; the records are not
//! the per-scan ones of [`crate::RawFileReader::inst_log`], which this module leaves as they were.

use crate::error::Result;
use crate::generic_data::{GenericDataHeader, GenericRecord};
use crate::reader::BinaryReader;
use std::io::{Cursor, Read, Seek};
use std::sync::{Arc, OnceLock};

/// How far before the first record the schema is looked for.
const WINDOW: u64 = 64 * 1024;

/// The status log of a file. A log read from the file keeps the bytes of its fixed-size records and decodes them on the first call of [`Self::records`] or
/// [`Self::times_min`], so opening a file does not allocate a string for every field of every record.
#[derive(Debug)]
pub struct StatusLog {
    pub header: GenericDataHeader,
    /// Bytes of the records not decoded yet (`count` of `size` bytes: the time as `f32`, then the fields of the schema); empty once decoded.
    bytes: Vec<u8>,
    count: usize,
    decoded: OnceLock<Decoded>,
}

#[derive(Debug)]
struct Decoded {
    times_min: Vec<f32>,
    records: Arc<[GenericRecord]>,
}

impl StatusLog {
    /// A log whose records are already decoded. `times_min` is empty when the layout of the file has no time before the record.
    pub fn new(
        header: GenericDataHeader,
        times_min: Vec<f32>,
        records: Arc<[GenericRecord]>,
    ) -> Self {
        let decoded = OnceLock::new();
        let _ = decoded.set(Decoded { times_min, records });
        Self {
            header,
            bytes: Vec::new(),
            count: 0,
            decoded,
        }
    }

    fn decoded(&self) -> &Decoded {
        self.decoded.get_or_init(|| {
            let size = self.header.fixed_record_size() + 4;
            let (mut times_min, mut records) = (Vec::new(), Vec::new());
            for chunk in self.bytes.chunks_exact(size).take(self.count) {
                let mut r = BinaryReader::new(Cursor::new(&chunk[4..]));
                match GenericRecord::read(&mut r, &self.header) {
                    Ok(rec) => {
                        times_min
                            .push(f32::from_le_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]));
                        records.push(rec);
                    }
                    // the strings were checked when the log was read, so this is not reached
                    Err(_) => break,
                }
            }
            Decoded {
                times_min,
                records: records.into(),
            }
        })
    }

    /// Time of each record in minutes; empty when the layout of the file has no time before the record.
    pub fn times_min(&self) -> &[f32] {
        &self.decoded().times_min
    }

    /// The records, shared with [`crate::RawFileReader::inst_log`] when the per-scan records are the log.
    pub fn records(&self) -> &[GenericRecord] {
        &self.decoded().records
    }
}

/// One numeric channel of the status log over time (`Spray Voltage` in `V`).
#[derive(Debug, Clone)]
pub struct StatusSeries {
    pub name: String,
    /// Unit as the label gives it (`V`, `°C`, `mbar`); empty when the label has none.
    pub unit: String,
    pub time_min: Vec<f64>,
    pub values: Vec<f64>,
}

/// A label as `(name, unit)`: `Spray Voltage (V):` → `("Spray Voltage", "V")`.
pub fn split_label(label: &str) -> (String, String) {
    let l = label.trim().trim_end_matches(':').trim();
    if let Some(body) = l.strip_suffix(')') {
        if let Some(open) = body.rfind('(') {
            let (name, unit) = (body[..open].trim(), body[open + 1..].trim());
            if !name.is_empty() && !unit.is_empty() {
                return (name.to_string(), unit.to_string());
            }
        }
    }
    (l.to_string(), String::new())
}

impl StatusLog {
    /// Every numeric channel as a series; nothing when the records carry no time.
    pub fn series(&self) -> Vec<StatusSeries> {
        if self.times_min().len() != self.records().len() {
            return Vec::new();
        }
        let mut out = Vec::new();
        for (i, d) in self.header.fields.iter().enumerate() {
            let pairs: Vec<(f64, f64)> = self
                .records()
                .iter()
                .zip(self.times_min())
                .filter_map(|(r, t)| Some((f64::from(*t), r.values.get(i)?.1.as_f64()?)))
                .filter(|(t, v)| t.is_finite() && v.is_finite())
                .collect();
            if pairs.is_empty() || d.label.trim().is_empty() {
                continue;
            }
            let (name, unit) = split_label(&d.label);
            out.push(StatusSeries {
                name,
                unit,
                time_min: pairs.iter().map(|p| p.0).collect(),
                values: pairs.iter().map(|p| p.1).collect(),
            });
        }
        out
    }
}

/// The schema that ends exactly at `end` inside `buf` (the bytes before the first record); `want` is the size a record must have.
fn schema_ending_at(buf: &[u8], want: Option<usize>) -> Option<GenericDataHeader> {
    for strict in [true, false] {
        for offset in (0..buf.len().saturating_sub(4)).step_by(2) {
            let n = u32::from_le_bytes(buf.get(offset..offset + 4)?.try_into().ok()?);
            if !(2..=500).contains(&n) {
                continue;
            }
            let mut c = BinaryReader::new(Cursor::new(&buf[offset..]));
            let Ok(Some(h)) = GenericDataHeader::try_read(&mut c) else {
                continue;
            };
            let fits = want.map_or(true, |w| h.fixed_record_size() + 4 == w);
            if c.position() == (buf.len() - offset) as u64 && (fits || !strict) {
                return Some(h);
            }
        }
        want?;
    }
    None
}

/// The status log whose schema ends at `addr` and whose `count` records follow; `region` is the length of the records when the next block's address says it.
/// `None` when no schema ends there or the records do not fit: the log is then not decoded, which is not an error of the file.
pub(crate) fn read_before<R: Read + Seek>(
    r: &mut BinaryReader<R>,
    addr: u64,
    count: u32,
    region: u64,
) -> Result<Option<StatusLog>> {
    if count == 0 || addr == 0 {
        return Ok(None);
    }
    let from = addr.saturating_sub(WINDOW);
    r.seek_to(from)?;
    let buf = r.read_bytes((addr - from) as usize)?;
    let want = (region > 0 && region % u64::from(count) == 0)
        .then(|| (region / u64::from(count)) as usize);
    let Some(header) = schema_ending_at(&buf, want) else {
        return Ok(None);
    };
    let size = header.fixed_record_size() as u64 + 4;
    r.seek_to(addr)?;
    r.check_count(u64::from(count), size)?;
    let bytes = r.read_bytes((u64::from(count) * size) as usize)?;
    // The only way decoding fails beyond the end of the bytes is a wide string that is not valid UTF-16: look for it here so a log that does not decode is
    // still "not decoded" at open, as when the records were read one by one.
    let wide: Vec<(usize, usize)> = header_wide_fields(&header);
    for rec in bytes.chunks_exact(size as usize) {
        for &(at, units) in &wide {
            if !wide_string_is_valid(&rec[4 + at..4 + at + units * 2]) {
                return Err(crate::error::Error::InvalidUtf16(addr));
            }
        }
    }
    Ok(Some(StatusLog {
        header,
        bytes,
        count: count as usize,
        decoded: OnceLock::new(),
    }))
}

/// `(byte offset in the fields area, UTF-16 units)` of each wide string field of a record.
fn header_wide_fields(header: &GenericDataHeader) -> Vec<(usize, usize)> {
    let mut at = 0;
    let mut out = Vec::new();
    for f in &header.fields {
        if f.field_type == crate::types::GenericType::WideString && f.length > 0 {
            out.push((at, f.length as usize));
        }
        at += GenericDataHeader::field_size(f);
    }
    out
}

/// True if the string stored in `bytes` (UTF-16LE, up to the first null unit) decodes.
fn wide_string_is_valid(bytes: &[u8]) -> bool {
    let units = bytes
        .chunks_exact(2)
        .map(|c| u16::from_le_bytes([c[0], c[1]]))
        .take_while(|&u| u != 0);
    char::decode_utf16(units).all(|c| c.is_ok())
}

/// [`read_before`] on any seekable source (the file, or bytes built by a test).
pub fn decode<R: Read + Seek>(source: R, addr: u64, count: u32, region: u64) -> Option<StatusLog> {
    read_before(&mut BinaryReader::new(source), addr, count, region)
        .ok()
        .flatten()
}
