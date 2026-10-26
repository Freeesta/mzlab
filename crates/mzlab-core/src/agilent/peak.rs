//! `AcqData/MSPeak.bin`: the points of every scan, one block per scan at the offset its record gives. Format 1: `points` m/z values then `points` intensities,
//! all little-endian, either `f64` (16 bytes per point: 5977 GC/MS) or `f32` (8 bytes per point: the centroids of the GC/TQ 7000D). The file starts with 68 bytes of header that no record points into.

use super::scan::ScanRec;
use crate::error::{Error, Result};
use crate::idxread::ReadAt;

fn bad(detail: String) -> Error {
    Error::new("err.file.agilent").with("detail", detail)
}

/// Bytes per point of a GC block (`m/z` and intensity of the same width), or None when the size matches neither layout.
pub fn gc_width(rec: &ScanRec) -> Option<usize> {
    let n = rec.points as u64;
    match rec.bytes as u64 {
        b if b == n * 16 => Some(8),
        b if b == n * 8 => Some(4),
        _ => None,
    }
}

/// The m/z values and intensities of a scan, m/z ascending.
pub fn read_points<R: ReadAt>(peak: &mut R, rec: &ScanRec) -> Result<(Vec<f64>, Vec<f64>)> {
    let n = rec.points as usize;
    if rec.uncompressed.is_some_and(|u| u != rec.bytes) {
        return Err(bad(format!(
            "scan {}: {} bytes stored for {} uncompressed (compressed spectra are not read)",
            rec.id,
            rec.bytes,
            rec.uncompressed.unwrap_or(0)
        )));
    }
    let width = match gc_width(rec) {
        Some(w) if rec.format == 1 => w,
        _ => {
            return Err(bad(format!(
                "scan {}: SpectrumFormatID {} with {} bytes for {} points is not a known layout",
                rec.id, rec.format, rec.bytes, rec.points
            )))
        }
    };
    let mut raw = vec![0u8; n * 2 * width];
    peak.read_at(rec.offset, &mut raw)?;
    let f = |k: usize| {
        if width == 8 {
            f64::from_le_bytes(raw[k * 8..k * 8 + 8].try_into().unwrap())
        } else {
            f32::from_le_bytes(raw[k * 4..k * 4 + 4].try_into().unwrap()) as f64
        }
    };
    let mz: Vec<f64> = (0..n).map(f).collect();
    let it: Vec<f64> = (n..2 * n).map(f).collect();
    if mz.iter().chain(&it).any(|v| !v.is_finite()) {
        return Err(bad(format!("scan {}: a value is not a number", rec.id)));
    }
    if mz.windows(2).all(|w| w[0] <= w[1]) {
        return Ok((mz, it));
    }
    let mut ord: Vec<usize> = (0..n).collect();
    crate::sortc::slice_by(&mut ord, |&a, &b| mz[a].total_cmp(&mz[b]));
    Ok((
        ord.iter().map(|&i| mz[i]).collect(),
        ord.iter().map(|&i| it[i]).collect(),
    ))
}
