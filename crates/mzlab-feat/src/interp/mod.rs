//! The interpretation of a row of the feature table (`interp/1`): for each row that passes the filters of `tp/1`, the modification it carries, every
//! DDA MS2 of the series linked to it (with the purity of the isolation), the comparison with the MS2 and the MSn tree of the parent, and the registry of
//! the filters (`filtri/1`). It is written in the `.mzfeat` as the block `interp` (`store`) and read back by row. It never gives an identity: the
//! position of the modification (`site`) belongs to a later step. Behind the feature `interp`.

pub mod albero;
mod build;
pub mod confronto;
pub mod filtri;
pub mod json;
pub mod modifica;
pub mod ms2link;
pub mod run;
pub mod store;

pub use build::{build, Inputs, Output, RowInterp};

pub const SCHEMA: &str = "interp/1";
/// Version of the block in the meta of the `.mzfeat`.
pub const VERSION: i64 = 1;
