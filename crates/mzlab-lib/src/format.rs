//! Constants and fixed-size records of the `.mzlib` v1 format (the byte-by-byte specification is docs/agenti/librerie.md).

use crate::util::{crc32c, le_f32, le_f64, le_u16, le_u32, le_u64};

pub const MAGIC: [u8; 4] = *b"MZLB";
pub const VERSION: u16 = 1;
/// A reader older than this cannot read the file.
pub const MIN_READER: u16 = 1;
pub const HEADER_LEN: usize = 36;
pub const SECTION_ENTRY_LEN: usize = 32;
pub const MAX_SECTIONS: u32 = 64;
/// Where the first section starts (the header and the section table are written last).
pub const HEAD_RESERVED: u64 = 256;

pub const SEC_SPECTRA: u32 = 1;
pub const SEC_PEAKS: u32 = 2;
pub const SEC_META: u32 = 3;
pub const SEC_FRAG: u32 = 4;
pub const SEC_NL: u32 = 5;

pub const SPECTRA_PER_PEAK_CHUNK: u32 = 4096;
pub const RECORDS_PER_META_CHUNK: u32 = 1024;
pub const RECORDS_PER_TABLE_CHUNK: u32 = 4096;
pub const SPEC_REC_LEN: usize = 32;
pub const CHUNK_ENTRY_LEN: usize = 40;
pub const METHOD_RAW: u8 = 0;
pub const METHOD_ZSTD: u8 = 1;
/// No chunk may declare more than this once decoded.
pub const MAX_CHUNK_RAW: u32 = 64 << 20;
/// Flash index pages: a fixed 64 KB slot holding `ENTRIES_PER_PAGE` entries of 10 bytes (m/z `u32`, spectrum `u32`, intensity `u16`).
pub const PAGE_BYTES: usize = 65536;
pub const ENTRIES_PER_PAGE: u32 = 6553;
pub const ENTRY_BYTES: usize = 10;
pub const PAGE_DIR_ENTRY_LEN: usize = 12;
pub const MAX_SPECTRA: u32 = 1 << 31;
/// The largest value of an intensity in the peak and index sections.
pub const INT_MAX: f64 = 65535.0;

pub const FLAG_FEW_PEAKS: u16 = 1;

