//! The instrument status log: the temperatures, voltages, vacuum and pump values the instrument reports while it acquires.
//!
//! Where the run header points (`inst_log_addr`) the records start; in a v66 file the schema (a [`GenericDataHeader`]) ends exactly there, and
//! each record is a `f32` time in minutes followed by the fields of the schema. [`read_before`] finds that schema; the records are not
//! the per-scan ones of [`crate::RawFileReader::inst_log`], which this module leaves as they were.

use crate::error::Result;
use crate::generic_data::{GenericDataHeader, GenericRecord};
use crate::reader::BinaryReader;
use std::io::{Cursor, Read, Seek};

/// How far before the first record the schema is looked for.
const WINDOW: u64 = 64 * 1024;

/// The decoded status log of a file.
#[derive(Debug)]
pub struct StatusLog {
    pub header: GenericDataHeader,
    /// Time of each record in minutes; empty when the layout of the file has no time before the record.
    pub times_min: Vec<f32>,
    pub records: Vec<GenericRecord>,
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
        if self.times_min.len() != self.records.len() {
            return Vec::new();
        }
        let mut out = Vec::new();
        for (i, d) in self.header.fields.iter().enumerate() {
            let pairs: Vec<(f64, f64)> = self
                .records
                .iter()
                .zip(&self.times_min)
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
    let (mut times_min, mut records) = (Vec::new(), Vec::new());
    for _ in 0..count {
        times_min.push(r.read_f32()?);
        records.push(GenericRecord::read(r, &header)?);
    }
    Ok(Some(StatusLog {
        header,
        times_min,
        records,
    }))
}

/// [`read_before`] on any seekable source (the file, or bytes built by a test).
pub fn decode<R: Read + Seek>(source: R, addr: u64, count: u32, region: u64) -> Option<StatusLog> {
    read_before(&mut BinaryReader::new(source), addr, count, region)
        .ok()
        .flatten()
}
