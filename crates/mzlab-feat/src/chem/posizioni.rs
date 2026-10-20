//! Position of a substitution (`posiz/1`): given a parent (SMILES), a transformation and the observed MS2 of the product, list the atoms where the change
//! can sit, group them by symmetry, compute the fragments each one predicts and say which groups of positions the spectrum supports, excludes or cannot
//! tell apart. It never picks a position and never assigns a level: `proposal.sublevel` is a suggestion (3a or 3b) for the chemist.
//! With the `reatt/1` of the parent the report also carries `prior`, the f(−) of each site as a «Prev.» that never changes the classes (`prior.rs`).
//! Graph limits are those of `grafo::Mol` (rings are not opened, no stereochemistry). Contract and rules: `docs/agenti/posizioni.md`.

mod confronto;
mod frammenti;
mod prior;
mod simmetria;
mod siti;
mod uscita;

pub use confronto::{match_spectrum, Class, Peak, Pol, Report, Tol};
pub use frammenti::{expected_fragments, Frag};
pub use prior::{prior_of, Prior, HALOGEN_CAVEAT, TIE};
pub use simmetria::symmetry_classes;
pub use siti::{check_formulas, is_multi, sites, Site};
#[cfg(feature = "record")]
pub use uscita::positions_record_json;
pub use uscita::{positions_json, positions_prior_json};
