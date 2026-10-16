//! mzLab chemistry engine: 3D geometry from SMILES (`docs/agenti/reattivita.md`).

pub mod embed;
pub mod minimize;
pub mod mol3d;

pub use embed::{embed, Conf};
pub use mol3d::Mol3d;
