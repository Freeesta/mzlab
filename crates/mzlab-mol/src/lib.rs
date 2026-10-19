//! The chemical structure as a first-class record `mol/1` (docs/agenti/mol.md): atoms, Kekulé bonds, stereo elements with a status,
//! enhanced-stereo groups, and optional coordinate, id, property and region layers. Used by the readers and writers of this crate and,
//! later, by every engine that needs a molecule; it links nothing beyond serde and sha2 and is not part of any site .wasm yet.

mod codec;
mod elements;
mod model;
mod validate;

pub use codec::{hex, MolError};
pub use model::{
    Atom, Bond, Cfg, Coords, GroupKind, Ids, Mol, Parity, Prop, Region, RegionKind, Status, Stereo,
    StereoGroup, FORMAT,
};
