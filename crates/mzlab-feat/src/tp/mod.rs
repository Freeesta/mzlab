//! The view of the transformation products (`tp/1`): which rows of the table are formed in the treatment, are not the parent in disguise and are
//! explained, and how sure their kinship with the parent is. It decides nothing about identity (that stays the cascade of `livelli/1`) and deletes no row:
//! every row keeps a status and the numbers behind it.
//!
//! Gates in order (`cancelli`): 1. formed (the treated samples beat the blanks, the dark and the t0 by `ratio`); 2. not an in-source fragment of the parent
//! (or of an accepted product), told by a constant ratio of areas along the series and the coelution; 3. explained (a rule, a cleavage, an imported
//! hypothesis or an MS2 related to the parent's). The rows that pass 1 and 2 but not 3 are the "formed and not explained" ones. Kinship (`parentela`) is a
//! second axis, read from the signs of the proofs of `evid/1`.

mod cancelli;
mod parentela;

use mzlab_core::error::Result;
use mzlab_core::json::{Out, Value};

use crate::aligned::Aligned;
use crate::evid::rowproofs::Signs;
use crate::hypo::Hypothesis;

pub const SCHEMA: &str = "tp/1";
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

/// How a row is explained (`TpBlock::via`, bits).
pub const VIA_RULE: u8 = 1;
pub const VIA_CLEAVAGE: u8 = 2;
pub const VIA_IMPORTED: u8 = 4;
pub const VIA_MS2: u8 = 8;
/// Reserved for the arcs of the network of mass differences.
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
    /// Tolerance of the fragments of the parent's MS2 and of the neutral losses, ppm.
    pub ppm: f64,
    /// An MS2 with a purity of isolation below this does not count as a proof.
    pub purity_min: f64,
    /// The parent is saturated at the detector: the kinetic coupling with its decay is not used.
    pub saturated: bool,
}

impl Default for TpParams {
    fn default() -> Self {
        TpParams { ratio: 3.0, isf_cv: 0.2, isf_rt: 0.1, isf_points: 3, ppm: 10.0, purity_min: 0.5, saturated: false }
    }
}

/// What the view reads.
pub struct TpInputs<'a> {
    pub aligned: &'a Aligned,
    /// Treatment time of every file in minutes; NaN = a control (blank, dark). A file at 0 is the t0.
    pub minutes: &'a [f64],
    pub hyps: &'a [Hypothesis],
    /// Signs of the proofs of every row (`evid::rowproofs::row_signs`).
    pub signs: &'a [Signs],
    /// m/z and RT of the parent in the table.
    pub parent: (f64, f64),
    /// Fragments of the MS2 of the parent, when it has one (the m/z of an in-source fragment must be among them).
    pub fragments: Option<&'a [f64]>,
    pub params: &'a TpParams,
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

    /// The object `tp` of the meta of the `.mzfeat`: schema, thresholds used, id of the parent and the rows of every status.
    pub fn meta_json(&self) -> String {
        let p = &self.params;
        let mut j = Out::default();
        j.raw("{\"schema\":").str(SCHEMA).raw(",\"progenitore\":").int(i64::from(self.parent_row) + 1);
        j.raw(",\"parametri\":{\"rapporto\":").num(p.ratio).raw(",\"isf_cv\":").num(p.isf_cv).raw(",\"isf_rt\":").num(p.isf_rt);
        j.raw(",\"isf_punti\":").int(p.isf_points as i64).raw(",\"ppm\":").num(p.ppm).raw(",\"purezza_min\":").num(p.purity_min);
        j.raw(",\"saturo\":").raw(if p.saturated { "true" } else { "false" }).raw("},\"conteggi\":{\"tp\":");
        j.int(self.count(ST_TP) as i64).raw(",\"controllo\":").int(self.count(ST_CONTROL) as i64);
        j.raw(",\"isf\":").int(self.count(ST_ISF) as i64).raw(",\"non_spiegate\":").int(self.count(ST_UNEXPLAINED) as i64).raw("}}");
        j.0
    }

    /// The thresholds and the parent from the object `tp` of a meta (the columns are read by the caller).
    pub fn from_meta(v: &Value) -> TpBlock {
        let num = |o: &Value, k: &str, d: f64| o.get(k).and_then(Value::f64).unwrap_or(d);
        let d = TpParams::default();
        let o = v.get("parametri");
        let get = |k: &str, dv: f64| o.map_or(dv, |o| num(o, k, dv));
        TpBlock {
            parent_row: (num(v, "progenitore", 0.0) as u32).wrapping_sub(1),
            params: TpParams {
                ratio: get("rapporto", d.ratio),
                isf_cv: get("isf_cv", d.isf_cv),
                isf_rt: get("isf_rt", d.isf_rt),
                isf_points: get("isf_punti", d.isf_points as f64) as usize,
                ppm: get("ppm", d.ppm),
                purity_min: get("purezza_min", d.purity_min),
                saturated: o.and_then(|o| o.get("saturo")).and_then(Value::bool).unwrap_or(false),
            },
            ..TpBlock::default()
        }
    }
}

/// The view of the transformation products of a table. `Ok(None)` when the parent is not in the table; an error when there are no controls or no treated
/// samples (the gates cannot be read, and a view with no gate would be a lie).
pub fn build(inp: &TpInputs) -> Result<Option<TpBlock>> {
    let Some(parent) = cancelli::parent_row(inp) else { return Ok(None) };
    let n = inp.aligned.len();
    let mut b = TpBlock { parent_row: parent as u32, params: inp.params.clone(), ..TpBlock::default() };
    b.status = vec![ST_TP; n];
    b.notes = vec![0; n];
    b.via = vec![0; n];
    b.kinship = vec![KIN_NONE; n];
    cancelli::formed(inp, parent, &mut b)?;
    cancelli::not_in_source(inp, parent, &mut b);
    cancelli::explained(inp, &mut b);
    parentela::kinship(inp, &mut b);
    Ok(Some(b))
}
