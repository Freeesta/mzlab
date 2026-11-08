use crate::error::Result;
use crate::reader::BinaryReader;
use std::io::{Read, Seek};

/// One entry in the scan index array (v66: 92 bytes).
#[derive(Debug)]
pub struct ScanIndexEntry {
    pub index: u32,
    pub scan_event: u16,
    pub scan_segment: u16,
    pub data_size: u32,
    pub start_time: f64,
    pub total_current: f64,
    pub base_intensity: f64,
    pub base_mz: f64,
    pub low_mz: f64,
    pub high_mz: f64,
    /// 64-bit offset relative to data stream start.
    pub offset: u64,
}

impl ScanIndexEntry {
    /// Minimum on-disk size in bytes of one entry for the given file version.
    /// Used to bound `Vec::with_capacity(num_scans)` against the file's
    /// actual remaining size before allocating.
    pub(crate) fn min_size_for_version(version: u32) -> u64 {
        // offset_32(4) + index(4) + scan_event(2) + scan_segment(2) + next(4)
        // + unk(4) + data_size(4) + 6 * f64(8) = 72 bytes, common to every
        // version (the offset_32 field is reused directly pre-v64).
        const BASE: u64 = 72;
        if version >= 66 {
            BASE + 8 /* u64 offset instead of offset_32 */ + 8 /* 2 trailing u32s */
        } else if version >= 64 {
            BASE + 8 /* u64 offset instead of offset_32 */
        } else {
            BASE
        }
    }

    /// One entry from one read of its bytes: a field-by-field read goes through the source once per field (14 calls per scan).
    pub(crate) fn read<R: Read + Seek>(r: &mut BinaryReader<R>, version: u32) -> Result<Self> {
        let mut b = [0u8; 88];
        let b = &mut b[..Self::min_size_for_version(version) as usize];
        r.read_bytes_into(b)?;
        let u16_at = |i: usize| u16::from_le_bytes([b[i], b[i + 1]]);
        let u32_at = |i: usize| u32::from_le_bytes([b[i], b[i + 1], b[i + 2], b[i + 3]]);
        let f64_at = |i: usize| {
            let mut w = [0u8; 8];
            w.copy_from_slice(&b[i..i + 8]);
            f64::from_le_bytes(w)
        };
        // offset_32 (0), index (4), scan_event (8), scan_segment (10), next (12), unk (16), data_size (20), 6 f64 (24..72),
        // then the u64 offset from v64 on and two unknown u32s in v66
        let index = u32_at(4);
        let scan_event = u16_at(8);
        let scan_segment = u16_at(10);
        let data_size = u32_at(20);
        let start_time = f64_at(24);
        let total_current = f64_at(32);
        let base_intensity = f64_at(40);
        let base_mz = f64_at(48);
        let low_mz = f64_at(56);
        let high_mz = f64_at(64);
        let offset = if version >= 64 {
            u64::from(u32_at(72)) | (u64::from(u32_at(76)) << 32)
        } else {
            u64::from(u32_at(0))
        };

        Ok(Self {
            index,
            scan_event,
            scan_segment,
            data_size,
            start_time,
            total_current,
            base_intensity,
            base_mz,
            low_mz,
            high_mz,
            offset,
        })
    }
}
