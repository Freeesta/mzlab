//! Per-row summaries of an aligned table: the edges of the peak, the charge, the quality and the best MS2 of the reference ion, taken over the files that
//! detected it (`rowmeta::compute`). Either empty (not computed: tables made by hand, files of format 1) or one value per row.

/// Marks "no MS2" in `RowMeta::ms2_file`.
pub const NO_FILE: u32 = u32::MAX;

/// The summary of one row.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RowQ {
    /// Median left and right edge of the peak over the files that detected it, minutes.
    pub rt_lo: f32,
    pub rt_hi: f32,
    /// Charge of the reference ion (0 = unknown).
    pub charge: i8,
    /// Medians of the shape, the signal over the noise and the FWHM (scans) over the files that detected the row; `qreason` is the OR of their `Q_*` bits.
    pub q_shape: f32,
    pub q_snr: f32,
    pub q_fwhm: f32,
    pub qreason: u8,
    /// Observed M+1/M of the reference ion (NaN when it has no isotopologue in a family).
    pub iso_m1: f32,
    /// File with the MS2 of the highest purity linked to the peak (`NO_FILE` = none) and that purity (-1 = none).
    pub ms2_file: u32,
    pub ms2_purity: f32,
}

impl RowQ {
    pub const NONE: RowQ = RowQ {
        rt_lo: f32::NAN,
        rt_hi: f32::NAN,
        charge: 0,
        q_shape: f32::NAN,
        q_snr: f32::NAN,
        q_fwhm: f32::NAN,
        qreason: 0,
        iso_m1: f32::NAN,
        ms2_file: NO_FILE,
        ms2_purity: -1.0,
    };
}

/// The summaries of all the rows, column by column.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct RowMeta {
    pub rt_lo: Vec<f32>,
    pub rt_hi: Vec<f32>,
    pub charge: Vec<i8>,
    pub q_shape: Vec<f32>,
    pub q_snr: Vec<f32>,
    pub q_fwhm: Vec<f32>,
    pub qreason: Vec<u8>,
    pub iso_m1: Vec<f32>,
    pub ms2_file: Vec<u32>,
    pub ms2_purity: Vec<f32>,
}

impl RowMeta {
    pub fn len(&self) -> usize {
        self.rt_lo.len()
    }

    pub fn is_empty(&self) -> bool {
        self.rt_lo.is_empty()
    }

    pub fn push(&mut self, q: &RowQ) {
        self.rt_lo.push(q.rt_lo);
        self.rt_hi.push(q.rt_hi);
        self.charge.push(q.charge);
        self.q_shape.push(q.q_shape);
        self.q_snr.push(q.q_snr);
        self.q_fwhm.push(q.q_fwhm);
        self.qreason.push(q.qreason);
        self.iso_m1.push(q.iso_m1);
        self.ms2_file.push(q.ms2_file);
        self.ms2_purity.push(q.ms2_purity);
    }

    pub fn get(&self, r: usize) -> RowQ {
        RowQ {
            rt_lo: self.rt_lo[r],
            rt_hi: self.rt_hi[r],
            charge: self.charge[r],
            q_shape: self.q_shape[r],
            q_snr: self.q_snr[r],
            q_fwhm: self.q_fwhm[r],
            qreason: self.qreason[r],
            iso_m1: self.iso_m1[r],
            ms2_file: self.ms2_file[r],
            ms2_purity: self.ms2_purity[r],
        }
    }

    /// `rows` summaries: these when they are all there, else `RowQ::NONE` for each (what a file of format 1 or a table made by hand holds).
    pub fn or_none(&self, rows: usize) -> RowMeta {
        if self.len() == rows {
            return self.clone();
        }
        let mut m = RowMeta::default();
        for _ in 0..rows {
            m.push(&RowQ::NONE);
        }
        m
    }

    pub fn heap_bytes(&self) -> usize {
        (self.rt_lo.capacity() * 4) * 6 + self.charge.capacity() + self.qreason.capacity() + self.ms2_file.capacity() * 4 + self.iso_m1.capacity() * 4
    }
}
