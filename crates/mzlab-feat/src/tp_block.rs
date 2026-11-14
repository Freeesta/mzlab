//! The block `tp/1` of the aligned table (`TpBlock`) with its codes, thresholds and formula candidates: plain data with no dependency, so the
//! table can carry it without reaching the gates of `tp`. The old paths `mzlab_feat::tp::…` stay as re-exports.

/// `NO_PARENT` in `TpBlock::parent_row`.
pub const NO_PARENT: u32 = u32::MAX;

/// Status of a row (`TpBlock::status`).
pub const ST_TP: u8 = 0;
pub const ST_CONTROL: u8 = 1;
pub const ST_ISF: u8 = 2;
pub const ST_UNEXPLAINED: u8 = 3;
pub const ST_PARENT: u8 = 4;

/// Notes of a row (`TpBlock::notes`, bits).
pub const NOTE_T0_PRESENT: u8 = 1;
pub const NOTE_ISF_COELUTES: u8 = 2;
pub const NOTE_ISF_OF_PRODUCT: u8 = 4;
pub const NOTE_CHIMERIC_MS2: u8 = 8;
/// Gate 1 by role (`build_by_role`): the reference that took the row out (the blank, the t0, the dark: "it forms in the dark too").
pub const NOTE_BLANK: u8 = 16;
pub const NOTE_T0: u8 = 32;
pub const NOTE_DARK: u8 = 64;
/// The series has no t0: every row carries it (more false positives).
pub const NOTE_NO_T0: u8 = 128;

/// How a row is explained (`TpBlock::via`, bits).
pub const VIA_RULE: u8 = 1;
pub const VIA_CLEAVAGE: u8 = 2;
pub const VIA_IMPORTED: u8 = 4;
pub const VIA_MS2: u8 = 8;
/// An arc of the network of mass differences (`rete`) explains the row.
pub const VIA_NETWORK: u8 = 16;

/// Kinship with the parent (`TpBlock::kinship`).
pub const KIN_NONE: u8 = 0;
pub const KIN_DOUBTFUL: u8 = 1;
pub const KIN_POSSIBLE: u8 = 2;
pub const KIN_PROBABLE: u8 = 3;
pub const KIN_CERTAIN: u8 = 4;

/// Thresholds of the gates; the values recorded in the block are the ones that were used.
#[derive(Debug, Clone, PartialEq)]
pub struct TpParams {
    /// The treated samples must beat the controls by this factor (NORMAN 3:1; 10:1 is the strict choice).
    pub ratio: f64,
    /// Largest coefficient of variation of the ratio of areas child/parent for an in-source fragment.
    pub isf_cv: f64,
    /// Largest distance in retention time of an in-source fragment from its parent, minutes.
    pub isf_rt: f64,
    /// Fewest times with both ions measured to judge a ratio.
    pub isf_points: usize,
    /// Tolerance of the fragments of the parent's MS2 and of the neutral losses: ppm, or Da when negative (the unit axis, `axis::half`).
    pub ppm: f64,
    /// An MS2 with a purity of isolation below this does not count as a proof.
    pub purity_min: f64,
    /// The parent is saturated at the detector: the kinetic coupling with its decay is not used.
    pub saturated: bool,
}

impl Default for TpParams {
    fn default() -> Self {
        TpParams {
            ratio: 3.0,
            isf_cv: 0.2,
            isf_rt: 0.1,
            isf_points: 3,
            ppm: 10.0,
            purity_min: 0.5,
            saturated: false,
        }
    }
}

/// What gate 1 by role adds to the meta: the rows taken out by each reference, whether the t0 is missing, the parent in the dark over the t0
/// (the share left after adsorption; shown, never a threshold) and the kinds of the blanks.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct RoleCounts {
    pub by_blank: usize,
    pub by_t0: usize,
    pub by_dark: usize,
    pub no_t0: bool,
    pub dark_over_t0: Option<f64>,
    pub blank_kinds: Vec<String>,
}

/// The block `tp/1`: one byte per row and column.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct TpBlock {
    pub status: Vec<u8>,
    pub notes: Vec<u8>,
    pub via: Vec<u8>,
    pub kinship: Vec<u8>,
    pub parent_row: u32,
    pub params: TpParams,
    /// Present when gate 1 read the roles (`build_by_role`).
    pub roles: Option<RoleCounts>,
    /// Formula candidates of the rows formed and not explained, sorted by row (`candidati`); a lead, never a verdict.
    pub fit_candidates: Vec<(u32, Vec<FitCandidate>)>,
}

impl TpBlock {
    pub fn len(&self) -> usize {
        self.status.len()
    }

    pub fn is_empty(&self) -> bool {
        self.status.is_empty()
    }

    pub fn heap_bytes(&self) -> usize {
        self.status.capacity() * 4
    }

    /// Rows with `status`.
    pub fn count(&self, status: u8) -> usize {
        self.status.iter().filter(|&&s| s == status).count()
    }
}

/// A formula candidate of a row.
#[derive(Debug, Clone, PartialEq)]
pub struct FitCandidate {
    pub neutral_formula: String,
    pub ion_formula: String,
    pub ppm: f64,
    pub element_fit: f64,
    pub ms2_explained: Option<f64>,
    pub isotope_fit: Option<f64>,
    pub total: f64,
}
