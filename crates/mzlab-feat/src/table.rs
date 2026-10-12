//! Table of the features of one file: one row per ion trace, one vector per column (same row in every vector).

pub use crate::aligned::*;

/// Bits of `flags`.
pub const FLAG_BACKGROUND: u8 = 1 << 0;
/// Fourier-transform side lobe of a much more intense ion.
pub const FLAG_FT_LOBE: u8 = 1 << 1;
pub const FLAG_RESOLVED: u8 = 1 << 2;
pub const FLAG_ISOTOPE: u8 = 1 << 3;
/// At least `min_points` points, shape and S/N above the limits of `quality::QParams`.
pub const FLAG_GOOD: u8 = 1 << 4;
/// The trace touches the first or the last scan of the file.
pub const FLAG_EDGE: u8 = 1 << 5;
/// Several maxima or MS2 that change along the peak: it may be a mixture (set by `resolve`).
pub const FLAG_MIXTURE: u8 = 1 << 6;
/// The purity of the MS2 was estimated with the default isolation window (the index does not carry it).
pub const FLAG_PURITY_EST: u8 = 1 << 7;

/// Roles of `FileFeatures::role` (set by `family::build_families`; 0 = the row takes no part in any family).
pub const ROLE_NONE: u8 = 0;
/// The ion the others of its family derive from (a family of one row has only this).
pub const ROLE_M: u8 = 1;
pub const ROLE_ISOTOPE: u8 = 2;
pub const ROLE_ADDUCT: u8 = 3;
/// In-source fragment, declared only with the proof of the MS2 of its parent.
pub const ROLE_ISF: u8 = 4;
pub const ROLE_DIMER: u8 = 5;
/// Linked to the family by a step that two readings explain and no proof separates.
pub const ROLE_UNDETERMINED: u8 = 6;

/// Bits of `FileFeatures::alt`: readings that stay open for the row.
pub const ALT_NH3: u8 = 1 << 0;
pub const ALT_FORMATE: u8 = 1 << 1;
/// Another role also fits the same step (adduct and loss), not NH3 or formate.
pub const ALT_ROLE: u8 = 1 << 2;
/// More than one ion could be the one it derives from.
pub const ALT_PARENT: u8 = 1 << 3;

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
    /// Pearson correlation of the raw trace with a Gaussian of the same apex and FWHM (-1..1).
    pub shape: Vec<f32>,
    /// Apex over the noise of the flanks (1.4826 MAD, never below a third of the intensity floor).
    pub snr: Vec<f32>,
    /// Full width at half maximum, in scans (interpolated).
    pub fwhm: Vec<f32>,
    /// Width of the right half over the left half at 10 % of the height; 0 when the trace does not come down to 10 %.
    pub asym: Vec<f32>,
    /// Scans without signal between the edges.
    pub gaps: Vec<u16>,
    /// Bits of `quality::Q_*`: why the peak is not good (0 = good).
    pub qreason: Vec<u8>,
    /// MS2 scans linked to the peak and the highest purity of the precursor among them (-1 = no MS2).
    pub ms2_n: Vec<u16>,
    pub purity: Vec<f32>,
    /// Area of the raw trace (no floor) within `quality::AREA_HALF_WINDOW` minutes of the apex, counts x seconds: the isotope ratios use it.
    pub area_win: Vec<f64>,
    /// Family of the row = position + 1 of its `ROLE_M` row (0 = none), the unit aligned across files; `role`: `ROLE_*`; `alt`: `ALT_*`.
    pub family_id: Vec<u32>,
    pub role: Vec<u8>,
    pub alt: Vec<u8>,
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

    /// Append row `i` of `src` (every column).
    pub fn push_row(&mut self, src: &FileFeatures, i: usize) {
        macro_rules! cp {
            ($($c:ident),*) => { $( self.$c.push(src.$c[i]); )* };
        }
        cp!(
            mz, rt, left, right, apex, scan_lo, scan_hi, area, height, npts, flags, iso_group,
            charge, shape, snr, fwhm, asym, gaps, qreason, ms2_n, purity, area_win
        );
    }

    /// Bytes of the table in a fixed layout (little-endian, column by column), the input of the digest of the output.
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut b = Vec::with_capacity(self.len() * 91 + 16);
        b.extend_from_slice(&(self.len() as u64).to_le_bytes());
        b.extend_from_slice(&self.dt.to_le_bytes());
        macro_rules! put {
            ($($c:ident),*) => { $( for v in &self.$c { b.extend_from_slice(&v.to_le_bytes()); } )* };
        }
        put!(mz, rt, left, right, apex, scan_lo, scan_hi, area, height, npts);
        b.extend_from_slice(&self.flags);
        put!(iso_group);
        b.extend(self.charge.iter().map(|&v| v as u8));
        put!(shape, snr, fwhm, asym, purity, gaps, ms2_n, area_win);
        b.extend_from_slice(&self.qreason);
        put!(family_id);
        b.extend_from_slice(&self.role);
        b.extend_from_slice(&self.alt);
        b
    }
}
