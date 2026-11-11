//! `_FUNCnnn.IDX`: one record per scan, in two layouts. Variant A (22 bytes) holds the offset in the DAT as `u32`; variant B (30 bytes) holds it as `u64` at byte 22. In both, bytes 4-7
//! hold the number of records of the scan in their low 22 bits and a mark in the top byte, then the TIC (`f32`) and the retention time in minutes (`f32`).

use super::folder::{damage, f32_at, read_error, u32_at};
use mzlab_core::error::Result;
use mzlab_core::readers::Limits;

const A: usize = 22;
const B: usize = 30;
/// The top byte of the word at byte 4 in every record of the families seen: 0x18 continuum scans stored uncalibrated, 0x08 reference (lock mass) and MRM functions,
/// 0x4C centroid scans the acquisition stored already calibrated. Only whole bytes were seen, no single bit is decoded.
const MARKS: [u8; 3] = [0x18, 0x4c, 0x08];
/// The mark of the scans whose m/z the acquisition already calibrated: the polynomial of `_HEADER.TXT` is not applied again.
pub(super) const CALIBRATED: u8 = 0x4c;
/// Low bits of the word at byte 4: the records of the scan (the bits above are flags).
const COUNT: u32 = 0x003F_FFFF;

#[derive(Debug, Clone, Copy)]
pub(super) struct Row {
    pub(super) offset: u64,
    /// Records of the scan: points of a scan, channels of a SIR or MRM scan.
    pub(super) count: u32,
    /// The top byte of the word at byte 4 (`MARKS`).
    pub(super) mark: u8,
    pub(super) tic: f32,
    /// Minutes.
    pub(super) rt: f32,
}

/// Whether `b` is variant A: only when its size fits and, if it fits both, when every record carries a known mark and the offsets never go back.
fn variant_a(b: &[u8]) -> bool {
    match (b.len().is_multiple_of(A), b.len().is_multiple_of(B)) {
        (true, false) => true,
        (false, _) => false,
        (true, true) => {
            let mut before = 0u32;
            b.chunks_exact(A).all(|r| {
                let o = u32_at(r, 0);
                let ok = MARKS.contains(&r[7]) && o >= before;
                before = o;
                ok
            })
        }
    }
}

/// The rows of `b`; `None` when its size is no whole number of rows of either variant (an index cut inside a row).
pub(super) fn read(b: &[u8], limits: &Limits) -> Result<Option<Vec<Row>>> {
    if b.is_empty() {
        return Ok(Some(Vec::new()));
    }
    if !b.len().is_multiple_of(A) && !b.len().is_multiple_of(B) {
        return Ok(None);
    }
    let a = variant_a(b);
    let step = if a { A } else { B };
    limits.ok_entries((b.len() / step) as u64)?;
    Ok(Some(
        b.chunks_exact(step)
            .map(|r| Row {
                offset: if a {
                    u32_at(r, 0) as u64
                } else {
                    u32_at(r, 22) as u64 | (u32_at(r, 26) as u64) << 32
                },
                count: u32_at(r, 4) & COUNT,
                mark: r[7],
                tic: f32_at(r, 8),
                rt: f32_at(r, 12),
            })
            .collect(),
    ))
}

/// A notice for each mark of `rows` that is none of `MARKS`, with how many scans carry it: their m/z keep the polynomial, as the 0x18 scans.
pub(super) fn mark_notes(name: &str, k: u32, rows: &[Row]) -> Vec<(String, String)> {
    let mut v: Vec<(u8, usize)> = Vec::new();
    for r in rows.iter().filter(|r| !MARKS.contains(&r.mark)) {
        match v.iter_mut().find(|x| x.0 == r.mark) {
            Some(x) => x.1 += 1,
            None => v.push((r.mark, 1)),
        }
    }
    v.into_iter()
        .map(|(m, n)| {
            let a = format!("{name}: function {k}: unknown calibration flag 0x{m:02X} in {n} scans, polynomial applied as for 0x18");
            ("meta.nonDecodificato".to_string(), a)
        })
        .collect()
}

/// Bytes per record of a function: the first scan whose extent (up to the offset of the next scan, or up to the end of the DAT for the last) is a whole number of records.
pub(super) fn width(rows: &[Row], dat_len: u64) -> Option<u64> {
    (0..rows.len()).find_map(|k| {
        let r = rows[k];
        let fine = rows.get(k + 1).map_or(dat_len, |n| n.offset);
        let d = fine.checked_sub(r.offset)?;
        let c = r.count as u64;
        // `then`, not `then_some`: the division must not run for a scan of 0 records
        (c > 0 && d > 0 && d.is_multiple_of(c)).then(|| d / c)
    })
}

/// (offset, length) of scan `k` given the record width, inside a DAT of `dat_len` bytes.
pub(super) fn extension(rows: &[Row], k: usize, width: u64, dat_len: u64) -> Result<(u64, u64)> {
    let r = rows.get(k).ok_or_else(|| read_error("scan out of range"))?;
    let len = (r.count as u64)
        .checked_mul(width)
        .ok_or_else(|| damage("scan size"))?;
    match r.offset.checked_add(len) {
        Some(fine) if fine <= dat_len => Ok((r.offset, len)),
        _ => Err(damage("scan past the end of the DAT")),
    }
}
