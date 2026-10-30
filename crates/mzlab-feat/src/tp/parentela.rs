//! Kinship with the parent (`parentela`), the second axis of the view next to the level of identity: read from the signs of the proofs and from how the row is
//! explained. A proof against is never offset by the others.
//!
//! - doubtful: formed and explained, but a proof of kinship is against it (a kinetic coupling that does not fit, a retention time on the wrong side, an MS2 that
//!   does not look like the parent's);
//! - certain: explained and its MS2 related to the parent's, for;
//! - probable: explained by a rule, a cleavage or an imported hypothesis, with the kinetic form of a product for;
//! - possible: explained, and nothing more.

use super::*;
use crate::evid::rowproofs::{sign_of, AGAINST, FOR};
use crate::evid::Kind;

pub(super) fn kinship(inp: &TpInputs, b: &mut TpBlock) {
    for r in 0..inp.aligned.len() {
        if b.status[r] != ST_TP {
            continue;
        }
        let s = &inp.signs[r];
        let t0 = b.notes[r] & NOTE_T0_PRESENT != 0;
        // the controls proof is against whenever the controls have a signal: for a product that also forms in the t0 that is not a reason for doubt
        let kinds = if t0 { vec![Kind::Kinetics, Kind::Rt, Kind::Network] } else { vec![Kind::Kinetics, Kind::Rt, Kind::Network, Kind::Controls] };
        let kinetic = sign_of(s, Kind::Kinetics) == FOR && !inp.params.saturated;
        b.kinship[r] = if kinds.iter().any(|&k| sign_of(s, k) == AGAINST) {
            KIN_DOUBTFUL
        } else if b.via[r] & VIA_MS2 != 0 {
            KIN_CERTAIN
        } else if kinetic && b.via[r] & (VIA_RULE | VIA_CLEAVAGE | VIA_IMPORTED | VIA_NETWORK) != 0 {
            KIN_PROBABLE
        } else {
            KIN_POSSIBLE
        };
    }
}
