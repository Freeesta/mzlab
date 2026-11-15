//! The part of the engine of mzFinder that the interpretation of the rows and the table views read without the Dig: the fragmentation and MSn
//! trees, the MS2 consensus, the spectral similarity, the identity graph and the files and registry of the series. `legacy` re-exports it.

pub mod edge;
#[cfg(feature = "chem")]
pub mod fragtree;
pub mod identity;
pub mod inclusion;
pub mod ms2;
#[cfg(feature = "chem")]
pub mod msn;
pub mod sample;
pub mod series;
pub mod similarity;

/// Errors of the adapter: the English text the page shows (`str(e)` of the Python).
pub type Res<T> = std::result::Result<T, String>;