/// Order of spectra in the table: polarity first (negative, unknown, positive), then precursor m/z.
pub fn pol_key(pol: i8) -> u8 {
    match pol {
        p if p < 0 => 0,
        0 => 1,
        _ => 2,
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SpecRec {
    pub prec: f64,
    pub first_peak: u32,
    pub n_peaks: u16,
    pub pol: i8,
    pub licence: u8,
    pub flags: u16,
    pub orig: u32,
    /// The highest entropy-weighted intensity: the index intensities are `u16` fractions of it.
    pub scale: f32,
}

impl SpecRec {
    pub fn to_bytes(&self) -> [u8; SPEC_REC_LEN] {
        let mut b = [0u8; SPEC_REC_LEN];
        b[0..8].copy_from_slice(&self.prec.to_le_bytes());
        b[8..12].copy_from_slice(&self.first_peak.to_le_bytes());
        b[12..14].copy_from_slice(&self.n_peaks.to_le_bytes());
        b[14] = self.pol as u8;
        b[15] = self.licence;
        b[16..18].copy_from_slice(&self.flags.to_le_bytes());
        b[20..24].copy_from_slice(&self.orig.to_le_bytes());
        b[24..28].copy_from_slice(&self.scale.to_bits().to_le_bytes());
        b
    }

    pub fn from_bytes(b: &[u8]) -> SpecRec {
        SpecRec {
            prec: le_f64(b, 0),
            first_peak: le_u32(b, 8),
            n_peaks: le_u16(b, 12),
            pol: b[14] as i8,
            licence: b[15],
            flags: le_u16(b, 16),
            orig: le_u32(b, 20),
            scale: le_f32(b, 24),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Header {
    pub version: u16,
    pub min_reader: u16,
    pub n_spectra: u32,
    pub n_sections: u32,
    pub n_peaks: u64,
    pub flags: u32,
}

impl Header {
    pub fn to_bytes(&self) -> [u8; HEADER_LEN] {
        let mut b = [0u8; HEADER_LEN];
        b[0..4].copy_from_slice(&MAGIC);
        b[4..6].copy_from_slice(&self.version.to_le_bytes());
        b[6..8].copy_from_slice(&self.min_reader.to_le_bytes());
        b[8..12].copy_from_slice(&self.n_spectra.to_le_bytes());
        b[12..16].copy_from_slice(&self.n_sections.to_le_bytes());
        b[16..24].copy_from_slice(&self.n_peaks.to_le_bytes());
        b[24..28].copy_from_slice(&self.flags.to_le_bytes());
        let c = crc32c(&b[..32]);
        b[32..36].copy_from_slice(&c.to_le_bytes());
        b
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SectionEntry {
    pub kind: u32,
    pub offset: u64,
    pub length: u64,
    /// CRC32C of the section's directory (the part before the data: counts and one entry per chunk or page, each with the CRC of its data).
    pub dir_len: u32,
    pub crc: u32,
}

impl SectionEntry {
    pub fn to_bytes(&self) -> [u8; SECTION_ENTRY_LEN] {
        let mut b = [0u8; SECTION_ENTRY_LEN];
        b[0..4].copy_from_slice(&self.kind.to_le_bytes());
        b[8..16].copy_from_slice(&self.offset.to_le_bytes());
        b[16..24].copy_from_slice(&self.length.to_le_bytes());
        b[24..28].copy_from_slice(&self.dir_len.to_le_bytes());
        b[28..32].copy_from_slice(&self.crc.to_le_bytes());
        b
    }

    pub fn from_bytes(b: &[u8]) -> SectionEntry {
        SectionEntry {
            kind: le_u32(b, 0),
            offset: le_u64(b, 8),
            length: le_u64(b, 16),
            dir_len: le_u32(b, 24),
            crc: le_u32(b, 28),
        }
    }
}

/// One entry of a chunked section's directory.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ChunkEntry {
    /// From the start of the section.
    pub offset: u64,
    /// Key of the first record (table section only): precursor m/z and polarity key.
    pub first_prec: f64,
    pub stored: u32,
    pub raw: u32,
    pub first: u32,
    pub crc: u32,
    pub method: u8,
    pub first_pol_key: u8,
}

impl ChunkEntry {
    pub fn to_bytes(&self) -> [u8; CHUNK_ENTRY_LEN] {
        let mut b = [0u8; CHUNK_ENTRY_LEN];
        b[0..8].copy_from_slice(&self.offset.to_le_bytes());
        b[8..16].copy_from_slice(&self.first_prec.to_le_bytes());
        b[16..20].copy_from_slice(&self.stored.to_le_bytes());
        b[20..24].copy_from_slice(&self.raw.to_le_bytes());
        b[24..28].copy_from_slice(&self.first.to_le_bytes());
        b[28..32].copy_from_slice(&self.crc.to_le_bytes());
        b[32] = self.method;
        b[33] = self.first_pol_key;
        b
    }

    pub fn from_bytes(b: &[u8]) -> ChunkEntry {
        ChunkEntry {
            offset: le_u64(b, 0),
            first_prec: le_f64(b, 8),
            stored: le_u32(b, 16),
            raw: le_u32(b, 20),
            first: le_u32(b, 24),
            crc: le_u32(b, 28),
            method: b[32],
            first_pol_key: b[33],
        }
    }
}

pub fn align8(x: u64) -> u64 {
    (x + 7) & !7
}

pub fn chunk_dir_len(n_chunks: u32) -> u64 {
    8 + n_chunks as u64 * CHUNK_ENTRY_LEN as u64
}

pub fn page_dir_len(n_pages: u32) -> u64 {
    align8(16 + n_pages as u64 * PAGE_DIR_ENTRY_LEN as u64)
}

/// Peak data of a chunk before compression: the m/z deltas as four byte planes, then the intensities as two byte planes.
pub fn shuffle_peaks(mz_delta: &[u32], it: &[u16]) -> Vec<u8> {
    let p = mz_delta.len();
    let mut out = vec![0u8; 6 * p];
    for (i, d) in mz_delta.iter().enumerate() {
        let b = d.to_le_bytes();
        for k in 0..4 {
            out[k * p + i] = b[k];
        }
    }
    for (i, v) in it.iter().enumerate() {
        let b = v.to_le_bytes();
        out[4 * p + i] = b[0];
        out[5 * p + i] = b[1];
    }
    out
}

/// Inverse of `shuffle_peaks` for `p` peaks (`raw.len() == 6 p` is checked by the caller).
pub fn unshuffle_peaks(raw: &[u8], p: usize) -> (Vec<u32>, Vec<u16>) {
    let mut d = Vec::with_capacity(p);
    let mut it = Vec::with_capacity(p);
    for i in 0..p {
        d.push(u32::from_le_bytes([
            raw[i],
            raw[p + i],
            raw[2 * p + i],
            raw[3 * p + i],
        ]));
        it.push(u16::from_le_bytes([raw[4 * p + i], raw[5 * p + i]]));
    }
    (d, it)
}
