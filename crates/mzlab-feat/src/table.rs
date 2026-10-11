//! Table of the features of one file: one row per ion trace, one vector per column (same row in every vector).

/// Bits of `flags`.
pub const FLAG_BACKGROUND: u8 = 1 << 0;
/// Fourier-transform side lobe of a much more intense ion.
pub const FLAG_FT_LOBE: u8 = 1 << 1;
pub const FLAG_RESOLVED: u8 = 1 << 2;
pub const FLAG_ISOTOPE: u8 = 1 << 3;
pub const FLAG_GOOD: u8 = 1 << 4;
// bits 5-7 reserved

/// Features of one file and polarity, sorted by m/z. `rt`, `left`, `right` in minutes; `area` in counts x seconds.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct FileFeatures {
    pub mz: Vec<f64>,
    pub rt: Vec<f32>,
    pub left: Vec<f32>,
    pub right: Vec<f32>,
    /// Position of the apex scan in the MS1 section of the index.
    pub apex: Vec<u32>,
    pub scan_lo: Vec<u32>,
    pub scan_hi: Vec<u32>,
    pub area: Vec<f64>,
    pub height: Vec<f32>,
    pub npts: Vec<u16>,
    pub flags: Vec<u8>,
    pub iso_group: Vec<u32>,
    pub charge: Vec<i8>,
    /// Median scan interval, seconds.
    pub dt: f64,
}

impl FileFeatures {
    pub fn len(&self) -> usize {
        self.mz.len()
    }

    pub fn is_empty(&self) -> bool {
        self.mz.is_empty()
    }

    /// Bytes of the table in a fixed layout (little-endian, column by column), the input of the digest of the output.
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut b = Vec::with_capacity(self.len() * 46 + 16);
        b.extend_from_slice(&(self.len() as u64).to_le_bytes());
        b.extend_from_slice(&self.dt.to_le_bytes());
        for v in &self.mz {
            b.extend_from_slice(&v.to_le_bytes());
        }
        for c in [&self.rt, &self.left, &self.right] {
            for v in c {
                b.extend_from_slice(&v.to_le_bytes());
            }
        }
        for c in [&self.apex, &self.scan_lo, &self.scan_hi] {
            for v in c {
                b.extend_from_slice(&v.to_le_bytes());
            }
        }
        for v in &self.area {
            b.extend_from_slice(&v.to_le_bytes());
        }
        for v in &self.height {
            b.extend_from_slice(&v.to_le_bytes());
        }
        for v in &self.npts {
            b.extend_from_slice(&v.to_le_bytes());
        }
        b.extend_from_slice(&self.flags);
        for v in &self.iso_group {
            b.extend_from_slice(&v.to_le_bytes());
        }
        b.extend(self.charge.iter().map(|&v| v as u8));
        b
    }
}
