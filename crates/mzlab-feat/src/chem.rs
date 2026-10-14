//! Chemistry from a structure. Not used by the engine of the proofs: it lives in a `.wasm` of its own (`crates/mzlab-feat-grafo-wasm`), loaded when
//! the user gives the SMILES of the parent.

pub mod frammenti;
pub mod grafo;
mod gruppi;
mod smiles;
