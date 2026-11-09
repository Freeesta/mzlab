//! Chemistry from a structure. Not used by the engine of the proofs: it lives in a `.wasm` of its own (`crates/mzlab-feat-grafo-wasm`), loaded when
//! the user gives the SMILES of the parent.

pub mod breaks;
pub mod fragmenter;
pub mod fragments;
#[cfg(feature = "record")]
mod grafo_record;
pub mod graph;
mod groups;
pub mod labels;
pub mod positions;
pub mod rules;
mod rules_site;
mod shuffle;
mod smiles;
pub mod structures;
