//! `data.ms`: the ChemStation file (type bytes `01 32`, «GC / MS Data File») that a MassHunter folder was converted from, and the only data file of an older `.D`.
//! Big-endian. The header holds Pascal strings (file type at 0x04, sample at 0x18, operator at 0x94, date at 0xB2, instrument at 0xD0, method at 0xE4), the number of
//! scans (`u32` at 0x116) and where the first record starts (`u16` at 0x10A, in words, minus 2). A record: `u16` length in words, `u32` time (ms), `u16` offset, `u16`
//! flag, `u16` 0, `u16` n, then n+1 pairs `(u16 m/z×20, u16 abundance)`: the base peak first, then the peaks from the highest m/z down; then `u16` 5, `u16` 4, `u16` 0
//! and `u32` TIC. Abundance: the top 2 bits are an exponent e, the value is `(low 14 bits) << 3e`. The m/z is a whole number of twentieths: coarser than the calibrated
//! values of `MSPeak.bin` (up to 0.05 apart; abundances differ by calibration too), the times and TICs are equal. Checked on three real runs (`tests/agilent_real.rs`).
//! Anything with another record length, count or trailer is an error that names the numbers.

use super::{hashes, info, meta, metadata, method, AgilentD, Files, ScanRec, SCAN_FULL, SCAN_SIM};
use crate::error::{Error, Result};

fn bad(detail: String) -> Error {
    Error::new("err.file.agilent").with("detail", detail)
}

