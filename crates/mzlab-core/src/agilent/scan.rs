//! `AcqData/MSScan.bin`: a header of 160 bytes and one fixed record of 184 bytes per scan (the layout of `MSScan.xsd`, which the data folder carries).
//! Only what the three GC/MS runs show is read; any other size or format is an error that names the numbers, never a guess.

use crate::error::{Error, Result};

/// Bytes before the first record (the same value is stored at `DATA_OFFSET_AT`).
pub const HEADER_LEN: usize = 160;
/// One scan record: the fields of `ScanRecordType` with a single `SpectrumParamValues`, packed without padding.
pub const RECORD_LEN: usize = 184;
const DATA_OFFSET_AT: usize = 88;

/// A scan record. Times are in minutes; `offset`/`bytes` locate the points in `MSPeak.bin`.
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
    pub points: u32,
    pub max_y: f64,
}

fn bad(detail: String) -> Error {
    Error::new("err.file.agilent").with("detail", detail)
}

struct At<'a>(&'a [u8]);

impl At<'_> {
    fn i32(&self, o: usize) -> i32 {
        i32::from_le_bytes([self.0[o], self.0[o + 1], self.0[o + 2], self.0[o + 3]])
    }
    fn f64(&self, o: usize) -> f64 {
        let mut b = [0u8; 8];
        b.copy_from_slice(&self.0[o..o + 8]);
        f64::from_le_bytes(b)
    }
    fn i64(&self, o: usize) -> i64 {
        let mut b = [0u8; 8];
        b.copy_from_slice(&self.0[o..o + 8]);
        i64::from_le_bytes(b)
    }
}

/// The records of an `MSScan.bin`. `peak_len`: size of `MSPeak.bin`, to check that every spectrum lies inside it.
pub fn parse(bytes: &[u8], peak_len: u64) -> Result<Vec<ScanRec>> {
    if bytes.len() < HEADER_LEN {
        return Err(bad(format!("MSScan.bin has {} bytes", bytes.len())));
    }
    let start = At(bytes).i32(DATA_OFFSET_AT) as usize;
    if start < DATA_OFFSET_AT + 4 || start > bytes.len() || !(bytes.len() - start).is_multiple_of(RECORD_LEN) {
        return Err(bad(format!("MSScan.bin: {} bytes, records from {start}, record size {RECORD_LEN} does not fit", bytes.len())));
    }
    let mut out = Vec::with_capacity((bytes.len() - start) / RECORD_LEN);
    for (k, r) in bytes[start..].chunks_exact(RECORD_LEN).enumerate() {
        let a = At(r);
        let rec = ScanRec {
            id: a.i32(0),
            method_id: a.i32(4),
            segment: a.i32(8),
            time_min: a.f64(12),
            level: a.i32(20),
            scan_type: a.i32(24),
            tic: a.f64(28),
            base_mz: a.f64(36),
            base_value: a.f64(44),
            cycle: a.i32(52),
            ion_mode: a.i32(60),
            polarity: a.i32(64),
            fragmentor: a.f64(68),
            collision_energy: a.f64(76),
            mz_of_interest: a.f64(84),
            format: a.i32(132),
            offset: a.i64(136).max(0) as u64,
            bytes: a.i32(144).max(0) as u32,
            points: a.i32(148).max(0) as u32,
            max_y: a.f64(160),
        };
        if !rec.time_min.is_finite() || rec.offset.checked_add(rec.bytes as u64).is_none_or(|e| e > peak_len) {
            return Err(bad(format!("scan record {} points outside MSPeak.bin or has no time", k + 1)));
        }
        out.push(rec);
    }
    Ok(out)
}
