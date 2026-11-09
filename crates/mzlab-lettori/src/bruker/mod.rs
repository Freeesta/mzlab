//! Bruker timsTOF `.d` folders (`analysis.tdf` + `analysis.tdf_bin`, `analysis.tsf` + `analysis.tsf_bin`): docs/agenti/bruker.md.
pub mod block;
pub mod calib;
pub mod header;
pub mod spectra;
pub mod sqlite_min;
pub mod sqlite_schema;
pub mod sqlite_wal;
pub mod tables;
pub mod tdf;
mod tdf_helpers;
pub mod tsf;
