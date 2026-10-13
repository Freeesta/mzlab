//! The view of the transformation products (`tp/1`): which rows of the table are formed in the treatment, are not the parent in disguise and are
//! explained, and how sure their kinship with the parent is. It decides nothing about identity (that stays the cascade of `livelli/1`) and deletes no row:
//! every row keeps a status and the numbers behind it.
//!
//! Gates in order (`cancelli`): 1. formed (the treated samples beat the blanks, the dark and the t0 by `ratio`; with the roles, each reference in turn and
//! the one that took the row out noted); 2. not an in-source fragment of the parent
//! (or of an accepted product), told by a constant ratio of areas along the series and the coelution; 3. explained (a rule, a cleavage, an imported
//! hypothesis or an MS2 related to the parent's). The rows that pass 1 and 2 but not 3 are the "formed and not explained" ones. Kinship (`parentela`) is a
//! second axis, read from the signs of the proofs of `evid/1`.

mod cancelli;
mod meta;
mod parentela;
mod rete;
mod tabella;

pub use rete::{arcs, explain_by_arcs, next_generation, Arc, NetParams};
pub use tabella::{role_of, table_json, Role, TABLE_SCHEMA};

use mzlab_core::error::Result;

use crate::trace::filled;
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
    /// Tolerance of the fragments of the parent's MS2 and of the neutral losses, ppm.
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
    /// The rules, to look for their differences of mass among the rows (`rete`); None = no network.
    pub rules: Option<&'a crate::hypo::Rules>,
}

/// The roles of the files (`campioni/2`, `mzlab_core::campioni::per_tp`): gate 1 reads them instead of guessing from the minutes. The infusion is
/// never a reference nor a treated file; standards, QC and files without a time take no part.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct TpRoles {
    pub treated: Vec<usize>,
    pub t0: Vec<usize>,
    pub blank: Vec<usize>,
    pub dark: Vec<usize>,
    pub infusion: Vec<usize>,
    /// Kind of every blank in the order of `blank` (`solvent`, `procedure`, "" when not said).
    pub blank_kinds: Vec<String>,
    /// Minutes of every file: the time of a treated file, 0 for a t0, NaN for every other one.
    pub minutes: Vec<f64>,
}

impl TpRoles {
    pub fn of(cs: &[mzlab_core::campioni::Campione]) -> TpRoles {
        let p = mzlab_core::campioni::per_tp(cs);
        TpRoles {
            blank_kinds: p.blank.iter().map(|&i| cs[i].blank_kind.clone().unwrap_or_default()).collect(),
            minutes: p.minutes.iter().map(|m| m.unwrap_or(f64::NAN)).collect(),
            treated: p.treated,
            t0: p.t0,
            blank: p.blank,
            dark: p.dark,
            infusion: p.infusion,
        }
    }

    /// The roles of the files from a `campioni/2` document (`error.badCampioni` when it is not one).
    pub fn from_json(campioni: &str) -> Result<TpRoles> {
        Ok(TpRoles::of(&mzlab_core::campioni::from_json(campioni.as_bytes())?))
    }

    /// The files the kinetics reads: the timed ones (the t0 and the treated) and the controls. With the roles the controls are the blanks and the
    /// dark only (never the infusion, a standard or a QC); without, every file with no time in `minutes` is a control.
    pub fn kinetic_files(minutes: &[f64], roles: Option<&TpRoles>) -> (Vec<usize>, Vec<usize>) {
        let timed = (0..minutes.len()).filter(|&k| minutes[k].is_finite()).collect();
        let controls = match roles {
            Some(ro) => (0..minutes.len()).filter(|k| ro.blank.contains(k) || ro.dark.contains(k)).collect(),
            None => (0..minutes.len()).filter(|&k| !minutes[k].is_finite()).collect(),
        };
        (timed, controls)
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

/// The view of the transformation products of a table. `Ok(None)` when the parent is not in the table; an error when there are no controls or no treated
/// samples (the gates cannot be read, and a view with no gate would be a lie). The roles come from the minutes: NaN = a control, 0 = the t0.
pub fn build(inp: &TpInputs) -> Result<Option<TpBlock>> {
    run_gates(inp, None)
}

/// `build` with the roles of the files: gate 1 says which reference took a row out, runs without a t0 (every row noted) and leaves out the
/// infusion; `inp.minutes` is replaced by `roles.minutes`.
pub fn build_by_role(inp: &TpInputs, roles: &TpRoles) -> Result<Option<TpBlock>> {
    let inp = TpInputs { minutes: &roles.minutes, ..*inp };
    run_gates(&inp, Some(roles))
}

fn run_gates(inp: &TpInputs, roles: Option<&TpRoles>) -> Result<Option<TpBlock>> {
    let Some(parent) = cancelli::parent_row(inp) else {
        return Ok(None);
    };
    let n = inp.aligned.len();
    let mut b = TpBlock {
        parent_row: parent as u32,
        params: inp.params.clone(),
        ..TpBlock::default()
    };
    b.status = filled(n, ST_TP)?;
    b.notes = filled(n, 0)?;
    b.via = filled(n, 0)?;
    b.kinship = filled(n, KIN_NONE)?;
    match roles {
        Some(roles) => b.roles = Some(cancelli::formed_by_role(inp, roles, parent, &mut b)?),
        None => cancelli::formed(inp, parent, &mut b)?,
    }
    cancelli::not_in_source(inp, parent, &mut b);
    cancelli::explained(inp, &mut b);
    if let Some(rules) = inp.rules {
        explain_by_arcs(inp, rules, &mut b, &NetParams::default())?;
    }
    parentela::kinship(inp, &mut b);
    Ok(Some(b))
}
