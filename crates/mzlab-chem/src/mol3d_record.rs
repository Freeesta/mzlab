//! `mol/1` record to `Mol3d` (feature `record`, in no .wasm); used by the parity test `tests/mol3d_record.rs`.

use crate::mol3d::Mol3d;
use mzlab_core::error::Result;
use mzlab_feat::chem::grafo::Mol;

/// The atoms of a `mol/1` record plus their hydrogens, no coordinates yet: `Mol3d::from_mol` of `Mol::from_record(rec, true)` (aromaticity
/// perceived from the Kekulé form, bonds sorted by atom pair).
pub fn mol3d_from_record(rec: &mzlab_mol::Mol) -> Result<Mol3d> {
    Mol3d::from_mol(&Mol::from_record(rec, true)?)
}
