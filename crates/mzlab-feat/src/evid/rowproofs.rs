//! The sign of every kind of proof of every row, for the views that read the proofs without holding the graph (`tp`): one byte per kind and row, made
//! identity by identity so that nothing but the signs is kept.

use super::prepared::Prepared;
use super::*;

/// Signs of the proofs of one row, in the order of `Kind::ALL`.
pub type Signs = [u8; 10];

pub const NONE: u8 = 0;
pub const FOR: u8 = 1;
pub const AGAINST: u8 = 2;
pub const NEUTRAL: u8 = 3;

/// The sign of the proof of `kind` in `signs` (`NONE` when the row has none).
pub fn sign_of(signs: &Signs, kind: Kind) -> u8 {
    signs[Kind::ALL.iter().position(|k| *k == kind).unwrap_or(0)]
}

/// Signs of the proofs of every row of the aligned table (the order of the rows, not the one of the graph).
pub fn row_signs(inp: &Inputs) -> Vec<Signs> {
    let prep = Prepared::new(inp);
    (0..inp.aligned.len())
        .map(|r| {
            let mut s: Signs = [NONE; 10];
            for e in &prep.identity(inp, r).breakdown.rows {
                let i = Kind::ALL.iter().position(|k| *k == e.kind).unwrap_or(0);
                if s[i] != AGAINST {
                    s[i] = match e.sign {
                        Sign::For => FOR,
                        Sign::Against => AGAINST,
                        Sign::Neutral => NEUTRAL,
                    };
                }
            }
            s
        })
        .collect()
}
