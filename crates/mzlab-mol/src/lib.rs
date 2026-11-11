//! The chemical structure as a first-class record `mol/1` (docs/agenti/mol.md): atoms, Kekulé bonds, stereo elements with a status,
//! enhanced-stereo groups, and optional coordinate, id, property and region layers. Used by the readers and writers of this crate, the
//! substructure matcher (`SubQuery`) and the engines that need a molecule; it links nothing beyond serde and sha2.

mod aromatic;
mod canon;
mod codec;
mod crippen;
mod crippen_tab;
mod cxsmiles;
pub mod delta;
mod descr;
mod elements;
mod kekule;
mod model;
mod molfile;
mod molfile_stereo;
mod molfile_v2000;
mod molfile_v3000;
mod query;
mod renumber;
mod sites;
mod smiles;
mod smiles_parse;
mod smiles_stereo;
mod smiles_write;
mod substruct;
mod transform;
mod transform_chain;
mod valence;
mod validate;

pub use canon::CAN_VERSION;
pub use codec::{hex, MolError};
pub use descr::Descriptors;
pub use model::{
    Atom, Bond, Cfg, Coords, GroupKind, Ids, Mol, Parity, Prop, Region, RegionKind, Status, Stereo,
    StereoGroup, FORMAT,
};
pub use query::{read_target, AtomTest, BondTest, SubQuery};
pub use sites::Site;
pub use substruct::SubOptions;
pub use transform::{Evidence, TRegion, TSite, Transformation, TRANSFORMATION_FORMAT};
