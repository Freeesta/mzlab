//! mzLab chemistry engine: 3D geometry from SMILES (`docs/agenti/reattivita.md`).

pub mod embed;
pub mod lengths;
pub mod minimize;
pub mod mol3d;

pub use embed::{embed, Conf, DEFAULT_CONFORMERS};
pub use mol3d::Mol3d;
