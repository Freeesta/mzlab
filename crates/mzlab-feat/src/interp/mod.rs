//! The interpretation of a row of the feature table (`interp/1`): for each row that passes the filters of `tp/1`, the modification it carries, every
//! DDA MS2 of the series linked to it (with the purity of the isolation), the comparison with the MS2 and the MSn tree of the parent, and the registry of
//! the filters (`filtri/1`). It is written in the `.mzfeat` as the block `interp` (`store`) and read back by row. It also says where the
//! modification sits (`site`, `site.rs`) and how to draw it (`drawing`, `drawing.rs`), never as an identity. Behind the feature `interp`.

mod apex;
mod build;
pub mod comparison;
pub mod drawing;
pub mod edit;
pub mod filters;
pub mod json;
pub mod ms2link;
pub mod redraw;
pub mod refilter;
pub mod reposition;
pub mod review;
pub mod run;
pub mod site;
pub mod site_json;
pub mod store;
pub mod tree;

pub use build::{build, Inputs, Output, RowInterp};

pub const SCHEMA: &str = "interp/1";
/// Version of the block in the meta of the `.mzfeat`.
pub const VERSION: i64 = 1;
