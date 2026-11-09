//! mzLab chemistry engine: 3D geometry from SMILES, GFN2-xTB with ALPB water and condensed Fukui functions (`reatt/1`) (`docs/agenti/reattivita.md`).

pub mod eht;
pub mod embed;
pub mod flow;
pub mod fukui;
pub mod lengths;
pub mod minimize;
pub mod mol3d;
#[cfg(feature = "record")]
pub mod mol3d_record;
pub mod protomer;
pub mod react;
pub mod step;
pub mod xtb;

pub use embed::{embed, Conf, DEFAULT_CONFORMERS};
pub use mol3d::Mol3d;
pub use react::{reactivity_json, ENGINE};
