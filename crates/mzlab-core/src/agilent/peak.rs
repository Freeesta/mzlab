//! `AcqData/MSPeak.bin`: the points of every scan, one block per scan at the offset its record gives. Format 1 (the only one seen): `points` m/z values
//! then `points` intensities, all little-endian `f64` (16 bytes per point). The file starts with 68 bytes of header that no record points into.

use super::scan::ScanRec;
use crate::error::{Error, Result};
use crate::idxread::ReadAt;

fn bad(detail: String) -> Error {
    Error::new("err.file.agilent").with("detail", detail)
}

/// The m/z values and intensities of a scan, m/z ascending.
pub fn read_points<R: ReadAt>(peak: &mut R, rec: &ScanRec) -> Result<(Vec<f64>, Vec<f64>)> {
    let n = rec.points as usize;
    if rec.format != 1 || rec.bytes as usize != n * 16 {
        return Err(bad(format!("scan {}: SpectrumFormatID {} with {} bytes for {} points is not a known layout", rec.id, rec.format, rec.bytes, rec.points)));
    }
    let mut raw = vec![0u8; n * 16];
    peak.read_at(rec.offset, &mut raw)?;
    let f = |k: usize| f64::from_le_bytes([raw[k], raw[k + 1], raw[k + 2], raw[k + 3], raw[k + 4], raw[k + 5], raw[k + 6], raw[k + 7]]);
    let mz: Vec<f64> = (0..n).map(|i| f(i * 8)).collect();
    let it: Vec<f64> = (0..n).map(|i| f((n + i) * 8)).collect();
    if mz.iter().chain(&it).any(|v| !v.is_finite()) {
        return Err(bad(format!("scan {}: a value is not a number", rec.id)));
    }
    if mz.windows(2).all(|w| w[0] <= w[1]) {
        return Ok((mz, it));
    }
    let mut ord: Vec<usize> = (0..n).collect();
    ord.sort_by(|&a, &b| mz[a].total_cmp(&mz[b]));
    Ok((ord.iter().map(|&i| mz[i]).collect(), ord.iter().map(|&i| it[i]).collect()))
}
