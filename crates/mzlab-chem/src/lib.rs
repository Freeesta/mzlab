//! mzLab chemistry engine: 3D geometry from SMILES, GFN2-xTB with ALPB water and condensed Fukui functions (`reatt/1`) (`docs/agenti/reattivita.md`).

pub mod bde;
pub mod eht;
pub mod embed;
pub mod fukui;
pub mod lengths;
pub mod minimize;
pub mod mol3d;
pub mod protomer;
pub mod reatt;
pub mod xtb;

pub use embed::{embed, Conf, DEFAULT_CONFORMERS};
pub use mol3d::Mol3d;
pub use reatt::{reactivity_json, ENGINE};