/// A scan of `data.ms`.
#[derive(Debug, Clone, PartialEq)]
pub struct DmScan {
    pub time_min: f64,
    pub tic: f64,
    pub base_mz: f64,
    pub base_value: f64,
    /// Ascending.
    pub mz: Vec<f64>,
    pub y: Vec<f64>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct DataMs {
    /// (offset, text) of the Pascal strings of the header.
    pub strings: Vec<(usize, String)>,
    pub scans: Vec<DmScan>,
    /// Where the records start and end: the bytes after `end` (a table that is not decoded) are declared by `AgilentD::metadata`.
    pub start: usize,
    pub end: usize,
}

const STRING_AT: [(usize, &str); 6] = [
    (0x04, "file type"),
    (0x18, "sample name"),
    (0x94, "operator"),
    (0xB2, "date"),
    (0xD0, "instrument"),
    (0xE4, "method"),
];
const COUNT_AT: usize = 0x116;
const START_AT: usize = 0x10A;
const FIXED: usize = 14 + 10;

/// The name of a Pascal string of the header, as `(label, text)`.
pub fn string_label(offset: usize) -> String {
    STRING_AT
        .iter()
        .find(|s| s.0 == offset)
        .map_or_else(|| format!("string at 0x{offset:X}"), |s| s.1.to_string())
}

pub fn parse(b: &[u8]) -> Result<DataMs> {
    if b.get(..2) != Some(&[0x01, 0x32]) {
        return Err(bad(
            "data.ms: not a ChemStation GC/MS data file (type bytes 01 32)".into(),
        ));
    }
    let n =
        be32(b, COUNT_AT).ok_or_else(|| bad("data.ms is shorter than its header".into()))? as usize;
    let start = (be16(b, START_AT).unwrap_or(0) * 2).saturating_sub(2);
    if n == 0 || start < 0x130 || start >= b.len() {
        return Err(bad(format!("data.ms: {n} scans, records from {start}")));
    }
    let mut strings = Vec::new();
    for (o, _) in STRING_AT {
        let len = b.get(o).copied().unwrap_or(0) as usize;
        if let Some(s) = b
            .get(o + 1..o + 1 + len)
            .filter(|s| len > 0 && s.iter().all(|c| (32..127).contains(c)))
        {
            strings.push((o, String::from_utf8_lossy(s).into_owned()));
        }
    }
    let mut scans = Vec::new();
    scans
        .try_reserve_exact(n.min(1 << 20))
        .map_err(|_| Error::memory_budget())?;
    let mut at = start;
    for k in 0..n {
        let words = be16(b, at)
            .ok_or_else(|| bad(format!("data.ms: record {} lies past the end", k + 1)))?;
        let peaks = be16(b, at + 12).unwrap_or(usize::MAX);
        if peaks > 20_000 || 2 * words != FIXED + 4 * (peaks + 1) || at + 2 * words > b.len() {
            return Err(bad(format!(
                "data.ms: record {} has {words} words for {peaks} peaks",
                k + 1
            )));
        }
        let pair = |i: usize| {
            (
                be16(b, at + 14 + 4 * i).unwrap_or(0),
                be16(b, at + 16 + 4 * i).unwrap_or(0),
            )
        };
        let (bm, bv) = pair(0);
        let (mut mz, mut y) = (Vec::with_capacity(peaks), Vec::with_capacity(peaks));
        for i in (1..=peaks).rev() {
            let (m, a) = pair(i);
            mz.push(m as f64 / 20.0);
            y.push(abundance(a));
        }
        scans.push(DmScan {
            time_min: be32(b, at + 2).unwrap_or(0) as f64 / 60000.0,
            tic: be32(b, at + 2 * words - 4).unwrap_or(0) as f64,
            base_mz: bm as f64 / 20.0,
            base_value: abundance(bv),
            mz,
            y,
        });
        at += 2 * words;
    }
    Ok(DataMs {
        strings,
        scans,
        start,
        end: at,
    })
}

/// The header strings of `data.ms`, and the parts of the file that are not decoded (the header numbers, the table after the records), with size and SHA-1.
pub(super) fn items(d: &DataMs, b: &[u8]) -> Vec<meta::Item> {
    let mut out: Vec<meta::Item> = d
        .strings
        .iter()
        .map(|(o, s)| meta::Item {
            origin: "data.ms".into(),
            name: string_label(*o),
            value: s.clone(),
        })
        .collect();
    out.push(meta::Item {
        origin: "data.ms".into(),
        name: "scans".into(),
        value: d.scans.len().to_string(),
    });
    out.extend(meta::collect(&[
        ("data.ms#header", &b[..d.start]),
        ("data.ms#after-records", b.get(d.end..).unwrap_or(&[])),
    ]));
    out
}

fn be16(b: &[u8], o: usize) -> Option<usize> {
    Some(u16::from_be_bytes([*b.get(o)?, *b.get(o + 1)?]) as usize)
}

fn be32(b: &[u8], o: usize) -> Option<u32> {
    Some(u32::from_be_bytes([
        *b.get(o)?,
        *b.get(o + 1)?,
        *b.get(o + 2)?,
        *b.get(o + 3)?,
    ]))
}

fn abundance(v: usize) -> f64 {
    ((v & 0x3fff) << (3 * (v >> 14))) as f64
}

impl AgilentD {
    /// A run that has only `data.ms` (and the side files): the scans come from it, with the m/z in twentieths as the file holds them.
    pub(super) fn open_data_ms(name: &str, f: &Files, bytes: &[u8]) -> Result<AgilentD> {
        let d = parse(bytes)?;
        let method = f.acqmeth.map(method::parse);
        let scan_type = if method
            .as_ref()
            .is_some_and(|m| m.mode.eq_ignore_ascii_case("SIM"))
        {
            SCAN_SIM
        } else {
            SCAN_FULL
        };
        let mut inline = Vec::with_capacity(d.scans.len());
        let mut records = Vec::with_capacity(d.scans.len());
        for (k, s) in d.scans.iter().enumerate() {
            let n = s.mz.len() as u32;
            records.push(ScanRec {
                id: k as i32 + 1,
                method_id: 1,
                segment: 0,
                time_min: s.time_min,
                level: 1,
                scan_type,
                tic: s.tic,
                base_mz: s.base_mz,
                base_value: s.base_value,
                cycle: k as i32 + 1,
                ion_mode: 0,
                polarity: 0,
                fragmentor: 0.0,
                collision_energy: 0.0,
                mz_of_interest: 0.0,
                format: 1,
                offset: 0,
                bytes: 16 * n,
                uncompressed: None,
                points: n,
                max_y: s.y.iter().cloned().fold(0.0, f64::max),
                extra: Vec::new(),
            });
            inline.push((s.mz.clone(), s.y.clone()));
        }
        let mut metadata = metadata(f);
        metadata.extend(items(&d, bytes));
        Ok(AgilentD {
            name: name.to_string(),
            records,
            segments: f.ms_ts.map(info::segments).unwrap_or_default(),
            device: f.devices.and_then(|d| info::devices(d).into_iter().next()),
            contents: f.contents.and_then(info::contents),
            tree: None,
            method,
            metadata,
            extra_names: Vec::new(),
            file_sha1: hashes(f),
            inline: Some(inline),
            anonymous: false,
        })
    }
}
