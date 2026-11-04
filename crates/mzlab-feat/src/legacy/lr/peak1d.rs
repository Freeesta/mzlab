//! `peaks.find_peak` of mzFinder for one trace: it lives in `mzlab_core::peak1d` so the engine of the site (the SIM table of the Data view) integrates
//! with the same code; re-exported here for the MRM, transition and MS2-link callers.

pub use mzlab_core::peak1d::*;
