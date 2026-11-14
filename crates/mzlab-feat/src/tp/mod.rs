//! The view of the transformation products (`tp/1`): which rows of the table are formed in the treatment, are not the parent in disguise and are
//! explained, and how sure their kinship with the parent is. It decides nothing about identity (that stays the cascade of `livelli/1`) and deletes no row:
//! every row keeps a status and the numbers behind it.
//!
//! Gates in order (`cancelli`): 1. formed (the treated samples beat the blanks, the dark and the t0 by `ratio`; with the roles, each reference in turn and
//! the one that took the row out noted); 2. not an in-source fragment of the parent
//! (or of an accepted product), told by a constant ratio of areas along the series and the coelution; 3. explained (a rule, a cleavage, an imported
//! hypothesis or an MS2 related to the parent's). The rows that pass 1 and 2 but not 3 are the "formed and not explained" ones. Kinship (`parentela`) is a
//! second axis, read from the signs of the proofs of `evid/1`.

mod gates;
mod candidates;
mod groups;
mod meta;
mod kinship;
mod network;
mod table;

pub use candidates::{candidates, candidates_json, element_fit};
pub use groups::{conditions as group_conditions, groups_json, infos_from_json, treated_by_condition, RowInfo};
pub use network::{arcs, arcs_in, explain_by_arcs, explain_by_arcs_in, next_generation, Arc, NetParams};
#[cfg(feature = "interp")]
pub(crate) use table::measured;
pub use table::{role_of, table_json, table_json_cond, Role, TABLE_SCHEMA};
/// The old path of the block and its codes, which live in `tp_block` so that the aligned table does not depend on `tp`.
pub use crate::tp_block::{FitCandidate, RoleCounts, TpBlock, TpParams, KIN_CERTAIN, KIN_DOUBTFUL, KIN_NONE, KIN_POSSIBLE, KIN_PROBABLE, NOTE_BLANK, NOTE_CHIMERIC_MS2, NOTE_DARK, NOTE_ISF_COELUTES, NOTE_ISF_OF_PRODUCT, NOTE_NO_T0, NOTE_T0, NOTE_T0_PRESENT, NO_PARENT, ST_CONTROL, ST_ISF, ST_PARENT, ST_TP, ST_UNEXPLAINED, VIA_CLEAVAGE, VIA_IMPORTED, VIA_MS2, VIA_NETWORK, VIA_RULE};

use mzlab_core::error::Result;

use crate::trace::filled;
use crate::aligned::Aligned;
use crate::evid::rowproofs::Signs;
use crate::hypo::Hypothesis;

pub const SCHEMA: &str = "tp/1";

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
    /// m/z of the MS1 ions of the file of the infusion of the parent (sorted), when there is one: an in-source fragment of the parent shows in it too.
    pub infusion: Option<&'a [f64]>,
    pub params: &'a TpParams,
    /// The rules, to look for their differences of mass among the rows (`rete`); None = no network.
    pub rules: Option<&'a crate::hypo::Rules>,
}

/// The roles of the files (`campioni/2`, `mzlab_core::samples::per_tp`): gate 1 reads them instead of guessing from the minutes. The infusion is
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
    /// With two or more conditions (`serie` of the samples), the treated files of each, in the order of the conditions; else empty. The network reads
    /// its correlations inside each condition.
    pub conditions: Vec<Vec<usize>>,
}

impl TpRoles {
    pub fn of(cs: &[mzlab_core::samples::Sample]) -> TpRoles {
        let p = mzlab_core::samples::per_tp(cs);
        TpRoles {
            blank_kinds: p.blank.iter().map(|&i| cs[i].blank_kind.clone().unwrap_or_default()).collect(),
            minutes: p.minutes.iter().map(|m| m.unwrap_or(f64::NAN)).collect(),
            treated: p.treated,
            t0: p.t0,
            blank: p.blank,
            dark: p.dark,
            infusion: p.infusion,
            conditions: groups::treated_by_condition(cs),
        }
    }

    /// The roles of the files from a `campioni/2` document (`error.badCampioni` when it is not one).
    pub fn from_json(samples: &str) -> Result<TpRoles> {
        Ok(TpRoles::of(&mzlab_core::samples::from_json(samples.as_bytes())?))
    }

    /// `from_json` with the files put in the order of the files of the `.mzfeat` `table`, by name (`samples::order_like`).
    pub fn for_table(samples: &str, table: &[u8]) -> Result<TpRoles> {
        let names = crate::store::table_names(&mut crate::store::Slice(table), table.len() as u64)?;
        TpRoles::from_json(&mzlab_core::samples::order_like(samples, &names)?)
    }

    /// `for_table` on a `.mzfeat` read by offsets; None or an empty text is no roles.
    pub fn opt_for_table<R: mzlab_core::idxread::ReadAt>(samples: Option<&str>, table: &mut R, size: u64) -> Result<Option<TpRoles>> {
        let Some(c) = samples.filter(|c| !c.is_empty()) else {
            return Ok(None);
        };
        let names = crate::store::table_names(table, size)?;
        TpRoles::from_json(&mzlab_core::samples::order_like(c, &names)?).map(Some)
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
    let Some(parent) = gates::parent_row(inp) else {
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
        Some(roles) => b.roles = Some(gates::formed_by_role(inp, roles, parent, &mut b)?),
        None => gates::formed(inp, parent, &mut b)?,
    }
    gates::not_in_source(inp, parent, &mut b);
    gates::explained(inp, &mut b);
    if let Some(rules) = inp.rules {
        let by = roles.map_or(&[][..], |r| r.conditions.as_slice());
        explain_by_arcs_in(inp, rules, &mut b, &NetParams::default(), by)?;
    }
    kinship::kinship(inp, &mut b);
    Ok(Some(b))
}
