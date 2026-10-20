//! The chemical structure as a first-class record `mol/1` (docs/agenti/mol.md): atoms, Kekulé bonds, stereo elements with a status,
//! enhanced-stereo groups, and optional coordinate, id, property and region layers. Used by the readers and writers of this crate and,
//! later, by every engine that needs a molecule; it links nothing beyond serde and sha2 and is not part of any site .wasm yet.

mod canon;
mod codec;
mod cxsmiles;
mod elements;
mod kekule;
mod model;
mod molfile;
mod molfile_stereo;
mod molfile_v2000;
mod molfile_v3000;
mod renumber;
mod sites;
mod smiles;
mod smiles_parse;
mod smiles_stereo;
mod smiles_write;
mod valence;
mod validate;

pub use canon::CAN_VERSION;
pub use codec::{hex, MolError};
pub use sites::Site;
pub use model::{
    Atom, Bond, Cfg, Coords, GroupKind, Ids, Mol, Parity, Prop, Region, RegionKind, Status, Stereo,
    StereoGroup, FORMAT,
};
